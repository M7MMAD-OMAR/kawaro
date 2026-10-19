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
