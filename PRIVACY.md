[English](PRIVACY.md) | [العربية](PRIVACY.ar.md)

# Privacy

Kawaro is a local file converter. Your files stay on your machine.

- **No uploads.** Kawaro opens no processing servers. Files are read and
  written locally, and the original file is never modified: the result is
  written to a temporary file and renamed beside the original.
- **No telemetry, no analytics, no accounts.** Nothing about your files,
  conversions or usage is collected or reported.
- **The only network access** is an optional update check, once a day. The app
  reads a signed manifest from GitHub Releases, verifies the signature, the
  size and the SHA-256 of a download before installing anything. GitHub sees
  your network address as with any download. Turn it off with
  `update_check = false` in `config.toml`.
- **Third party components.** The Slint UI library (MIT) is compiled in and
  makes no network calls. The optional engines (FFmpeg, LibreOffice,
  Ghostscript) are local processes taken from your system or installed at your
  request; Kawaro does not send them anything over the network.

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

The website's own notice: <https://kawaro.sbarah.com/en/privacy/>. Code
signing policy: [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md). Security
reports: [SECURITY.md](SECURITY.md).
