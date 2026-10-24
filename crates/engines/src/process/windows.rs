//! The Windows side of [`ProcessTree`] (S-030, ADR 0005 section 3): each
//! child runs in its own kill-on-close job object, so cancelling, the
//! timeout or the service dying kills the whole tree. Listed in
//! `.cargo/mutants.toml` `exclude_globs`, since the Linux mutation runs never
//! build this file; the decisions stay mutated in `office/tree.rs`.

use super::ProcessTree;
use crate::office::tree::{self, CREATE_NO_WINDOW, CREATE_SUSPENDED, JobOps};
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject,
};
use windows_sys::Win32::System::Threading::{
    OpenThread, ResumeThread, THREAD_SUSPEND_RESUME, TerminateProcess,
};

const _: () = assert!(CREATE_SUSPENDED == windows_sys::Win32::System::Threading::CREATE_SUSPENDED);
const _: () = assert!(CREATE_NO_WINDOW == windows_sys::Win32::System::Threading::CREATE_NO_WINDOW);

fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

struct WinOps<'a> {
    cmd: &'a mut Command,
}

impl JobOps for WinOps<'_> {
    type Job = OwnedHandle;
    type Child = Child;

    fn create_job(&mut self) -> io::Result<OwnedHandle> {
        // SAFETY: an unnamed job with default security; null means failure.
        let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if job.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a valid job handle that nothing else owns.
        Ok(unsafe { OwnedHandle::from_raw_handle(job) })
    }

    fn kill_on_close(&mut self, job: &OwnedHandle) -> io::Result<()> {
        // SAFETY: a plain C struct, valid when zeroed.
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        // SAFETY: the pointer and size describe `info`, alive for the call.
        check(unsafe {
            SetInformationJobObject(
                job.as_raw_handle(),
                JobObjectExtendedLimitInformation,
                (&raw const info).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        })
    }

    fn spawn_suspended(&mut self) -> io::Result<(Child, u32)> {
        let child = self
            .cmd
            .creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW)
            .spawn()?;
        let pid = child.id();
        Ok((child, pid))
    }

    fn assign(&mut self, job: &OwnedHandle, child: &Child) -> io::Result<()> {
        // SAFETY: both handles are open for the call.
        check(unsafe {
            AssignProcessToJobObject(job.as_raw_handle(), child.as_raw_handle() as HANDLE)
        })
    }

    fn threads_of(&mut self, pid: u32) -> io::Result<Vec<u32>> {
        // SAFETY: a thread snapshot of the whole system (the pid argument is
        // ignored for threads); INVALID_HANDLE_VALUE means failure.
        let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snap == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a valid snapshot handle that nothing else owns.
        let snap = unsafe { OwnedHandle::from_raw_handle(snap) };
        let mut entries = Vec::new();
        // SAFETY: a plain C struct, valid when zeroed, with its size set.
        let mut entry: THREADENTRY32 = unsafe { std::mem::zeroed() };
        entry.dwSize = size_of::<THREADENTRY32>() as u32;
        // SAFETY: `entry` is alive and sized for each call.
        let mut more = unsafe { Thread32First(snap.as_raw_handle(), &raw mut entry) };
        while more != 0 {
            entries.push((entry.th32OwnerProcessID, entry.th32ThreadID));
            // SAFETY: as above.
            more = unsafe { Thread32Next(snap.as_raw_handle(), &raw mut entry) };
        }
        Ok(tree::threads_of(pid, &entries))
    }

    fn resume(&mut self, tid: u32) -> io::Result<()> {
        // SAFETY: opens the thread by id; null means failure.
        let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, 0, tid) };
        if thread.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a valid thread handle that nothing else owns.
        let thread = unsafe { OwnedHandle::from_raw_handle(thread) };
        // SAFETY: the thread handle is open for the call.
        if unsafe { ResumeThread(thread.as_raw_handle()) } == u32::MAX {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn terminate_process(&mut self, child: &Child) {
        // SAFETY: the process handle is open. A failure means it is gone.
        unsafe {
            TerminateProcess(child.as_raw_handle() as HANDLE, 1);
        }
    }
}

impl ProcessTree {
    /// Spawns `cmd` suspended inside a new kill-on-close job, then resumes it.
    pub(crate) fn spawn(cmd: &mut Command) -> io::Result<ProcessTree> {
        let (job, child) = tree::start_in_job(&mut WinOps { cmd })?;
        Ok(ProcessTree { child, job })
    }

    /// Kills every process in the job. Dropping the tree closes the job.
    pub(crate) fn kill_all(&mut self) {
        // SAFETY: the job handle is open. A failure means the job is empty.
        unsafe {
            TerminateJobObject(self.job.as_raw_handle(), 1);
        }
    }
}
