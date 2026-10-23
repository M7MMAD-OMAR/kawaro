//! The named pipe transport (S-040b, ADR 0005 section 2). Every decision is
//! a pure function in `transport.rs` or `transport/deadline.rs`, tested on
//! Linux; this file only calls the system with them, so it is left out of
//! the mutants run (`.cargo/mutants.toml`) and checked live in S-052.
//!
//! - The pipe is `\\.\pipe\io.github.m7mmad_omar.SbarHive.<SID>.<session>`.
//! - Every instance gets a DACL that grants the current user alone
//!   (`dacl_sddl`) and rejects remote clients (`pipe_mode`).
//! - The first instance claims the name (`open_mode(true)`): a second
//!   service gets `ERROR_ACCESS_DENIED`, read by `bind_result`.
//! - The client checks that the server process runs as the same user
//!   (`verify_owner`) before it writes a byte.
//! - A read timeout is a watcher that calls `CancelIoEx` on the handle.

use super::deadline::read_with_deadline;
use super::{
    BindOutcome, Bound, Cleanup, ConnectStep, NextInstance, accept_plan, bind_result, connect_plan,
    dacl_sddl, open_mode, pipe_mode, pipe_name, verify_owner,
};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    self as foundation, GetLastError, HANDLE, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION,
};
use windows_sys::Win32::System::IO::CancelIoEx;
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT, WaitNamedPipeW,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION,
};

// The pure functions carry literal values; each must match windows_sys, or
// the Windows build fails here.
const _: () = assert!(super::OPEN_MODE_FIRST == PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE);
const _: () = assert!(super::OPEN_MODE_LATER == PIPE_ACCESS_DUPLEX);
const _: () = assert!(
    super::PIPE_MODE
        == PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS
);
const _: () = assert!(super::ERROR_ACCESS_DENIED == foundation::ERROR_ACCESS_DENIED);
const _: () = assert!(super::ERROR_FILE_NOT_FOUND == foundation::ERROR_FILE_NOT_FOUND);
const _: () = assert!(super::ERROR_PIPE_BUSY == foundation::ERROR_PIPE_BUSY);
const _: () = assert!(super::ERROR_PIPE_CONNECTED == foundation::ERROR_PIPE_CONNECTED);

/// The pipe buffer size each way; requests and replies are short lines.
const BUFFER: u32 = 64 * 1024;

/// Where the service listens: a pipe name.
#[derive(Clone, Debug)]
pub struct Endpoint(String);

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The endpoint of this user's service in this session. If the token
/// cannot be read the SID part is empty; `bind` and `connect` then fail on
/// the same token read.
pub fn endpoint() -> Endpoint {
    let sid = current_sid().unwrap_or_default();
    let mut session = 0;
    // SAFETY: `session` is a valid out pointer for the call.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
    Endpoint(pipe_name(&sid, session))
}

/// A NUL terminated UTF-16 copy of `s` for the wide API.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// The security descriptor built from `dacl_sddl`, freed on drop.
struct Descriptor(PSECURITY_DESCRIPTOR);

// SAFETY: the descriptor is plain memory owned by this value alone and only
// read by `CreateNamedPipeW`.
unsafe impl Send for Descriptor {}

impl Descriptor {
    fn for_user(sid: &str) -> io::Result<Descriptor> {
        let sddl = wide(&dacl_sddl(sid));
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: `sddl` is NUL terminated and `sd` is a valid out pointer.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Descriptor(sd))
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe { LocalFree(self.0) };
    }
}

/// The only `CreateNamedPipeW` call: every instance gets `sd`.
fn create_instance(name: &[u16], sd: &Descriptor, first: bool) -> Result<OwnedHandle, u32> {
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd.0,
        bInheritHandle: 0,
    };
    // SAFETY: `name` is NUL terminated and `attributes` outlives the call.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            open_mode(first),
            pipe_mode(),
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        // SAFETY: no other call since CreateNamedPipeW.
        return Err(unsafe { GetLastError() });
    }
    // SAFETY: a valid pipe handle that nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
}

pub struct Listener {
    name: Vec<u16>,
    sd: Descriptor,
    /// The instance the next `accept` waits on. Normally there is one, so a
    /// client finds the pipe busy, never missing, between two accepts. It
    /// is empty when a client took the last instance and no fresh one could
    /// be made: a new client then gets `ERROR_FILE_NOT_FOUND` until the next
    /// accept creates one, within 50 ms (S-028b AC5).
    next: Mutex<Option<OwnedHandle>>,
}

impl Listener {
    /// Claims the pipe name with the first instance, unless a service
    /// already owns it.
    pub fn bind(e: &Endpoint) -> io::Result<Bound> {
        let sd = Descriptor::for_user(&current_sid()?)?;
        let name = wide(&e.0);
        let first = create_instance(&name, &sd, true);
        match (bind_result(first.as_ref().err().copied()), first) {
            (BindOutcome::Listen, Ok(handle)) => Ok(Bound::Listening(Listener {
                name,
                sd,
                next: Mutex::new(Some(handle)),
            })),
            (BindOutcome::AlreadyRunning, _) => Ok(Bound::AlreadyRunning),
            (BindOutcome::Failed(err), _) => Err(err),
            (BindOutcome::Listen, Err(code)) => Err(io::Error::from_raw_os_error(code as i32)),
        }
    }

    /// Waits for a client on the current instance, creating one first when
    /// the slot is empty, then puts a fresh instance in its place as
    /// `accept_plan` decides before handing the connection over.
    pub fn accept(&self) -> io::Result<Stream> {
        let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
        let current = match next.take() {
            Some(handle) => handle,
            None => create_instance(&self.name, &self.sd, false)
                .map_err(|code| io::Error::from_raw_os_error(code as i32))?,
        };
        // SAFETY: a valid pipe handle; synchronous, so no OVERLAPPED.
        let connected = unsafe { ConnectNamedPipe(current.as_raw_handle(), null_mut()) } != 0;
        let failure = io::Error::last_os_error();
        let connect_error = (!connected).then(|| failure.raw_os_error().unwrap_or(0) as u32);
        let fresh = create_instance(&self.name, &self.sd, false).ok();
        let plan = accept_plan(connected, connect_error, fresh.is_some());
        let current = match plan.next {
            NextInstance::Fresh => {
                *next = fresh;
                current
            }
            NextInstance::Keep => {
                // SAFETY: a valid pipe handle; makes it ready for a new client.
                unsafe { DisconnectNamedPipe(current.as_raw_handle()) };
                *next = Some(current);
                return Err(failure);
            }
            NextInstance::Empty => current,
        };
        if plan.hand_over {
            Ok(Stream::new(File::from(current), None))
        } else {
            Err(failure)
        }
    }

    /// The pipe goes with its last handle, so there is nothing to remove.
    pub fn cleanup(&self) -> Cleanup {
        Cleanup(None)
    }
}

pub struct Stream {
    file: File,
    /// Shared with `try_clone` copies, as a socket's timeout is on Unix.
    timeout: Arc<Mutex<Option<Duration>>>,
    server_pid: Option<u32>,
}

impl Stream {
    fn new(file: File, server_pid: Option<u32>) -> Stream {
        Stream {
            file,
            timeout: Arc::new(Mutex::new(None)),
            server_pid,
        }
    }

    /// Opens the pipe, waiting up to 2000 ms while it is busy, and checks
    /// that the server runs as this user before anything is written.
    pub fn connect(e: &Endpoint) -> io::Result<Stream> {
        let name = wide(&e.0);
        let start = Instant::now();
        let file = loop {
            let err = match OpenOptions::new()
                .read(true)
                .write(true)
                .security_qos_flags(SECURITY_IDENTIFICATION)
                .open(&e.0)
            {
                Ok(file) => break file,
                Err(err) => err,
            };
            let Some(code) = err.raw_os_error() else {
                return Err(err);
            };
            match connect_plan(Some(code as u32), start.elapsed()) {
                ConnectStep::Wait(rest) => {
                    // 0 would mean the pipe's default wait.
                    let ms = u32::try_from(rest.as_millis()).unwrap_or(u32::MAX).max(1);
                    // SAFETY: `name` is NUL terminated.
                    unsafe { WaitNamedPipeW(name.as_ptr(), ms) };
                }
                ConnectStep::Fail(err) => return Err(err),
                ConnectStep::Connected => {}
            }
        };
        let mut pid = 0;
        // SAFETY: a valid pipe handle and a valid out pointer.
        let known = unsafe { GetNamedPipeServerProcessId(file.as_raw_handle(), &mut pid) } != 0;
        // Fails closed: an unknown owner is an empty SID, never this user's.
        let server_sid = if known {
            process_sid(pid).unwrap_or_default()
        } else {
            String::new()
        };
        let file = verify_owner(&server_sid, &current_sid()?, file)?;
        Ok(Stream::new(file, Some(pid)))
    }

    /// A read past `timeout` fails with `ErrorKind::TimedOut`.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        if timeout == Some(Duration::ZERO) {
            return Err(io::Error::new(
                ErrorKind::InvalidInput,
                "cannot set a 0 duration timeout",
            ));
        }
        *self.timeout.lock().unwrap_or_else(PoisonError::into_inner) = timeout;
        Ok(())
    }

    pub fn try_clone(&self) -> io::Result<Stream> {
        Ok(Stream {
            file: self.file.try_clone()?,
            timeout: self.timeout.clone(),
            server_pid: self.server_pid,
        })
    }

    /// The service's process id, on a client stream.
    pub fn server_pid(&self) -> Option<u32> {
        self.server_pid
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let timeout = *self.timeout.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(timeout) = timeout else {
            return self.file.read(buf);
        };
        // The handle outlives the watcher: read_with_deadline joins it.
        let handle = self.file.as_raw_handle() as usize;
        let file = &mut self.file;
        read_with_deadline(
            timeout,
            || file.read(buf),
            move || {
                // SAFETY: the handle is open until read_with_deadline returns.
                unsafe { CancelIoEx(handle as HANDLE, null()) };
            },
        )
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// The SID of this process's user, as text.
fn current_sid() -> io::Result<String> {
    // SAFETY: the pseudo handle of this process needs no closing.
    token_sid(unsafe { GetCurrentProcess() })
}

/// The SID of the user that runs process `pid`, as text.
fn process_sid(pid: u32) -> io::Result<String> {
    // SAFETY: plain call; the result is checked before use.
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a valid process handle that nothing else owns.
    let process = unsafe { OwnedHandle::from_raw_handle(process) };
    token_sid(process.as_raw_handle())
}

/// Reads `TokenUser` from the process token and converts the SID to text.
fn token_sid(process: HANDLE) -> io::Result<String> {
    let mut token: HANDLE = null_mut();
    // SAFETY: `process` is a valid handle and `token` a valid out pointer.
    if unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a valid token handle that nothing else owns.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    let mut len = 0u32;
    // SAFETY: a size query with no buffer; it fails and sets `len`.
    unsafe { GetTokenInformation(token.as_raw_handle(), TokenUser, null_mut(), 0, &mut len) };
    // u64 cells keep TOKEN_USER aligned.
    let mut buf = vec![0u64; (len as usize).div_ceil(8).max(1)];
    // SAFETY: `buf` holds at least `len` bytes.
    let ok = unsafe {
        GetTokenInformation(
            token.as_raw_handle(),
            TokenUser,
            buf.as_mut_ptr().cast(),
            len,
            &mut len,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the call filled `buf` with a TOKEN_USER, and `buf` is aligned.
    let user = unsafe { &*buf.as_ptr().cast::<TOKEN_USER>() };
    let mut text: *mut u16 = null_mut();
    // SAFETY: the SID points into `buf`, alive here; `text` is an out pointer.
    if unsafe { ConvertSidToStringSidW(user.User.Sid, &mut text) } == 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `text` is a NUL terminated string from the call above.
    let sid = unsafe {
        let mut n = 0;
        while *text.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text, n))
    };
    // SAFETY: allocated by ConvertSidToStringSidW.
    unsafe { LocalFree(text.cast()) };
    Ok(sid)
}
