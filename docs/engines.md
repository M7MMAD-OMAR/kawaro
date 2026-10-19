# Engines and their dependencies

Every library an engine pulls in, with its license and the reason it is
here. SbarHive is GPL-3.0-or-later; each license below is permissive and
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
- **Backend:** the system `ffmpeg` and `ffprobe` binaries, run as child processes. Nothing is linked, so the FFmpeg build's license (LGPL 2.1 or GPL 2/3, depending on how the distribution configured it) does not combine with SbarHive's GPLv3 code. SbarHive ships no FFmpeg binary in v0.2.
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
