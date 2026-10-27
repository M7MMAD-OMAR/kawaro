[English](windows-smartscreen.md) | [العربية](windows-smartscreen.ar.md)

# Kawaro and the SmartScreen warning

Windows screens downloaded programs with SmartScreen. This page says what the
dialog shows, what to click, and how to check that your download is the file
we published. Nothing here asks you to turn SmartScreen off.

## What the dialog shows

Before the build is signed, the dialog reads "Windows protected your PC" and
names an unknown publisher: "Unknown publisher". After the build is signed
with the release certificate, the dialog shows the publisher name from the
certificate instead. SmartScreen weighs two things: the publisher behind the
file (is the file signed, and does the certificate have a history) and the
file itself (has this exact file been downloaded without trouble). A new
certificate has no history yet, so a warning can still appear at first even
on a signed file, and it fades as downloads accumulate without reports.

## What to click

1. Verify the download first (the next section).
2. If the values match, click "More info".
3. Click "Run anyway" if you accept the file.

These are the only two clicks needed. Never disable SmartScreen for Kawaro.

## Verify the download against SHA256SUMS

Every release publishes SHA256SUMS next to the files. Put it in the same
folder as the file you downloaded.

- Windows: `certutil -hashfile kawaro-<version>-windows-x64.msi SHA256`
- Linux: `sha256sum -c SHA256SUMS --ignore-missing`

Every value must match. If a value differs, do not run the file.

## EV certificates

An EV certificate does not remove the warning: under Microsoft's current
guidance EV no longer bypasses SmartScreen. Kawaro 0.4.0 uses a self-signed test certificate. A trusted signing certificate is not active yet, and no date can be promised for a warning to disappear.

## For the maintainer: one submission per release

For every release, after the artifacts are built and signed:

1. Submit the built MSI to Microsoft's malware analysis portal:
   https://www.microsoft.com/wdsi/filesubmission
2. Keep the submission confirmation.
3. Expect the warning to fade as downloads accumulate on the same
   certificate. Keep one signing identity across releases, never modify a
   file after signing it, and keep every signature timestamped.
