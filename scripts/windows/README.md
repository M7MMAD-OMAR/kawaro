[English](README.md) | [العربية](README.ar.md)

# Windows build scripts

These scripts cross build and check the Windows programs on Linux, for the
`x86_64-pc-windows-gnu` target. They need `mingw64-gcc` and the Rust target
(`rustup target add x86_64-pc-windows-gnu`). Outside CI (`CI=true`), the
build scripts run themselves inside
`systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` with two jobs,
and print `cgroup: <path>` as their first line. The target dir defaults to
`${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target`; a `CARGO_TARGET_DIR` you set is
kept.

| command | what it does |
| --- | --- |
| `scripts/windows/build.sh` | builds `x86_64-pc-windows-gnu/release/kawaro.exe` |
| `scripts/windows/check-dlls.sh <exe>` | reads the DLL imports with `x86_64-w64-mingw32-objdump -p` and fails (exit 1) when any `lib*.dll` is imported, since mingw DLLs are not shipped; prints `dll check: ok <name> <N> imports` when clean. `--text <file>` reads saved objdump output instead |
| `scripts/windows/check.sh` | runs clippy for Windows on core, engines and the CLI, all targets, warnings as errors |
| `scripts/windows/make-iso.sh` | builds nothing; packs the release `kawaro.exe` (`bin/`), the test programs from `debug/deps` (`tests/`), `run-tests.cmd` and `SHA256SUMS` into `kawaro-win-tests.iso` in the target dir with `genisoimage -J -R` |
| `scripts/windows/run-tests.cmd` | on Windows, runs every `tests\*.exe` on the disc with `--test-threads=1` and writes `results.txt` (one `<exe> <errorlevel>` line each, then `total N failed F`) and `log.txt` in `%USERPROFILE%\kawaro-tests` |

A full check after a change:

```
scripts/windows/build.sh
scripts/windows/check-dlls.sh ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target/x86_64-pc-windows-gnu/release/kawaro.exe
scripts/windows/check.sh
```

For the Windows test machine, build the test programs, pack them, and attach
the ISO to the `win11` VM as a read only CD (not run by these scripts):

```
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli
scripts/windows/make-iso.sh
virsh -c qemu:///session attach-disk win11 ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target/kawaro-win-tests.iso sdb --type cdrom --mode readonly
```

Then run `D:\run-tests.cmd` (or the CD drive letter) in the VM.
