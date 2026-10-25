# softbuffer 0.4.8, patched for Kawaro

Source: crates.io `softbuffer` 0.4.8 (MIT OR Apache-2.0), unpacked as is.

One change, in `src/backends/wayland/buffer.rs`: the shared memory buffers
use `wl_shm::Format::Argb8888` instead of `Xrgb8888`. Slint's software
renderer already writes premultiplied alpha into the top byte; with the
`X` format the compositor ignored it, so the hive's transparent corners
showed as opaque black on every Wayland compositor. `Argb8888` is a format
every Wayland compositor must support.

Remove this copy (and the `[patch.crates-io]` entry in the workspace
`Cargo.toml`) once upstream softbuffer supports alpha on Wayland.
