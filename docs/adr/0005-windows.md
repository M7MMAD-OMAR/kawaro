# ADR 0005: SbarHive on Windows (release 0.6.0)

Status: proposed
Date: 2026-10-22

## Context

The 0.6.0 goal in `docs/PLAN.md`: a user on Windows 10 or 11 selects a file in Explorer and picks SbarHive from the right-click menu; the hive appears at the pointer and the output is saved next to the original, in Arabic and English, without any file leaving the machine. This document settles the build tools, IPC, starting the daemon, paths, the tray, notifications, focus, drop and the shortcut, Explorer integration, engines, the installer, testing, and the split into stories.

Nothing has run on Windows yet. In the `win11` machine (libvirt user session, EFI, 6 GB, user-mode network, spice channels, no guest agent and no snapshots) Windows may not be fully installed, and the machine was not started for this document. Therefore: (verified) means a line read in code with its version and path, or a command run on the development machine, and (assumed) means untested behaviour on Windows, to be verified in the live checklist.

**Starting point (verified, 2026-10-22):** on Fedora with `mingw64-gcc` 16.2.1 and rust-std for `x86_64-pc-windows-gnu`, without rustup and without MSVC, `cargo check --workspace --target x86_64-pc-windows-gnu` fails in two places only: `crates/cli/src/client.rs` (`std::os::unix::net`, `exec` and `process_group`) and `crates/ui/src/lib.rs:242-246`, because `winit::platform::startup_notify` is defined for X11 and Wayland only (winit 0.30.13, `src/platform/mod.rs:13`). The check did not reach the daemon because it depends on ui. The daemon also has `UnixListener` and `UnixStream` (`main.rs`, `server.rs` and `state.rs:32`), ksni, which is Linux only, and `xdg-open` in `notify.rs`.

## Findings and decisions

### 1. Build tools

- **Decision:** cross building with `x86_64-pc-windows-gnu` from Linux is the primary path, locally and in CI on ubuntu. CI has a second job on `windows-latest` that runs `cargo test` for core, engines and cli natively with MSVC, so the code is tested on real Windows without the machine. The published release is the same gnu build the developer builds.
- **Reason:** the development machine has no MSVC and no rustup, and the starting point proves that all dependencies compile for gnu. The renderer is software (ADR 0001), and on Windows softbuffer draws with GDI, so we need neither skia nor heavy C++ linking. Whether prebuilt skia binaries exist for gnu does not concern us (assumed to be MSVC only; not verified).
- **Rejected:** MSVC for the release, because every build needs a Windows runner and cannot be reproduced on the developer's machine. `cargo-xwin`, because it downloads the CRT and SDK from Microsoft by accepting their licence. `x86_64-pc-windows-gnullvm`, because its rust-std is not installed and it shows no advantage here.
- **Linking:** the first build story (not just `cargo check`) verifies that the executables import only system libraries: `x86_64-w64-mingw32-objdump -p sbarhived.exe | grep 'DLL Name'`, that is, no `libgcc_s_seh-1.dll` and no `libwinpthread-1.dll`. If they appear, `-static` is added to the gnu linker. libwebp-sys is built by the same mingw gcc (assumed to be linked statically).
- **Subsystems:** `sbarhived.exe` uses `windows_subsystem = "windows"`, so no console window appears when Explorer starts it, and its log goes to `%LOCALAPPDATA%\SbarHive\logs\sbarhived.log` instead of stderr. `sbarhive.exe` stays a console application for the command line. Checks that count `notify:` lines in stderr read the log file on Windows.
- **Extra gate for Windows stories:** `cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings` inside the limited systemd scope (`MemoryMax=6G`, `-j 2`).

### 2. IPC over a named pipe

- **Decision:** the messages stay as they are: one JSON line per `Request` and `Response` from `core::ipc`, and the pipe is in byte mode (`PIPE_TYPE_BYTE | PIPE_READMODE_BYTE`), so `BufRead::read_line` works as on Unix. The `token` field stays `None` on Windows, so the schema does not change.
- **Name:** `\\.\pipe\io.github.m7mmad_omar.SbarHive.<SID>.<session>`, from the user's SID and the session number (`ProcessIdToSessionId`), because the pipe namespace is shared by all users and sessions. A pure function in core builds it and is tested on Linux, like `socket_path()` today.
- **Security:**
  - An explicit DACL grants full access to the current user alone. The default descriptor grants read access to Everyone and to the anonymous account (pipe security page), so we do not accept it.
  - `PIPE_REJECT_REMOTE_CLIENTS` rejects clients over the network.
  - The client verifies that the owner of the server process (`GetNamedPipeServerProcessId`, then the process token) is the same user, so a process that grabbed the name first cannot fool it. This corresponds to the `$XDG_RUNTIME_DIR` directory with 0700 permissions on Linux.
- **Single instance:** the first `CreateNamedPipeW` uses `FILE_FLAG_FIRST_PIPE_INSTANCE`. Its failure with `ERROR_ACCESS_DENIED` means a daemon is running, and this replaces the `already_running` ping check.
- **Timeouts:** pipes have no `set_read_timeout`. Connecting uses `WaitNamedPipeW` with a timeout, and a read with a timeout (500 ms in the daemon check, 10 s in the client) uses a watchdog thread that calls `CancelIoEx` when it expires. `try_clone` for subscribers (`state.rs`) uses `DuplicateHandle` through `File::try_clone`. The handle is synchronous, so its operations are serialised, and this does no harm: after reading the request the server only writes, and after sending the client only reads (assumed).
- **Shape in code:** a transport module in core (`Listener` and `Stream` with the same interface on both platforms), Unix on top of `UnixListener` as today, and Windows in a separate file `transport/windows.rs` on windows-sys, added to `exclude_globs` in `.cargo/mutants.toml` like `non_unix.rs`. The daemon and the command line use it instead of `std::os::unix` directly.
- **Rejected:** `interprocess` 2.4.6 (0BSD OR Apache-2.0). It has `security_descriptor`, `server_process_id` and `FILE_FLAG_FIRST_PIPE_INSTANCE` (verified in code, `src/os/windows/named_pipe/listener/create_instance.rs`), but it adds three new crates (`doctest-file`, `recvmsg` and `widestring`), and we build the security descriptor ourselves in either case. The direct code is about 200 lines.

### 3. Starting the daemon and stopping the process tree

- **From the command line:** `ensure_daemon` starts `sbarhived.exe` with `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB` through `CommandExt::creation_flags`. If breaking away from the terminal's job is refused (`ERROR_ACCESS_DENIED`), it starts again without the last flag; otherwise a kill-on-close job would kill the daemon along with the command line. `exec_daemon` becomes start, wait, then exit with the daemon's code.
- **From Explorer:** the registry command calls `sbarhived.exe open "%1"` directly, not `sbarhive.exe`. If a daemon is running, it becomes a client: it grants the daemon the foreground right (section 5), sends `Open` and exits. Otherwise it becomes the daemon and shows the hive itself, so there is no cost of starting a third process.
- **Engines as child processes (`office/non_unix.rs`; since S-030 in `process/windows.rs`, FFmpeg included):** one job object per conversion job, with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. To avoid the race of children spawned before assignment: start with `CREATE_SUSPENDED`, then `AssignProcessToJobObject` with the `Child` handle, then resume its only thread (`CreateToolhelp32Snapshot`, `OpenThread` and `ResumeThread`), because `Child` in std does not expose the thread handle, and `raw_attribute` (`PROC_THREAD_ATTRIBUTE_JOB_LIST`) is unstable. Cancellation and timeout use `TerminateJobObject`, and closing the handle kills whatever remains even if the daemon dies. This matters because `soffice.exe` starts `soffice.bin` as a child (assumed).

### 4. Paths and atomic writes

- Settings and actions: `%APPDATA%\SbarHive\config.toml` and `%APPDATA%\SbarHive\actions\`. Log: `%LOCALAPPDATA%\SbarHive\logs\`. The variables are read, then `SHGetKnownFolderPath` if they are missing. `user_actions_dir` moves from `daemon/state.rs` to core next to `Config::path`, and the choice becomes a pure function that takes the environment, so it is tested on Linux.
- Temporary files: `std::env::temp_dir()` gives `%TEMP%` (`GetTempPath2W`), with no change in `tempfile`. The temporary output file stays next to the output (`output_impl::temp_path`), so it is always on the same disk.
- **No new rename code.** `std::fs::rename` replaces the target if it exists; on Windows it uses `MoveFileExW` with a fallback to `SetFileInformationByHandle`, and on Windows 10 1607 and later it matches Unix behaviour where the file system supports `FileRenameInfoEx` (std documentation), so saving `config.toml` is atomic as it is. `finalize` starts with `hard_link` (`CreateHardLinkW`), which fails if the target exists, and works on NTFS. On FAT and exFAT (USB drives) it falls back to the check then `rename`. A small improvement for Windows alone: this fallback becomes `MoveFileExW` without `REPLACE_EXISTING`, so it fails if the target exists, and the race documented in `output_impl.rs:116` goes away.

### 5. Tray, notifications, focus, drop and the shortcut

- **Shell thread:** one thread in the daemon with a Win32 message loop and a hidden top-level window (not `HWND_MESSAGE`, because message-only windows do not receive the `TaskbarCreated` message broadcast when Explorer restarts). It hosts the tray icon and the shortcut. COM and WinRT are not initialised on the main thread before winit, because winit panics on `RPC_E_CHANGED_MODE` from `OleInitialize` (verified in code, winit 0.30.13, `platform_impl/windows/window.rs:1168-1175`).
- **Tray:** `Shell_NotifyIconW` directly on windows-sys: a tooltip with progress (128 characters at most), and a `TrackPopupMenu` menu with the same three items, with `MFT_RIGHTORDER` for Arabic. The icon is an `.ico` file next to the program (`LoadImageW` with `LR_LOADFROMFILE`), so no resource compiler is needed for now. ksni becomes a dependency for `cfg(target_os = "linux")` only, and the two `tray` modules share the same interface (`start`, `set_active` and `set_lang`).
  - **Rejected:** `tray-icon` 0.26.1 (MIT OR Apache-2.0), because it pulls in `muda ^0.21` while Slint pulls in `muda` 0.19.3, so two versions enter the tree, and because it needs a message loop on its own thread anyway.
- **Notifications:** the existing notify-rust 4.18.2, which on Windows uses `tauri-winrt-notification` 0.8.1 (MIT OR Apache-2.0, present in `Cargo.lock`). Without `app_id` it sends under the PowerShell name (verified in code, `notify-rust-4.18.2/src/windows.rs:76-79`). Therefore `.app_id("io.github.m7mmad_omar.SbarHive")`, and the installer registers this identifier (AUMID) with a Start menu shortcut and with the key `HKCU\Software\Classes\AppUserModelId\<id>` holding `DisplayName` and `IconUri`. "Open folder" becomes `explorer.exe /select,<file>` instead of `xdg-open`, and `wait_for_action` exists on Windows (`windows.rs:111`).
- **The hive is a winit window on Windows** (the rule in CLAUDE.md and ADR 0004): undecorated, transparent, always on top, with no taskbar button (`with_skip_taskbar`). The position is at `GetCursorPos`, and the work area of the monitor under the pointer (`MonitorFromPoint` and `GetMonitorInfoW`, that is, `rcWork`) is passed to the same `layout::plan_at`, so the shape adapts to the edges as on the layer path.
  - **Design rules:** no shadow: `with_undecorated_shadow(false)`, which is the default anyway (verified, winit 0.30.13, `platform_impl/windows/mod.rs:52`), and is written explicitly. No Windows 11 corner rounding: `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND`. There is no compositor blur here, so surfaces stay opaque and the glass tokens are not used, as on GNOME. Transparent DWM backdrops (Acrylic) are a later decision.
  - **First risk: transparency.** softbuffer 0.4.8 presents with `Gdi::BitBlt` (verified, `vendor/softbuffer/src/backends/win32.rs:189`), and winit makes the window transparent with `DwmEnableBlurBehindWindow` and an empty region (`window.rs:1231-1242`). Does the alpha channel survive `BitBlt`? (assumed, unknown). If it does not, the corners appear black as happened on Wayland. This is the first check on the machine, and the two alternatives in order: a change in our copy of softbuffer that presents with `UpdateLayeredWindow` for a `WS_EX_LAYERED` window with premultiplied alpha, which is what the Slint renderer writes, following the pattern of the Wayland change documented in `SBARHIVE-PATCH.md`; then a window region from the union of the hexagons (`SetWindowRgn`) with non-antialiased edges, which confines clicks to the shape for free.
- **Focus:** Windows allows `SetForegroundWindow` for a process started by the foreground process, or one that received the last input event, or one that a foreground process allowed with `AllowSetForegroundWindow`, and the permission is dropped at the user's first input. The client `sbarhived.exe` is started by Explorer, which is the foreground process, so it calls `AllowSetForegroundWindow(daemon pid)` (from `GetNamedPipeServerProcessId`) before sending `Open`. Then the daemon calls `SetForegroundWindow` when it shows. This is the counterpart of the xdg-activation token, and so `token_hook` in `lib.rs` is replaced on Windows and the original is confined to X11 and Wayland. `sbarhive.exe` from the terminal does the same. Closing on the window path stays as it is: Esc, a selection, or focus loss.
- **Drop (the "hive first, then drag" order, ADR 0003):** winit 0.30.13 registers an `IDropTarget` with OLE and answers with `DROPEFFECT_COPY` only, so Explorer does not delete the original, which satisfies the copy-only rule. It sends `HoveredFile` on enter and `DroppedFile` on drop. But it ignores the `_pt` position and sends nothing in `DragOver` (verified, `platform_impl/windows/drop_handler.rs:79-150`), and Slint 1.18.1 does not pass on the two events. So on its own it is not enough to select a cell.
  - **Decision:** we catch the two events in the existing `FocusWatcher::window_event`, and read `GetCursorPos` on a 16 ms timer between `HoveredFile` and `DroppedFile` or `HoveredFileCancelled`, so `Layout::pick` highlights the cell and selects it, as the X11 path does with `QueryPointer`.
  - **Rejected for now:** our own `IDropTarget` (`with_drag_and_drop(false)` then `RegisterDragDrop` with the `windows` 0.62.2 crate already in the tree). It stays an alternative if polling fails.
  - **The S-021 rule** (the drop hive appears only for a real drag): on Windows it appears from the shortcut only while the left mouse button is held (`GetAsyncKeyState(VK_LBUTTON)`). The files are known only at `HoveredFile`, as on Wayland.
- **Global shortcut:** `RegisterHotKey` on the shell thread's window. Only the press arrives, so the release (`drop --hold` and `--release`, S-019e) uses polling of `GetAsyncKeyState` after `WM_HOTKEY`, which is the same method as `global-hotkey` 0.9.0 (verified, `src/platform_impl/windows/mod.rs:153-163`). We write it directly and do not add the crate (Apache-2.0 OR MIT), because it opens its own window and thread and pulls in `windows-sys` 0.59. On Windows the daemon reads the pointer position itself, so `--at` is not needed. Does `WM_HOTKEY` arrive during an OLE drag from Explorer? (assumed, a check on the machine).

### 6. Explorer integration

- **Decision for 0.6.0:** a legacy verb in the registry under `HKCU\Software\Classes\*\shell\SbarHive` (and `Directory\shell` later if we need it), with `MUIVerb`, `Icon` and `MultiSelectModel = Player`, and the command `"<dir>\sbarhived.exe" open "%1"`. No administrator rights and no signing. The installer writes it, and `sbarhive integration install` writes it and `uninstall` removes it for those who use the zip file. The key text is a pure function in core, tested on Linux.
- **Multiple selection:** the legacy verb with a command is invoked once per file (assumed from the documentation, to be verified on the machine), and its limit is 100 items for Player and 15 for Document. So the daemon merges consecutive `Open` requests: the first shows the hive at once, and whatever arrives within 300 ms of the last request is added to the selection and the cells are rebuilt for the common type, so the 150 ms budget holds for the first show. On a cold start Explorer starts several `sbarhived.exe` instances together: one wins with `FILE_FLAG_FIRST_PIPE_INSTANCE` and creates the pipe before `ui::build`, and the rest become clients that wait with `WaitNamedPipeW` within the same timeout, and their requests fall into the same merge (assumed). If flicker or slowness shows up on the machine, the alternative is a `DropTarget` verb: an out-of-process COM server (`LocalServer32` under `HKCU\Software\Classes\CLSID`, with no administrator and no signing) that receives all items in one `IDataObject` with no limit. It gets its own story if needed.
- **Windows 11:** legacy verbs appear under "Show more options" (or Shift+F10), not in the first menu. The modern menu needs `IExplorerCommand` in a DLL inside Explorer, and a sparse package that grants identity, and both need signing with a certificate. **Out of scope for 0.6.0** until the owner provides a certificate; its place is `integrations/windows/{explorer-command,sparse-package}/` as in CLAUDE.md. This departs from the letter of the PLAN goal on Windows 11, so it needs the owner's acceptance.

### 7. Engines

- The image engine stays as it is.
- **Discovery:** `PATH` first, then the known paths: `%ProgramFiles%\LibreOffice\program\`, `%ProgramFiles%\gs\gs*\bin\` (the newest), and an `ffmpeg\bin` folder next to the program if the user put it there. Then the registry: `HKLM\SOFTWARE\LibreOffice\UNO\InstallPath` and `HKLM\SOFTWARE\GPL Ghostscript\<ver>` (assumed, to be verified on the machine).
- **Windows names:** `soffice.com` then `soffice.exe` (the first is a console wrapper that waits for completion, assumed), `gswin64c.exe` (present in `pdf.rs:272`), and `ffmpeg.exe` and `ffprobe.exe`. `is_executable` in `non_unix.rs` accepts `.exe` and `.com` only.
- **Required fix:** `office::file_url` percent-encodes `:` and `\`, so for `C:\a b\x` it gives a wrong URL. On Windows it must give `file:///C:/a%20b/x`. A pure function tested on Linux with Windows paths as strings. The `file:` prefix in ffmpeg with a drive letter (`file:C:\...`) is an item in the live checklist.
- **No automatic download in 0.6.0.** The "engine not found" message names the program and the link to its official site. Download on demand is decided later, because it touches signature verification and updates.

### 8. Installer

- **Decision:** a per-user MSI (`InstallScope="perUser"`, in `%LOCALAPPDATA%\Programs\SbarHive`), built with `wixl` from msitools 0.106 on Linux, locally and in CI on ubuntu. It writes the verb keys, the AUMID key, the Start menu shortcut with the identifier, and starting the daemon at logon (`HKCU\...\Run`), and removes them all on uninstall. Alongside it, a portable zip file and the `sbarhive integration install` command.
- **Reason:** no administrator rights, a clean uninstall from "Apps", and building on the same host without Windows. msitools is a build tool only and is not shipped, and its licence (LGPL-2.1-or-later, GPL-2.0-or-later and MS-RL) does not affect the program.
- **Rejected:** WiX 4 and later (.NET on Windows, that is, CI only). Inno Setup (Windows only, or wine). NSIS (`mingw32-nsis` 3.11, Zlib) stays an alternative if `wixl` cannot install per user or create the shortcut with an identifier, because `wixl` supports only part of the WiX 3 schema (assumed, the first thing to verify).
- **Unsigned in 0.6.0:** SmartScreen shows "Windows protected your PC" when running from a downloaded file, and the bypass is "More info" then "Run anyway". This is documented in README in Arabic and English with a SHA-256 fingerprint for each file. Signing comes later with `osslsigncode` 2.12 (GPL-3.0-or-later) on Linux when a certificate is available.

### 9. Testing

- **On Linux, in every story:** the pure logic (the pipe name, choosing paths from the environment, the registry key text, `file_url`, merging) lives in shared modules, so it is tested and mutated on Linux. The thin Windows code lives in separate files added to `exclude_globs`, and the tests on Windows cover it.
- **Unit tests on Windows:** the `windows-latest` job in CI runs `cargo test` for core, engines and cli. On the machine: `cargo test --no-run --target x86_64-pc-windows-gnu` builds the test binaries, and a script `scripts/windows/run-tests.cmd` runs them and writes the results to a file. wine is not installed (wine-core 11.0 is available in Fedora). It is suitable for core and engines tests without a UI if the owner agrees to install it, but pipe ACLs and focus can only be judged on real Windows.
- **Transfer to the machine without an agent:** an ISO image with the files is attached with `virsh -c qemu:///session attach-disk win11 <iso> sdb --type cdrom --mode readonly`, which is a guaranteed route. The user-mode network (slirp or passt, not verified) may allow HTTP to the host, so it is not relied on until tried. The best option, with the owner's approval: install the virtio-win guest tools, which include `qemu-guest-agent`, so running and reading results becomes automatic through `virsh qemu-agent-command` (`guest-exec` and `guest-file-read`), along with spice-webdavd for a shared folder from Boxes.
- **What stays manual in the machine's UI (the live checklist):** transparency of the hive's corners; its position at the pointer, at the edges and at 150% scale; focus and Esc when opened from Explorer with one file, then with ten files; dropping from Explorer onto a cell and confirming the original remains; the shortcut during a drag; the tray and its menu in Arabic; the notification and its open-folder button; the three engines after installing them; cancellation killing `soffice.bin`; the installer and uninstall; the SmartScreen warning; and Narrator on the hive (accesskit_windows is in the tree through Slint). Every test window has a fixed bounded size, its actual geometry is verified, and only what the test opened is closed.
- **Recovery:** a clean snapshot of the machine before each round if the owner agrees to create one; with EFI the snapshot is external.

## Crates

No new crate in `Cargo.lock`. The new item is one direct dependency under `cfg(windows)`:

| Crate | Version | Licence | Where | Note |
| --- | --- | --- | --- | --- |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | core, engines, cli, ui, daemon | already in the tree through tempfile, rustix, winit and others |
| notify-rust (existing) ← tauri-winrt-notification | 0.8.1 | MIT OR Apache-2.0 | daemon | already in the tree, enabled on Windows only |
| windows (fallback only) | 0.62.2 | MIT OR Apache-2.0 | ui | if we need our own `IDropTarget`; already in the tree |

Rejected: `interprocess` 2.4.6 (0BSD OR Apache-2.0), `tray-icon` 0.26.1 (MIT OR Apache-2.0) and `global-hotkey` 0.9.0 (Apache-2.0 OR MIT). Build tools (not shipped): mingw64-gcc 16.2.1 (installed), msitools 0.106, mingw32-nsis 3.11 as a fallback, osslsigncode 2.12 later, and wine-core 11.0 optionally. All are in the Fedora repository, and those not installed need the owner's approval. They are recorded in `docs/engines.md` with the first story that adds them.

## Proposed stories

Numbering starts at S-040, because S-021 is taken ("the drop hive appears only for a real drag"), and the PLAN.md phrase "numbered from S-021" is wrong. Stories S-040a to S-044 do not touch `crates/ui`, because another session is editing `crates/ui/src/layer.rs` now, and they are verified with `cargo check -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --target x86_64-pc-windows-gnu`. The daemon is not checked for Windows before S-045 because it depends on ui, but its changes in S-040a and S-041 are built and tested on Linux.

| Story | Scope | Touches ui |
| --- | --- | --- |
| S-040a | The transport interface in core with the current Unix implementation, the pure pipe name function, and moving `client.rs` and the daemon onto it; all tested on Linux | no |
| S-040b | The Windows pipe: a DACL for the user alone, rejecting remote clients, the single instance, the read timeout with `CancelIoEx`, and verifying the server's owner | no |
| S-041 | Paths: `%APPDATA%` and `%LOCALAPPDATA%`, moving `user_actions_dir` to core, and the `finalize` fallback with `MoveFileExW` without replacing | no |
| S-042 | Starting the daemon from the command line (the flags and the fallback when breaking away from the job is refused), the `exec_daemon` replacement, and `AllowSetForegroundWindow` before `Open` and `Drop` | no |
| S-043a | The job object with suspend, then assign, then resume in `non_unix.rs`, and `is_executable` | no |
| S-043b | Program names on Windows, their paths and the registry, and the `file_url` fix | no |
| S-044 | Build: a cross build script inside the limited scope, the `objdump` check, the clippy gate for the target, the two CI jobs, the ISO script and `run-tests.cmd` | no |
| S-045 | The first ui story: a `cfg` gate for `token_hook` and the Wayland and X11 specifics in `lib.rs` only, ksni for Linux, empty tray and notification modules for Windows, and `windows_subsystem` and the log file, so the whole workspace builds for Windows | yes (after layer.rs is finished) |
| S-046 | The hive window: transparency and its alternatives, the position at the pointer within the work area, no shadow and no rounding, focus with `SetForegroundWindow`, client mode in `sbarhived open`, and merging multiple selections | yes |
| S-047 | Drop: `HoveredFile` and `DroppedFile` in `FocusWatcher`, polling `GetCursorPos`, `Layout::pick`, and the S-021 rule | yes |
| S-048 | The shell thread: the tray icon, its menu in Arabic and `TaskbarCreated`, and the shortcut with press and release | no (daemon) |
| S-049 | Notifications: `app_id`, `explorer.exe /select`, and counting notifications from the log file | no (daemon) |
| S-050 | Explorer integration: `integrations/windows/registry/`, and `sbarhive integration install` and `uninstall` | no |
| S-051 | The installer: a per-user MSI with `wixl` (or NSIS), the AUMID, the shortcut, starting at logon, uninstall, and uploading it with the release | no |
| S-052 | The live checklist on the machine, updating README and CHANGELOG, and the 0.6.0 tag | no |

S-048 to S-051 do not depend on ui, so they can come before S-046 if the wait for layer.rs runs long, but checking them for Windows needs S-045. Each story is a working day or less, and S-046, if it grows, splits into S-046a (the window, transparency and position) and S-046b (focus, client mode and merging).

## What the owner needs to provide

- [ ] Windows 11 fully installed in `win11` with a local account (the machine's state may be "installation"), and preferably Windows 10 22H2 as well if it is still supported.
- [ ] Permission to install inside the machine: the virtio-win guest tools (which include `qemu-guest-agent`) and spice-webdavd, and FFmpeg, LibreOffice and Ghostscript for checking the engines.
- [ ] Permission to create a clean snapshot of the machine.
- [ ] Permission to install tools on the host: msitools, xorriso or genisoimage for ISO images, and optionally wine-core and mingw32-nsis.
- [ ] A signing certificate (OV or EV, or a cloud signing service) for the installer and for the sparse package. Without it: the legacy menu alone, and the SmartScreen warning.

## Still open

- [ ] Does the owner accept that SbarHive appears on Windows 11 under "Show more options" in 0.6.0?
- [ ] The 300 ms merge or a `DropTarget` verb for multiple selection? Settled by measurement on the machine.
- [ ] The default shortcut on Windows (many Win combinations are reserved by the system), and whether "long press" stays as on Hyprland.
- [ ] The minimum: Windows 10 22H2 or 11 only? And is arm64 out of scope?
- [ ] softbuffer transparency through `BitBlt`, `WM_HOTKEY` arriving during an OLE drag, the behaviour of `soffice.com`, the `file:` prefix in ffmpeg, and the ability of `wixl` to install per user: all (assumed) until the first round on the machine.

## Sources

- Pipe security and the default descriptor: https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights
- `CreateNamedPipeW` (`PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE`): https://learn.microsoft.com/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew
- `GetNamedPipeServerProcessId`: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid
- `SetForegroundWindow` and its conditions: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow
- `AllowSetForegroundWindow`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-allowsetforegroundwindow
- Process creation flags: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
- Job objects: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- `MoveFileExW`: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw , and `std::fs::rename` on Windows: https://doc.rust-lang.org/std/fs/fn.rename.html
- Known folders: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid
- `Shell_NotifyIconW`: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw
- `RegisterHotKey`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey
- AppUserModelID identifiers, which notifications from an unpackaged application need: https://learn.microsoft.com/en-us/windows/win32/shell/appids
- Static verbs, `MultiSelectModel` and its limits, and the `IDropTarget` method: https://learn.microsoft.com/en-us/windows/win32/shell/context-menu-handlers
- The Windows 11 menu with `IExplorerCommand`: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer , and identity through a sparse package: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/grant-identity-to-nonpackaged-apps
- SmartScreen: https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/
- winit 0.30.13 (`drop_handler.rs`, `window.rs`): https://github.com/rust-windowing/winit/tree/v0.30.13/src/platform_impl/windows
- softbuffer 0.4.8 (`win32.rs`): https://github.com/rust-windowing/softbuffer/blob/v0.4.8/src/backends/win32.rs
- notify-rust 4.18.2: https://github.com/hoodie/notify-rust , and tauri-winrt-notification: https://github.com/tauri-apps/winrt-notification
- msitools and `wixl`: https://gitlab.gnome.org/GNOME/msitools , NSIS: https://nsis.sourceforge.io , and osslsigncode: https://github.com/mtrojnar/osslsigncode
