# Engines and their dependencies

Every library an engine pulls in, with its license and the reason it is
here. Kawaro is GPL-3.0-or-later; each license below is permissive and
compatible with it (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, Zlib,
BSL-1.0, Unicode-3.0). Check this file before adding a dependency.

## Image engine (`crates/engines/src/image.rs`, story S-004)

Ops, inputs and parameters are in `docs/architecture.md`, section
"Image engine operations". Behaviour decided while building it:

- Output format comes from the extension of `task.output` (png, jpg, jpeg,
  webp, avif, gif, bmp, tif, tiff, ico). Any other extension is a
  `BadParam` on `output`. `compress` and `strip` keep the format because the
  queue keeps the extension.
- `quality` 1 to 100, default 90 (75 for `compress`). It drives JPEG, WebP
  and AVIF. WebP at quality 100 is lossless.
- PNG is written at balanced zlib compression; `compress` uses the best
  level with adaptive filters.
- `resize`: `scale` is a percent from 1 to 1000; `max_side` never enlarges.
- ICO holds 16, 32, 48, 64, 128 and 256 px frames up to the longest side
  of the source (16 at least), on a transparent square.
- `to-pdf`: one page per image, 1 pt per pixel. Upright 8 bit gray or RGB
  JPEGs are copied into the PDF untouched (DCTDecode); everything else is
  stored losslessly with Flate, alpha composited onto white.
- No encoder writes EXIF, XMP or ICC data, so every re-encode strips
  metadata. ICC profiles are therefore dropped too (known limitation).

Not supported in this build:

- AVIF input. `image` decodes AVIF only through `dav1d`, a C library that
  must be installed on the system. We refuse AVIF input with an `Input`
  error that says so, instead of depending on a system library.
- HEIC, HEIF and JXL input (no permissive pure Rust decoders yet).
- Animation: GIF and WebP inputs use their first frame.

### image 0.25.10

- License: MIT OR Apache-2.0.
- Why: decoding and encoding for PNG, JPEG, GIF, BMP, TIFF, ICO, lossless
  WebP and AVIF, EXIF orientation (`ImageDecoder::orientation`), Lanczos3
  resizing, rotation and flips. Pure Rust.
- Features: default features off; enabled `png jpeg gif bmp tiff ico webp
  avif rayon`. The `nasm` feature stays off, so rav1e builds without
  assembly and NASM is not needed. `avif-native` (dav1d) stays off.
- Notable transitive crates: `png`, `zune-jpeg` (Zlib), `gif`, `tiff`
  (MIT), `image-webp`, `ravif` (BSD-3-Clause), `rav1e` (BSD-2-Clause),
  `rayon`, `moxcms`. All permissive.

### ravif 0.13 and rav1e 0.8 (through `image`'s `avif` feature)

- License: BSD-3-Clause (ravif), BSD-2-Clause (rav1e).
- Why: AVIF encoding in pure Rust. Encoder speed is 7 (scale 1 to 10), so
  a photo encodes in seconds rather than minutes.

### webp 0.3.1 and libwebp-sys 0.9

- License: MIT OR Apache-2.0 (webp), MIT (libwebp-sys). The bundled
  libwebp C sources are BSD-3-Clause.
- Why: `image` writes lossless WebP only. Lossy WebP needs libwebp, which
  `libwebp-sys` compiles from bundled sources with the system C compiler
  (no system libwebp needed). Default features (`img`, which pulls a second
  copy of `image`) are off.

### pdf-writer 0.15.0

- License: MIT OR Apache-2.0.
- Why: small pure Rust PDF writer (from the Typst project). Lets us embed
  JPEG bytes as DCTDecode streams without re-encoding. Chosen over
  `printpdf` (larger, pulls fonts and more) and `lopdf` (a reader and
  editor; we only write).

### miniz_oxide 0.8

- License: MIT OR Zlib OR Apache-2.0.
- Why: zlib compression of raw pixel streams inside the PDF. Already in
  the tree through `png`, so it adds no new code.

## ffmpeg

- **Engine id:** `ffmpeg` (`crates/engines/src/ffmpeg.rs`, story `backlog/S-008-ffmpeg-engine.md`).
- **Backend:** the system `ffmpeg` and `ffprobe` binaries, run as child processes. Nothing is linked, so the FFmpeg build's license (LGPL 2.1 or GPL 2/3, depending on how the distribution configured it) does not combine with Kawaro's GPLv3 code. Kawaro ships no FFmpeg binary in v0.2.
- **Availability:** `available()` runs `ffmpeg -version`, `ffprobe -version` and `ffmpeg -hide_banner -encoders` once per engine instance and caches the result. The engine is available when both binaries run. Encoders are checked per operation at run time (see below), because the `Engine` trait has no per-action availability yet.
- **Actions:** `actions/video.toml` and `actions/audio.toml`.

### Operations

The target format always comes from the extension of `task.output`, and the output is written with an explicit `-f` muxer because `task.output` is a hidden temporary name.

| op | input | targets | encoding |
| --- | --- | --- | --- |
| `convert-video` | video | mp4, m4v, mov | H.264 (`libx264 -preset medium -crf <crf, 23>`, yuv420p, even size) + AAC 160k, `-movflags +faststart` |
| | | webm | VP9 (`libvpx-vp9 -crf <crf, 32> -b:v 0 -row-mt 1 -deadline good -cpu-used 2`) + Opus 128k |
| | | mkv | `-c copy` when every audio and video stream uses a codec Matroska accepts (h264, hevc, av1, vp8, vp9, mpeg4, mpeg1/2, theora, aac, mp3, mp2, opus, vorbis, flac, ac3, eac3, dts, truehd, alac, pcm), otherwise H.264 + AAC |
| `to-gif` | video | gif | one filtergraph: `fps=<fps, 12>,scale=<w>:-1:flags=lanczos,split,palettegen,paletteuse`, `-loop 0`; `w` is `width` (480) but never more than the source width |
| `extract-audio` | video | mp3, ogg/oga, opus, m4a, flac, wav | first audio stream only, no video |
| `convert-audio` | audio | same as above | first audio stream only; cover art is dropped |
| `compress-video` | video | same container as the input | H.264 `-crf <crf, 28>` + AAC 128k, scaled to `max_height` (1080) only when the source is taller; webm uses VP9 `-crf <webm_crf, 36>` + Opus 96k because webm cannot hold H.264 |

Audio encoders: mp3 `libmp3lame -q:a 2`, ogg `libvorbis -q:a 5`, opus `libopus -b:a 128k`, m4a `aac -b:a 192k`, flac `flac`, wav `pcm_s16le`.

Parameters (from `[action.params]`, all optional): `crf` 0 to 63, `webm_crf` 0 to 63, `fps` 1 to 60, `width` 16 to 4096, `max_height` 16 to 8640. Out of range values return `EngineError::BadParam`.

`compress-video` supports mp4, m4v, mov, mkv, avi, flv, ts, 3gp and webm. wmv, mpeg and mpg return `UnsupportedOp`, and the action hides itself for them with `skip_ext`.

### Missing encoders

| missing | effect |
| --- | --- |
| `libx264` | falls back to `libopenh264` at a fixed bitrate (4M for convert, 2M for compress) |
| `libx264` and `libopenh264` | H.264 targets return `EngineError::Unavailable("ffmpeg has no H.264 (libx264 or libopenh264) encoder")` |
| `libvpx-vp9` | webm targets return `Unavailable` naming `libvpx-vp9` |
| `libopus` | webm and opus targets return `Unavailable` naming `libopus` |
| `libvorbis` | ogg is written with Opus inside Ogg instead; if `libopus` is also missing, `Unavailable` |
| `libmp3lame` | mp3 targets return `Unavailable` naming `libmp3lame` |

AAC, FLAC, PCM and GIF use FFmpeg's built-in encoders, present in every build. Fedora's `ffmpeg-free` lacks `libx264` but has `libopenh264`; RPM Fusion's `ffmpeg` has all of them.

### Process, progress and cancel

- Command: `ffmpeg -hide_banner -nostdin -nostats -loglevel error -progress pipe:1 -y -i file:<input> <codec args> -f <muxer> file:<output>`, with stdin closed. Paths go through the `file:` protocol (relative paths also get `./`), so a name starting with `-` or containing `:` is never read as an option or a protocol.
- Before running, `ffprobe -show_entries format=duration:stream=codec_type,codec_name,width,height -of flat` reads the duration and stream layout. A file ffprobe cannot read still goes to ffmpeg, which reports the real error.
- Progress is `out_time_us / duration`, reported only when it grows, capped below 1.0 while running, and exactly 1.0 after ffmpeg exits with status 0. Without a known duration, only the final 1.0 is reported.
- The run loop wakes every 50 ms. When the cancel token fires, ffmpeg is killed and waited for, and the engine returns `EngineError::Cancelled`. Removing the partial `task.output` is left to the queue.
- A non-zero exit returns `EngineError::Failed` with the exit status and the last 20 non-empty stderr lines.

### Limitations

- Subtitles and data streams are dropped by every operation.
- `convert-video` and `compress-video` keep the first video stream and every audio stream; extract and convert audio keep the first audio stream.
- No hardware encoding (VAAPI, NVENC) in v0.2.
- The `-progress` output of `to-gif` jumps to the end, because `palettegen` needs the whole clip before the first frame is written.

## office

Engine id `office` (`crates/engines/src/office.rs`, story S-009). Runs
LibreOffice headless as a child process.

| op | inputs | output (from `task.output` extension) |
| --- | --- | --- |
| `to-pdf` | doc docx odt rtf txt md xls xlsx ods csv ppt pptx odp | pdf |
| `convert-doc` | doc docx odt rtf txt md | docx or odt |
| `convert-sheet` | xls xlsx ods csv | xlsx or ods |
| `convert-slides` | ppt pptx odp | pptx or odp |

Params: `timeout_s` (seconds, default 180, any positive number).

Behaviour:

- Program: `soffice`, then `libreoffice`, searched once per process in `PATH`
  (on Windows also the install folder and the registry, see "Finding the
  programs" below).
  `available()` is false when neither exists. Markdown is supported: the
  LibreOffice 26.2 tested here imports `.md` as a real Writer document.
- Each run creates a private work directory in the system temp dir holding a
  copy of the input (`in.<ext>`), a throwaway user profile passed with
  `-env:UserInstallation=file:///...`, and `out/`. The copy means LibreOffice
  never writes `.~lock` files next to the original and odd file names cannot
  be read as options. The private profile means a run never collides with the
  user's open LibreOffice and never touches their profile. The whole
  directory is deleted after success, failure, cancel and timeout.
- Office Open XML targets use explicit filters (`MS Word 2007 XML`,
  `Calc MS Excel 2007 XML`, `Impress MS PowerPoint 2007 XML`).
- The exit code is not trusted: LibreOffice can exit 0 after a load failure.
  The task fails unless `out/in.<target>` exists and is not empty.
- The result is renamed to `task.output`, or copied when the temp dir is on
  another file system.
- Progress: 0.0 at start and 1.0 at the end; LibreOffice reports nothing in
  between.
- An input of the wrong family (a spreadsheet given to `convert-doc`) is an
  input error. The action schema has no "only these extensions" field, so
  `actions/documents.toml` hides conversions with `skip_ext` listing the other
  families; a mixed selection can still offer one, and the engine then fails
  the files that do not fit.

### Child processes (shared with ghostscript)

`run_child` in `office.rs` starts the tool in its own process group
(`process_group(0)`), streams stdout lines to the caller, keeps the last
4 KiB of stderr for error messages, and polls every 25 ms. Cancel and timeout
send `SIGKILL` to the whole group, which takes the LibreOffice launcher,
`oosplash` and `soffice.bin` down together. The group is also killed after a
normal exit so nothing outlives the task. On Windows the fallback is
`Child::kill` on the direct child only.

## pdf

`available()` is per engine, not per op, so PDF work is two engine ids, both in
`crates/engines/src/pdf.rs` (story S-010):

| engine | op | params | inputs | output |
| --- | --- | --- | --- | --- |
| `pdf` | `merge` | none | n, in selection order | pdf |
| `pdf` | `rotate` | `degrees` 90 (default), 180, 270 | 1 | pdf |
| `pdf` | `split` | none | 1 | zip of `page-001.pdf`... |
| `ghostscript` | `compress` | `preset` screen, ebook (default), printer; `timeout_s` (300) | 1 | pdf |
| `ghostscript` | `to-images` | `dpi` 36..600 (default 150); `timeout_s` (300) | 1 | zip of `page-001.png`... |

`pdf` is pure Rust on lopdf and always available. `ghostscript` runs `gs`
(`gswin64c`/`gswin32c` on Windows) and is hidden when it is missing.

Decisions:

- **split and to-images write one ZIP.** The engine contract writes exactly
  one file per task to a temp path that the queue renames atomically. A folder
  of pages would need a second output kind in core (a directory rename, name
  collision rules for a folder, partial cleanup), which is a contract change
  needing an ADR. A ZIP fits the existing contract and keeps the atomic
  rename. Entries are stored, not deflated: PDF pages and PNG are already
  compressed. Names are zero padded to at least 3 digits and wider for 1000
  pages or more, so they sort in page order.
- **compress keeps the original when it does not shrink.** If Ghostscript's
  result is not strictly smaller, the input is copied to `task.output` and the
  task still succeeds, so the user never gets a larger "compressed" file.
- **Inherited attributes are flattened.** Before merge, rotate and split,
  `MediaBox`, `CropBox`, `Resources` and `Rotate` inherited from the page tree
  are copied onto each page, so pages keep their size and fonts in the new
  tree and rotation adds to the effective value.
- **rotate** adds `degrees` to each page's effective `/Rotate`, modulo 360.
- **merge** drops the inputs' catalogs, outlines and page tree nodes and
  builds one new page tree; unreachable objects are pruned.
- **split** builds each page's PDF from a copy of the document with the page
  tree cut to that page and `Outlines`, `StructTreeRoot` and `OpenAction`
  removed, then prunes unreachable objects.
- Ghostscript always runs with `-dSAFER -dBATCH -dNOPAUSE`, an absolute input
  path (never read as an option), and `%` escaped in output paths. Progress
  comes from its `Processing pages 1 through N.` and `Page n` lines.

Limitations:

- Encrypted PDFs that lopdf cannot open, or that stay encrypted after
  loading, are rejected as input errors; there is no password prompt.
- split copies the whole document once per page in memory; a very large PDF
  with many pages is slow. A page whose links point at other pages keeps
  those pages as extra objects in its file (not shown as pages).
- merge does not carry bookmarks, forms (AcroForm) or tagged structure.

### Libraries and tools (office, pdf)

| name | used by | license | notes |
| --- | --- | --- | --- |
| LibreOffice | office | MPL-2.0 | external program, not linked |
| Ghostscript | ghostscript | AGPL-3.0 | external program, not linked |
| lopdf 0.45 | pdf | MIT | `default-features = false`; declares rust-version 1.88, above the workspace's 1.85 |
| zip 9 | pdf, ghostscript | MIT | `default-features = false` (stored only); tests add `deflate-flate2` to read LibreOffice archives |
| libc 0.2 | office, ghostscript | MIT OR Apache-2.0 | unix only, for `killpg` |
| tempfile 3 | office, ghostscript | MIT OR Apache-2.0 | private work directories |
| windows-sys 0.61.2 | core (IPC transport), office, ghostscript, ffmpeg | MIT OR Apache-2.0 | Windows only, for the named pipe (S-040b), the job object (S-043a) and the registry lookup of the engines (S-043b); already in Cargo.lock through other crates |

lopdf's own dependencies (flate2, aes, cbc, ecb, md-5, sha2, nom, indexmap,
weezl, rangemap, stringprep, encoding_rs, brotli-decompressor) are MIT,
Apache-2.0 or BSD-3-Clause, all compatible with GPL-3.0-or-later.

## Finding the programs (S-043b)

`crates/engines/src/locate.rs` looks each external program up (once per process for LibreOffice and Ghostscript, once per engine for ffmpeg), in this
order, and takes the first executable file: `PATH`, then on Windows the usual
install folder, then on Windows the folder the installer wrote in the
registry (`HKEY_LOCAL_MACHINE`, read by `locate/windows.rs`). On Windows a
file counts only with an `.exe` or `.com` name. Kawaro never downloads an
engine; a missing one makes its actions unavailable, and running one returns
a message with the program's name and its download page.

| program | Linux (`PATH` only) | Windows names, in order | Windows install folder | Windows registry | download page |
| --- | --- | --- | --- | --- | --- |
| LibreOffice | `soffice`, then `libreoffice` | `soffice.com`, `soffice.exe` | `%ProgramFiles%\LibreOffice\program` | default value of `SOFTWARE\LibreOffice\UNO\InstallPath` (the folder) | https://www.libreoffice.org/download/ |
| Ghostscript | `gs` | `gswin64c.exe`, `gswin32c.exe` | `%ProgramFiles%\gs\<newest gs*>\bin`, newest by version number | newest version subkey of `SOFTWARE\GPL Ghostscript`, its `GS_DLL` value's folder | https://ghostscript.com/releases/ |
| FFmpeg | `ffmpeg`, `ffprobe` | `ffmpeg.exe`, `ffprobe.exe` | `<folder of kawaro.exe>\ffmpeg\bin` | none | https://ffmpeg.org/download.html |

Within one folder the names keep their order. The two registry keys are
assumptions from ADR 0005, checked on a real Windows install in S-052. On
Windows `ffmpeg` and `ffprobe` start with `CREATE_NO_WINDOW`, so no console
window flashes. Every engine child, FFmpeg included, runs in one
`ProcessTree` (`crates/engines/src/process.rs`, S-030): its own process
group on Unix, a kill-on-close job object on Windows, so cancelling, the
timeout or the service dying ends the whole tree.

## Windows build tools (S-044)

Used on the Fedora build host only, to cross build and check the Windows
programs; nothing from them is shipped. `scripts/windows/check-dlls.sh`
refuses an executable that imports a mingw runtime DLL (`lib*.dll`), so no
mingw library ends up beside `kawaro.exe`.

| tool | version | license | used by | why |
| --- | --- | --- | --- | --- |
| mingw64-gcc | 16.2.1 (Fedora 44, installed) | GPL-3.0-or-later with the GCC runtime exception (runtime parts LGPL and others) | `scripts/windows/build.sh`, `check.sh` | linker for `x86_64-pc-windows-gnu` |
| mingw64-binutils | 2.46.0 (Fedora 44, installed) | GPL-3.0-or-later (some parts GPL-2.0-or-later, LGPL and BSD-3-Clause) | `scripts/windows/check-dlls.sh` | its `x86_64-w64-mingw32-objdump` reads the DLL imports |
| mingw-w64 | Ubuntu 24.04 package, version of the runner image | GPL-3.0-or-later with the GCC runtime exception, binutils GPL-3.0-or-later | `windows-cross` job in `.github/workflows/ci.yml` | the Ubuntu package that gives the CI runner the same mingw gcc and `x86_64-w64-mingw32-objdump` |
| genisoimage | 1.1.11 (Fedora 44, installed) | GPL-2.0-only | `scripts/windows/make-iso.sh` (S-044b) | writes the ISO that carries the test programs to the Windows test machine |
| msitools | 0.106.58 (Fedora 44, installed) | LGPL-2.1-or-later (library), GPL-2.0-or-later (tools `wixl`, `msiinfo`), MS-RL (the bundled WiX schema data) | `scripts/windows/make-msi.sh` (S-051) and the S-051 tests | `wixl` builds the per-user MSI from `installer/windows/kawaro.wxs`, `msiinfo` reads its tables; build tool only, nothing from it is shipped |
| msitools (CI) | Ubuntu 22.04 package, version of the runner image | same as above | `windows` job in `packages.yml`, `ci.yml` and `scripts/release/dry-run.sh` (S-055) | builds the MSI on the runner; installed with apt, the job log shows the version; build tool only |
| minisign (CI) | Ubuntu package, version of the runner image (24.04 on `ci.yml` and on `update-manifest`; 22.04 has no minisign package, S-073) | ISC | `update-manifest` job, `scripts/sign-update-manifest.sh`, `scripts/release/dry-run.sh` and `tests/release` (S-053c, S-055) | signs the update manifest and verifies the signature before it is published; the dry run and the tests use a throwaway key; build tool only |
| zip | Info-ZIP 3.0 (Fedora 44, installed) | Info-ZIP | `scripts/windows/make-zip.sh` (S-051) | writes the portable zip with fixed timestamps; build tool only |

## Application dependencies (daemon, CLI, UI)

Not engines, recorded here because CLAUDE.md asks for every new library.

| crate | version | license | used by | why |
| --- | --- | --- | --- | --- |
| slint | 1.18.1 (pinned) | GPL-3.0-only OR Slint royalty-free OR Slint software licence; we use it under GPL-3.0 | ui, daemon | hive and settings windows, software renderer (ADR 0001) |
| Slint Material components | vendored from Slint v1.18.1 in `crates/ui/material/` | MIT | ui | settings window components, shadows removed (ADR 0002) |
| ksni | 0.3.6 | Unlicense | daemon (Linux only, `[target.'cfg(target_os = "linux")'.dependencies]` since S-045) | tray icon over StatusNotifierItem, blocking API without tokio |
| notify-rust | 4.18.2 | MIT OR Apache-2.0 | daemon | end-of-job desktop notification with an "Open folder" action |
| clap | 4.6.7 | MIT OR Apache-2.0 | cli | argument parsing |
| IBM Plex Sans Arabic | system copy, embedded | OFL-1.1 | ui, site | the only interface font, so rendering never depends on system fonts |
| wayland-client | 0.31.15 | MIT | ui (Linux) | drop receiver: a second connection on winit's `wl_display` (S-014b, ADR 0003); already in the tree through winit |
| wayland-backend | 0.3.17, `client_system` | MIT | ui (Linux) | `Backend::from_foreign_display` for that connection; already in the tree through winit |
| rustix | 1.1.5, `event`, `pipe` and `fs` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | ui (Linux) | pipe and poll to read the dragged `text/uri-list` with a timeout, `memfd_create` for the overlay's shm buffers (S-018b); already in the tree through wayland-backend |
| i-slint-backend-winit | 1.18.1 (pinned, `=1.18.1`) | GPL-3.0-only OR Slint royalty-free OR Slint software licence; we use it under GPL-3.0 | ui (Linux) | the winit platform wrapped by the thin layer-shell platform (S-018b, ADR 0004); already in the tree through slint |
| i-slint-core | 1.18.1 (pinned, `=1.18.1`) | GPL-3.0-only OR Slint royalty-free OR Slint software licence; we use it under GPL-3.0 | ui (Linux) | `InternalToken` and `SlintContextWeak` for the forwarded `Platform` methods (S-018b); already in the tree through slint |
| wayland-protocols | 0.32.13, `client` and `staging` | MIT | ui (Linux) | viewporter, single pixel buffer, fractional scale and xdg-activation for the hive overlay (S-018b); already in the tree through winit |
| wayland-protocols-wlr | 0.3.12, `client` | MIT | ui (Linux) | `zwlr_layer_shell_v1` for the hive overlay (S-018b); already in the tree through winit |
| zbus | 5.19.0, `async-io`, `blocking-api` | MIT | ui (Linux) | reads `org.a11y.Status.ScreenReaderEnabled` and its `PropertiesChanged`, and whether `org.gnome.Orca.Service` has an owner, so the hive is a window while a screen reader runs (S-018c, S-019a); already in the tree through accesskit and the tray |
| ashpd | 0.13.13 (pinned, `=0.13.13`), `default-features = false`, `global_shortcuts` and `async-io` | MIT | daemon (Linux) | the GlobalShortcuts portal client (S-014e1): `Registry.Register`, `CreateSession`, `BindShortcuts` and the `Activated` and `Deactivated` signals on the zbus of the tree, without tokio; the only package it adds to `Cargo.lock` |
| futures-util | 0.3.34, `std` | MIT OR Apache-2.0 | daemon (Linux) | `StreamExt::next` over the portal's signal stream (S-014e1); already in the tree through zbus and ashpd |
| md-5 | 0.11.0 | MIT OR Apache-2.0 | ui | centre cell picture (S-018g): MD5 of the file URI names the cached thumbnail; already in the tree through another dependency |
| png | 0.17.16 | MIT OR Apache-2.0 | ui | centre cell picture (S-018g): reads the `Thumb::*` text chunks of a cached thumbnail; was a dev-dependency, already in the tree |
| minisign-verify | 0.3.0 | MIT (checked with `cargo info`, 2026-10-24) | core (S-053a) | checks the minisign signature of the update manifest (ADR 0006); no dependencies, so it adds exactly one package to `Cargo.lock`; chosen over the `ed25519-dalek` fallback (which would add six packages and a hand written parser) because it is MIT, tiny and reads the minisign file formats itself |
| sha2 | 0.11.1 | MIT OR Apache-2.0 | core (S-053b) | SHA-256 of a downloaded update package, streamed in 64 KiB reads (ADR 0006); already in the tree, now a direct dependency of core, so no new package |
| semver | 1.0.28 | MIT OR Apache-2.0 | core (S-053a) | version precedence for the update rules; already in the tree, now a direct dependency of core |
| winresource | 0.1.34 | MIT (checked in its Cargo.toml, 2026-10-27) | cli, daemon (build dependency, S-085) | `build.rs` embeds `design/brand/kawaro.ico` and the version resource (ProductName and FileDescription `Kawaro \| كوارو`) in `kawaro.exe` and `kawarod.exe`, for a Windows target only; it writes `#pragma code_page(65001)` into its rc file, so mingw `windres` keeps the Arabic name (S-085 D5), and links the resource object directly; nothing of it is shipped. Its dependencies `toml` 1.1.8 (MIT OR Apache-2.0) and `version_check` 0.9.5 (MIT OR Apache-2.0) were already in the tree, so it adds exactly one package to `Cargo.lock` |
| windows-sys `Win32_System_Registry` | 0.61.2 | MIT OR Apache-2.0 | core (Windows only, S-085) | `RegSetKeyValueW` writes the bilingual AUMID DisplayName as REG_SZ UTF-16; a feature of a crate already in the tree, no new package |

## Website build tools

Not part of the app. Pagefind 1.5.2 (MIT) builds the docs search index at site build time through `npx` in `scripts/build-site.sh`; its Arabic word stemming is the reason it was chosen. The site is generated by `site/build.py` (Python standard library only).

## Release signing tools (S-083)

Not part of the app and never linked into any binary: build and release
tooling only. `scripts/windows/sign-local.sh` and the packages.yml windows
job run them inside `registry.fedoraproject.org/fedora:44`.

| Tool | Version (fedora:44) | Licence | Used for |
| --- | --- | --- | --- |
| osslsigncode | 2.12 | GPL-3.0-or-later | Authenticode signing, RFC 3161 timestamps and verification of kawaro.exe, kawarod.exe and the MSI |
| OpenSSL | 3.5.9 | Apache-2.0 | key and certificate generation for the throwaway test certificate, and `openssl ts` for the test timestamp authority |

## Release distribution tools (S-103)

Build and release tooling only, never linked into a binary. Run in CI on Ubuntu.

| Tool | Version | Licence | Used for |
| --- | --- | --- | --- |
| Komac | 2.17.1 | GPL-3.0 | `scripts/channels/winget.sh` opens the WinGet manifest update pull request from the maintainer's fork. Chosen over wingetcreate (MIT, but a .NET tool aimed at Windows) because it is a single Linux binary. It reads the token from the `GITHUB_TOKEN` environment variable, a classic token with `public_repo`. Running a GPL-3.0 tool in CI imposes nothing on Kawaro's own licence. |
