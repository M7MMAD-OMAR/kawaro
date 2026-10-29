# Microsoft Defender false positive (S-098)

Measured 2026-10-29 on Windows 11 (Edge download, then Defender): `kawaro-0.4.1-windows-x64.msi` is blocked with "Virus detected". Protection history:

- Detected: `Program:Win32/Contebrew.A!ml` (a machine learning generic detection, "This program has potentially unwanted behavior").
- Affected file: `kawarod.exe` inside the MSI cabinet (`kawaro.cab->DaemonExe`).
- Not a known malware family: `!ml` means a cloud model scored a new, unsigned program. Kawaro is a local file converter, GPLv3, source at https://github.com/M7MMAD-OMAR/kawaro.

Hashes (SHA-256):

- `kawarod.exe` 0.4.1: `27a701dee42fa1aa4d297290a1c935b5ed44a13ed78d51b13799b97c4b7498c5`
- `kawaro.exe` 0.4.1: `773f7cca3bf0c6402e2491fc20f4cd185105404c3d7abf8c2584dbbfa526c5e6`
- MSI 0.4.1: `f0062d2542ba48b8789040a24c701e7542aa85f3917f0aa575c42099b655ff68`
- `kawarod.exe` 0.4.0 (for comparison): `ee2a05295858f583f595e3e337eed94c76a004c37a419141e4cc7d86d1d49473`

## Submission text (Microsoft Security Intelligence, "Software developer", file submission)

Product: Kawaro, open source (GPLv3) local file converter. Release: https://github.com/M7MMAD-OMAR/kawaro/releases/tag/v0.4.1. Detection: Program:Win32/Contebrew.A!ml on kawarod.exe (SHA-256 above), inside kawaro-0.4.1-windows-x64.msi. I believe this is a false positive: the program converts files on the user's machine, has no telemetry and no remote control. Behaviours that may look unusual and are intended: it runs a background service started at logon (HKCU Run key), adds an Explorer right-click verb (HKCU), stops its own previous daemon during an update, and an update downloads a signed MSI from GitHub and runs msiexec after the user asks for it. Source for each is in the repository. Please remove the detection.

## Owner steps (needs a Microsoft account)

1. Open https://www.microsoft.com/wdsi/filesubmission and sign in.
2. Choose "Software developer", then upload the MSI and `kawarod.exe` from the release, paste the submission text, send.
3. Repeat for every release until the signature reputation builds; keep the submission ID in this file.
