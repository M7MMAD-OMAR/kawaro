[English](release-notes-0.5.1.md) | [العربية](release-notes-0.5.1.ar.md)

# Kawaro 0.5.1

2026-11-22

## Release scope

Kawaro 0.5.1 brings together accepted work S-117 through S-151 except the still-draft S-138. These notes describe the changes and their validation boundaries, not verification of published packages, signatures or channels.

## Highlights

- **See the result before you run.** The customization panel previews the transformed image, not only the source, through one newest-wins worker, and can show the encoded size on request. Combined actions such as images to PDF show no preview.
- **Named presets.** Save, apply, rename and delete your own option sets per image action. A preset whose action changed asks for review instead of applying silently.
- **Results and safe retry.** A Results page in Settings, opened from the tray, and `kawaro jobs`, `results`, `retry` and `cancel` show each file's outcome. Retry runs only the failed files, once, and refuses when an input changed. `kawaro cancel INSTANCE:JOB` never cancels a job of an earlier daemon run.
- **Why an action is missing.** `kawaro doctor`, the hive and MCP give each unavailable action a reason and install help. A tool installed while Kawaro runs is picked up after a reload.
- **Text from images and searchable PDFs.** `image.to-text` uses the system Tesseract for Arabic and English. `pdf.make-searchable` uses a system OCRmyPDF 14.4 or newer, skips pages that already have text and refuses signed or encrypted PDFs. Both are optional system tools, nothing is downloaded, and no Arabic OCR quality claim is made yet.
- **Every action reachable.** When more actions match than the outer ring holds, a More cell pages through them, also with PageUp and PageDown.
- **Review and security fixes.** `kawaro results` escapes control characters in file names, the service socket stays in a private per-user directory, and the output copy between disks never follows a symlink.
- **Typed options and customization.** Actions share validated per-run options across the hive, CLI and MCP. Customize or Shift+Enter opens readable native Material fields, presets, Reset and Run, with Arabic RTL and light/dark controls. Invalid input blocks Run. Back or the first Escape returns to the action hive without losing the customization session; explicit Run submits one job. CLI scripts can use `kawaro run ACTION --set KEY=VALUE` and `kawaro actions --id ACTION --schema`.
- **Calculated target.** Next to the preview the panel shows the target dimensions, ratio and percentage. No fresh under-150 ms show or preview measurement is claimed.
- **Local MCP, by choice.** `kawaro mcp` uses stdio, not a network listener. It is disabled and read-only by default. Enable it explicitly, restrict `allowed_roots`, and set `read_only = false` only to allow conversions. Format discovery and planning need no writes. Audit logging is configurable, including its path and rotation. The MCP host has its own privacy policy; choose one you trust with tool results.
- **More PDF and image operations.** Extract, delete, reorder and split PDF pages; set or clear PDF metadata; control images-to-PDF page size, margins, orientation and DPI. PDF text extraction requires system Poppler; qpdf cleanup, linearization and checks require system qpdf. Ghostscript compression has a DPI option. Image tools add fit/fill/exact resizing, anchored cropping, optional JPEG metadata retention and target-byte compression that may reduce dimensions and reports an unattainable target honestly. PDF compression skips an already optimized result instead of creating a redundant copy. Optional-engine actions require their engine to be available; qpdf is not downloaded by Kawaro. Page extraction/reordering drops outlines, forms and open actions; PDF text extraction is not OCR and Arabic/RTL extraction has limits.
- **Drop first, or keep your preference.** DropFirst is the default: drop files on the drop surface, then choose an action in the normal hive. Choose while dragging remains an explicit setting, and an existing saved preference is preserved. The shortcut can also open files already copied to the clipboard. The corrected welcome flow teaches DropFirst. Conversion writes new files beside the originals, never changes or moves them.
- **One session, one submission.** DropFirst closes its drop surface before handing off once; choose-during-drag submits once without reopening. Ended or replaced sessions reject late callbacks. The Hyprland binding fix cancels with Escape while Alt stays held and delivers release after another key, without waiting for the watchdog. First-run language selection and settings focus, Escape and single-window handling are also corrected.

## After updating

On Hyprland, `kawaro doctor --repair` updates only the generated Kawaro-owned binding file. That file must be sourced by your active configuration for the Escape/release fix to take effect. Back up custom integrations first. Hand-maintained legacy Kawaro binding sections need equivalent S-140 updates manually; the repair command does not edit foreign configuration. Do not duplicate bindings or overwrite unrelated shortcuts. Updating binaries alone does not rewrite custom bindings.

## Known limits

- **Real GNOME and KDE focus/activation remains the top platform risk**, tracked in [issue #1](https://github.com/M7MMAD-OMAR/kawaro/issues/1). Private compositor checks do not close it or establish all-platform support. Bare Alt is not a supported portal shortcut there; use the supported chord path such as Super+Z.
- S-138 remains DRAFT: near/distant whole-output coverage and final position are unverified. This release does not claim to complete that work.
- S-142 to S-151 were checked by fmt, clippy, the workspace tests and the Windows cross stages, not by a compositor or a real desktop. After a drop on More the layer hive asks for the keyboard late, which a compositor may ignore. OCRmyPDF admits 250 pages, but a job stops after 10 minutes. Without the daemon, Windows shows no source picture in the panel.
- S-139/S-140 have accepted private native evidence, not universal desktop coverage. Legacy Hyprlang runtime, separate compositor Closed-event injection, IME and physical hotplug remain unverified.
- X11 evidence remains a private virtual X session with xfwm4, not a real GNOME/KDE X11 desktop. Real file-manager drag sources and Shift move-request behavior remain unchecked. The new customization UI runtime on Windows and physical Windows hardware are not verified; Windows remains self-signed and SmartScreen may warn. macOS is unsupported.

## Privacy and validation

Kawaro processes source files locally: no processing uploads, accounts or telemetry. Update checks and update downloads are separate network activity; optional engine installation may also download software, not your source files.

These notes do not certify release binaries, package installation, checksum or update-manifest signatures, or distribution-channel publication. Historical VM and private compositor results do not establish new platform acceptance.
