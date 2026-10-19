# Engines

One section per engine. Each section is self-contained and owned by the
track that wrote it.

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
