# S-101 and S-100 Windows 11 live check (2026-10-30)

Setup: win11 VM (libvirt session), driven through the QEMU guest agent (exec and file write), screendump and
input-send-event for clicks. Orbit could not list the VM window (`orbit_handoff_candidates` returned BACKEND_ERROR),
so the virsh fallback was used. Builds: current main (HEAD 9c2262f) with only the workspace version changed
(Cargo.toml and Cargo.lock restored afterwards), base = 0.4.2, candidate = 0.4.3 with a temporary
`docs/whats-new/0.4.3.json` (removed after the build, not committed). Manifest signed with the real update key
(`scripts/make-update-manifest.sh`, `scripts/sign-update-manifest.sh`). The VM reached a local throttled HTTPS
server (about 500 KB/s) as github.com (hosts entry, a new test CA imported into LocalMachine Root, leaf with a CRL
distribution point served on port 80, port proxies 443 and 80 to the host). The base installs were MSIs built from
current main, not the released 0.4.2 MSI.

## Goal 1: S-100 AC4, the new daemon is the running process (PASS)

`goal1-msiexec-upgrade-live-daemon.txt`. Run as the signed in user (elevated, session 1), `msiexec /i` base 0.4.2
then `msiexec /i` 0.4.3, both `/qn`:

- base rc 0, daemon pid 5532 version 0.4.2; upgrade rc 0, daemon pid 10988 version 0.4.3 started 8 s later.
- the old pid 5532 is gone, exactly one kawarod process remains, path `%LOCALAPPDATA%\Programs\Kawaro\kawarod.exe`.
- installed kawarod.exe SHA-256 starts 3d74485d0556, equal to the 0.4.3 package build.
- `config.toml` and the `s100-sentinel.txt` in `%APPDATA%\Kawaro` kept; `actions` and `kawaro.ico` present.

Note: the install is per user (`%LOCALAPPDATA%\Programs\Kawaro`), not Program Files.

## Goal 2: in-app update 0.4.2 to 0.4.3 (PASS, with two small defects)

Ask mode (`update_mode = "ask"`), English UI, Updates page.

| Check | Result | Evidence |
| --- | --- | --- |
| version at start | `Installed version: 0.4.2`, header 0.4.2 | 01 |
| checking then Downloading with percent | `Downloading 0.4.3: 10% (1.7 of 17.2 MB)` up to 95%, both buttons greyed | 02, 03 |
| ask mode needs Update now | after the download: `Download finished. Ready to install 0.4.3.`, nothing installed, no msiexec started for 30 s, Update now enabled | 04 |
| Installing line | `Installing 0.4.3. Kawaro will restart by itself.`, both buttons greyed | 05 |
| restart and reopen | window closes, new kawarod (pid 4712, old 3660) starts, settings window reopens on its own about 5 s later on the Updates page | 06, 07 |
| message and What's new | `Updated to 0.4.3`, `Installed version: 0.4.3`, card `What is new in 0.4.3` with the four notes | 07 |
| files remain | kawaro.exe (d3013eb5ece9), kawarod.exe (1b3ef7536c05) equal the new build, `actions`, `kawaro.ico`, config and sentinel kept | NOTES |
| failed install | corrupt MSI with a valid signed manifest (0.4.4): red `Update to 0.4.4 did not install. Kawaro is still 0.4.3.` and `The update could not be downloaded or installed. Check for updates to try again.`, toast `Update 0.4.4 failed. The current version is unchanged.`, no up to date claim, daemon relaunched (still 0.4.3) | 08, 09, 10 |

First pass (Arabic UI, build without bundled notes): `07a` shows `Updated to 0.4.3` in Arabic and no What's new card.
That was my build (no `docs/whats-new/0.4.3.json` at compile time, the notes are bundled by `crates/core/build.rs`),
not a product defect; it also shows the card depends on the bundled file, not on the manifest notes, after the update.

## Defects and observations (crates not touched)

1. The reopened settings window does not come to the foreground. In both successful runs it opened behind the
   Edge window that was in front (`06`: only a strip with Cancel and Save is visible below Edge); the taskbar icon
   brought it up. In the failed run it opened in front. Windows foreground lock after a restart by a background
   process; the reopen should request activation (or flash and bring to front).
2. Ask mode downloads the package as soon as Check for updates finds one (percent shown, then ready); only the
   install waits for Update now. Probably intended, listed because the task text could read either way.
3. Stale cache: after a successful update `%LOCALAPPDATA%\Kawaro\updates\0.4.3\` keeps the MSI and its log
   (the old build of 0.4.3 stayed and was correctly re-downloaded because the hash differed). Cleanup of the
   installed version's directory is missing or late.
4. The Windows toast header reads `io.github.m7mmad_omar.Kawaro` (the AppUserModelID) instead of `Kawaro`.
5. The settings window title is `Welcome to Kawaro` (and the Arabic equivalent) on every page; the window is also
   taller than the 1080p screen, the Save and Cancel bar sits under the taskbar at the first position.
6. The language buttons change the UI at once but only persist after Save (by design, noted so it is not a bug).

## Not done or still open

- Auto mode install path and the real released 0.4.2 MSI as the base were not rerun.
- Welcome film, SmartScreen and Defender are out of scope.
- VM leftovers: test CA `Kawaro S101 local test CA` in the root store (valid 30 days), the hosts entry and port
  proxies from the earlier agent, `C:\s101` with the two MSIs and logs. VM config restored to `language = "ar"`,
  `update_mode = "auto"`. Kawaro 0.4.3 (build of this session) is installed.
