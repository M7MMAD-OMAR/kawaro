[English](macos-gap.md) | [العربية](macos-gap.ar.md)

# macOS gap: what fails on `aarch64-apple-darwin`

Measured by the `macOS` workflow (`.github/workflows/macos.yml`, ADR 0009, story S-069) on `macos-14`, Rust stable, at the 0.4.0-beta.1 sources. Runs: https://github.com/M7MMAD-OMAR/kawaro/actions/runs/40244771468 and https://github.com/M7MMAD-OMAR/kawaro/actions/runs/40245043420 (the second adds `cargo check --keep-going`). Both runs finished green because the cargo steps are informational.

Result: `cargo check --workspace --all-targets` fails with exactly 2 errors, both in `crates/ui`. `cargo test --workspace --no-fail-fast` stops at the same compile error, so **no test has run on macOS yet**.

## Per crate

| Crate | macOS result | Error class | Proposed next story |
| --- | --- | --- | --- |
| `crates/core` (`kawaro-core`) | Compiles | none | Run its tests in isolation (see S-069a) |
| `crates/engines` (`kawaro-engines`) | Compiles | none (engines are external processes, so their tests will still need ffmpeg, Ghostscript and others installed on the runner) | S-069a, then S-069c |
| `crates/cli` (`kawaro-cli`) | Compiles (`Checking kawaro-cli` finished without errors) | none seen | S-069a |
| `crates/ui` (`kawaro-ui`) | Fails, 2 errors | Missing `cfg` gate. `crates/ui/src/lib.rs:254` imports `winit::platform::startup_notify` (E0432), which winit 0.30.13 builds only for X11 and Wayland (`cfg(any(x11_platform, wayland_platform))`). `crates/ui/src/lib.rs:258` then calls `with_activation_token` (E0599), which exists only through that trait. The function `token_hook` is gated `#[cfg(unix)]`, and macOS is Unix but has no startup notification. | S-069b: narrow the gate to `cfg(all(unix, not(target_os = "macos")))` with a no-op `token_hook` for macOS |
| `crates/daemon` (`kawaro-daemon`) | Not reached: depends on `kawaro-ui`, so its own errors are hidden | Unknown. Guesses, not findings: the daemon's use of `ashpd` (the crate itself compiles on macOS; the GlobalShortcuts portal has no macOS backend and is gated `cfg(unix)`), `libc` use, `/proc` and `XDG_*` paths | S-069b rerun after the ui fix, then S-069d if errors appear |

Other facts from the logs:
- `wayland-*` crates in `crates/ui` are already behind `cfg(target_os = "linux")` and caused no error. Slint's own dependencies built without trouble.
- The vendored `softbuffer` compiled.

## Proposed stories

- **S-069a** Run the tests of `core`, `engines` and `cli` on macOS per package (`cargo test -p kawaro-core -p kawaro-engines -p kawaro-cli`), so a ui error stops hiding them. Output: the first real list of failing tests (paths, `/proc`, XDG directories, symlink and rename behaviour are the usual suspects).
- **S-069b** Fix the `startup_notify` gate in `crates/ui` (above), then re-run to see the daemon errors.
- **S-069c** Install the engines on the runner (`brew install ffmpeg ghostscript imagemagick`) so engine adapter tests can run, and decide where macOS finds its engines (Homebrew prefix `/opt/homebrew/bin` is not on the default search path of a GUI app).
- **S-069d** Replace Linux only assumptions in the daemon and core (config, cache and runtime directories, IPC socket location, tray) with macOS equivalents, one story per finding.
- **S-069e** Make the job required once it is green, and add it to pull requests.
- Later, on a real Mac (ADR 0009): Finder integration, signing, notarization, hive focus and placement. A headless runner cannot show these.

## Not verified

- Anything that needs a window server or a file manager.
- The daemon and the tests of every crate (hidden by the ui error, see above).
- Intel macOS (`macos-13` and `x86_64-apple-darwin`) was not tried.
