//! FFmpeg engine (story S-008). Runs the system `ffmpeg` and `ffprobe`
//! binaries as child processes; nothing is linked, so the FFmpeg build's
//! license (LGPL or GPL) never mixes with ours.
//!
//! Every operation picks its target from the extension of `task.output` and
//! always writes with an explicit `-f` muxer, because `task.output` is a
//! hidden temporary name. The pure planning functions below build the whole
//! command line, so they are unit tested without spawning anything.

use crate::locate::{self, Program};
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::collections::{HashSet, VecDeque};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

/// Lines of ffmpeg stderr kept for [`EngineError::Failed`] messages.
pub const STDERR_TAIL: usize = 20;
/// How often the run loop wakes up to look at the cancel token.
const CANCEL_POLL: Duration = Duration::from_millis(50);
/// Where to get FFmpeg, for the message when it is missing.
const FFMPEG_URL: &str = "https://ffmpeg.org/download.html";

const OPS: &[(&str, MediaKind)] = &[
    ("convert-video", MediaKind::Video),
    ("to-gif", MediaKind::Video),
    ("extract-audio", MediaKind::Video),
    ("convert-audio", MediaKind::Audio),
    ("compress-video", MediaKind::Video),
];

/// Encoder names reported by `ffmpeg -encoders`.
pub type Encoders = HashSet<String>;

pub struct FfmpegEngine {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    /// `None` when either binary is missing. Probed once per engine.
    encoders: OnceLock<Option<Encoders>>,
}

impl Default for FfmpegEngine {
    /// Uses the paths from [`located`].
    fn default() -> Self {
        let (ffmpeg, ffprobe) = located();
        Self::with_binaries(ffmpeg.clone(), ffprobe.clone())
    }
}

/// `ffmpeg` and `ffprobe` as `locate::find` sees them, looked up once per
/// process. A program not found keeps its bare name, so the error names it.
pub(crate) fn located() -> &'static (PathBuf, PathBuf) {
    static LOCATED: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();
    LOCATED.get_or_init(|| {
        let bin = |program, name: &str| locate::find(program).unwrap_or_else(|| name.into());
        (
            bin(Program::Ffmpeg, "ffmpeg"),
            bin(Program::Ffprobe, "ffprobe"),
        )
    })
}

impl FfmpegEngine {
    /// Engine using explicit binaries, for tests and custom installs.
    pub fn with_binaries(ffmpeg: impl Into<PathBuf>, ffprobe: impl Into<PathBuf>) -> Self {
        Self {
            ffmpeg: ffmpeg.into(),
            ffprobe: ffprobe.into(),
            encoders: OnceLock::new(),
        }
    }

    fn encoders(&self) -> Option<&Encoders> {
        self.encoders
            .get_or_init(|| {
                if !runs_ok(&self.ffmpeg, &["-version"]) || !runs_ok(&self.ffprobe, &["-version"]) {
                    return None;
                }
                let out = command(&self.ffmpeg)
                    .args(["-hide_banner", "-encoders"])
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .ok()?;
                Some(parse_encoders(&String::from_utf8_lossy(&out.stdout)))
            })
            .as_ref()
    }

    fn probe(&self, input: &Path) -> MediaInfo {
        let out = command(&self.ffprobe)
            .args(["-v", "error", "-show_entries"])
            .arg("format=duration:stream=codec_type,codec_name,width,height")
            .args(["-of", "flat"])
            .arg(input_url(input))
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output();
        // A failed probe prints nothing on stdout, which parses to the default.
        out.map(|o| parse_probe(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default()
    }
}

/// Every ffmpeg and ffprobe command starts here, so none opens a console
/// window on Windows.
fn command(bin: &Path) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(crate::office::tree::CREATE_NO_WINDOW);
    }
    cmd
}

fn runs_ok(bin: &Path, args: &[&str]) -> bool {
    command(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

impl Engine for FfmpegEngine {
    fn id(&self) -> &'static str {
        "ffmpeg"
    }

    fn available(&self) -> bool {
        self.encoders().is_some()
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        OPS.iter().map(|(op, kind)| (*op, vec![*kind])).collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        if !OPS.iter().any(|(op, _)| *op == task.op) {
            return Err(EngineError::UnsupportedOp(task.op.clone()));
        }
        let params = Params::from_task(task)?;
        let [input] = task.inputs.as_slice() else {
            return Err(EngineError::BadParam {
                name: "inputs".into(),
                reason: format!("expected one file, got {}", task.inputs.len()),
            });
        };
        cancel.check()?;
        let encoders = self.encoders().ok_or_else(|| {
            EngineError::Unavailable(format!(
                "`{}` or `{}` not found; install FFmpeg from {FFMPEG_URL}",
                self.ffmpeg.display(),
                self.ffprobe.display()
            ))
        })?;
        if !input.is_file() {
            return Err(EngineError::Input {
                path: input.clone(),
                reason: "not a readable file".into(),
            });
        }
        let info = self.probe(input);
        let ext = sbarhive_core::media::extension(&task.output);
        let codec_args = plan(&task.op, &ext, &params, &info, encoders)?;
        let args = command_args(input, &task.output, &ext, codec_args)?;
        let child = command(&self.ffmpeg)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        supervise(child, info.duration, progress, cancel)
    }
}

/// Validated task parameters. Absent keys fall back to per-op defaults.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    pub crf: Option<i64>,
    pub webm_crf: Option<i64>,
    pub fps: Option<f64>,
    pub width: Option<i64>,
    pub max_height: Option<i64>,
}

impl Params {
    pub fn from_task(task: &Task) -> Result<Self, EngineError> {
        Ok(Self {
            crf: int_param(task, "crf", 0, 63)?,
            webm_crf: int_param(task, "webm_crf", 0, 63)?,
            fps: float_param(task, "fps", 1.0, 60.0)?,
            width: int_param(task, "width", 16, 4096)?,
            max_height: int_param(task, "max_height", 16, 8640)?,
        })
    }
}

fn bad(name: &str, reason: String) -> EngineError {
    EngineError::BadParam {
        name: name.into(),
        reason,
    }
}

fn int_param(task: &Task, name: &str, min: i64, max: i64) -> Result<Option<i64>, EngineError> {
    let Some(value) = task.params.get(name) else {
        return Ok(None);
    };
    match value.as_integer() {
        Some(v) if (min..=max).contains(&v) => Ok(Some(v)),
        _ => Err(bad(
            name,
            format!("expected an integer from {min} to {max}"),
        )),
    }
}

fn float_param(task: &Task, name: &str, min: f64, max: f64) -> Result<Option<f64>, EngineError> {
    if !task.params.contains_key(name) {
        return Ok(None);
    }
    match task.param_f64(name) {
        Some(v) if (min..=max).contains(&v) => Ok(Some(v)),
        _ => Err(bad(name, format!("expected a number from {min} to {max}"))),
    }
}

/// What ffprobe told us about the input. Everything is optional: a file
/// ffprobe cannot read still goes to ffmpeg, which reports the real error.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaInfo {
    pub duration: Option<f64>,
    pub streams: Vec<StreamInfo>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StreamInfo {
    pub kind: String,
    pub codec: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

impl MediaInfo {
    fn first_video(&self) -> Option<&StreamInfo> {
        self.streams.iter().find(|s| s.kind == "video")
    }
}

/// Parses `ffprobe -of flat` output.
pub fn parse_probe(text: &str) -> MediaInfo {
    let mut info = MediaInfo::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        if key == "format.duration" {
            info.duration = value.parse().ok().filter(|d: &f64| *d > 0.0);
            continue;
        }
        let Some(rest) = key.strip_prefix("streams.stream.") else {
            continue;
        };
        let Some((index, field)) = rest.split_once('.') else {
            continue;
        };
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };
        if info.streams.len() <= index {
            info.streams.resize(index + 1, StreamInfo::default());
        }
        let s = &mut info.streams[index];
        match field {
            "codec_type" => s.kind = value.to_string(),
            "codec_name" => s.codec = value.to_string(),
            "width" => s.width = value.parse().ok(),
            "height" => s.height = value.parse().ok(),
            _ => {}
        }
    }
    info
}

/// Encoder names from `ffmpeg -hide_banner -encoders`, listed after the
/// `------` separator line.
pub fn parse_encoders(text: &str) -> Encoders {
    text.lines()
        .skip_while(|l| l.trim() != "------")
        .skip(1)
        .filter_map(|l| l.split_whitespace().nth(1))
        .map(str::to_string)
        .collect()
}

/// The `-f` muxer for an output extension.
pub fn muxer_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "mp4" | "m4v" | "m4a" => "mp4",
        "mov" => "mov",
        "mkv" => "matroska",
        "webm" => "webm",
        "avi" => "avi",
        "flv" => "flv",
        "ts" => "mpegts",
        "3gp" => "3gp",
        "gif" => "gif",
        "mp3" => "mp3",
        "ogg" | "oga" => "ogg",
        "opus" => "opus",
        "flac" => "flac",
        "wav" => "wav",
        _ => return None,
    })
}

/// Input as an ffmpeg URL: the `file:` protocol, so a name starting with `-`
/// or holding `:` is never read as an option or another protocol.
pub fn input_url(path: &Path) -> OsString {
    let mut url = OsString::from("file:");
    if path.is_relative() {
        url.push("./");
    }
    url.push(path);
    url
}

fn unsupported(op: &str, ext: &str) -> EngineError {
    EngineError::UnsupportedOp(format!("{op} to .{ext}"))
}

fn missing(what: &str) -> EngineError {
    EngineError::Unavailable(format!("ffmpeg has no {what} encoder"))
}

fn has(enc: &Encoders, name: &str) -> bool {
    enc.contains(name)
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// H.264 video: libx264 with crf, else libopenh264 at a fixed bitrate.
fn h264(enc: &Encoders, crf: i64, fallback_bitrate: &str) -> Result<Vec<String>, EngineError> {
    if has(enc, "libx264") {
        let mut v = args(&["-c:v", "libx264", "-preset", "medium", "-crf"]);
        v.push(crf.to_string());
        v.extend(args(&["-pix_fmt", "yuv420p"]));
        Ok(v)
    } else if has(enc, "libopenh264") {
        let mut v = args(&["-c:v", "libopenh264", "-rc_mode", "bitrate", "-b:v"]);
        v.push(fallback_bitrate.to_string());
        v.extend(args(&["-pix_fmt", "yuv420p"]));
        Ok(v)
    } else {
        Err(missing("H.264 (libx264 or libopenh264)"))
    }
}

fn vp9(enc: &Encoders, crf: i64) -> Result<Vec<String>, EngineError> {
    if !has(enc, "libvpx-vp9") {
        return Err(missing("VP9 (libvpx-vp9)"));
    }
    let mut v = args(&["-c:v", "libvpx-vp9", "-crf"]);
    v.push(crf.to_string());
    v.extend(args(&[
        "-b:v",
        "0",
        "-row-mt",
        "1",
        "-deadline",
        "good",
        "-cpu-used",
        "2",
    ]));
    Ok(v)
}

fn opus(enc: &Encoders, bitrate: &str) -> Result<Vec<String>, EngineError> {
    if !has(enc, "libopus") {
        return Err(missing("Opus (libopus)"));
    }
    Ok(vec![
        "-c:a".into(),
        "libopus".into(),
        "-b:a".into(),
        bitrate.into(),
    ])
}

fn aac(bitrate: &str) -> Vec<String> {
    vec!["-c:a".into(), "aac".into(), "-b:a".into(), bitrate.into()]
}

/// Even dimensions, which yuv420p H.264 requires.
const EVEN_SIZE: &str = "scale=trunc(iw/2)*2:trunc(ih/2)*2";

/// Codecs Matroska accepts as they are, so `convert-video` to mkv copies.
const MKV_COPY: &[&str] = &[
    "h264",
    "hevc",
    "av1",
    "vp8",
    "vp9",
    "mpeg4",
    "mpeg2video",
    "mpeg1video",
    "theora",
    "aac",
    "mp3",
    "mp2",
    "opus",
    "vorbis",
    "flac",
    "ac3",
    "eac3",
    "dts",
    "truehd",
    "alac",
    "pcm_s16le",
    "pcm_s24le",
    "pcm_s32le",
    "pcm_f32le",
];

fn mkv_can_copy(info: &MediaInfo) -> bool {
    let av: Vec<_> = info
        .streams
        .iter()
        .filter(|s| s.kind == "video" || s.kind == "audio")
        .collect();
    !av.is_empty() && av.iter().all(|s| MKV_COPY.contains(&s.codec.as_str()))
}

/// Audio codec arguments for an audio-only target extension.
fn audio_codec(ext: &str, enc: &Encoders) -> Option<Result<Vec<String>, EngineError>> {
    Some(match ext {
        "mp3" if has(enc, "libmp3lame") => Ok(args(&["-c:a", "libmp3lame", "-q:a", "2"])),
        "mp3" => Err(missing("MP3 (libmp3lame)")),
        "ogg" | "oga" if has(enc, "libvorbis") => Ok(args(&["-c:a", "libvorbis", "-q:a", "5"])),
        "ogg" | "oga" => opus(enc, "128k"),
        "opus" => opus(enc, "128k"),
        "m4a" => {
            let mut v = aac("192k");
            v.extend(args(&["-movflags", "+faststart"]));
            Ok(v)
        }
        "flac" => Ok(args(&["-c:a", "flac"])),
        "wav" => Ok(args(&["-c:a", "pcm_s16le"])),
        _ => return None,
    })
}

/// Video ops map the first video stream and every audio stream.
fn video_map() -> Vec<String> {
    args(&["-map", "0:v:0", "-map", "0:a?", "-sn", "-dn"])
}

/// Containers `compress-video` re-encodes with H.264 and AAC.
const H264_CONTAINERS: &[&str] = &["mp4", "m4v", "mov", "mkv", "avi", "flv", "ts", "3gp"];

/// Codec and filter arguments placed between the input and the output.
pub fn plan(
    op: &str,
    ext: &str,
    p: &Params,
    info: &MediaInfo,
    enc: &Encoders,
) -> Result<Vec<String>, EngineError> {
    let faststart = || args(&["-movflags", "+faststart"]);
    match op {
        "convert-video" => {
            let mut v = video_map();
            match ext {
                "mp4" | "m4v" | "mov" => {
                    v.extend(h264(enc, p.crf.unwrap_or(23), "4M")?);
                    v.extend(args(&["-vf", EVEN_SIZE]));
                    v.extend(aac("160k"));
                    v.extend(faststart());
                }
                "webm" => {
                    v.extend(vp9(enc, p.crf.unwrap_or(32))?);
                    v.extend(opus(enc, "128k")?);
                }
                "mkv" if mkv_can_copy(info) => v.extend(args(&["-c", "copy"])),
                "mkv" => {
                    v.extend(h264(enc, p.crf.unwrap_or(23), "4M")?);
                    v.extend(args(&["-vf", EVEN_SIZE]));
                    v.extend(aac("160k"));
                }
                _ => return Err(unsupported(op, ext)),
            }
            Ok(v)
        }
        "to-gif" => {
            if ext != "gif" {
                return Err(unsupported(op, ext));
            }
            let want = p.width.unwrap_or(480);
            let width = info
                .first_video()
                .and_then(|s| s.width)
                .map_or(want, |w| w.min(want));
            let fps = p.fps.unwrap_or(12.0);
            let graph = format!(
                "fps={fps},scale={width}:-1:flags=lanczos,split[a][b];[a]palettegen[p];[b][p]paletteuse"
            );
            Ok(args(&[
                "-map", "0:v:0", "-an", "-sn", "-dn", "-vf", &graph, "-loop", "0",
            ]))
        }
        "extract-audio" | "convert-audio" => {
            let mut v = args(&["-map", "0:a:0", "-vn", "-sn", "-dn"]);
            v.extend(audio_codec(ext, enc).ok_or_else(|| unsupported(op, ext))??);
            Ok(v)
        }
        "compress-video" => {
            let mut v = video_map();
            if ext == "webm" {
                v.extend(vp9(enc, p.webm_crf.unwrap_or(36))?);
                v.extend(opus(enc, "96k")?);
                return Ok(v);
            }
            if !H264_CONTAINERS.contains(&ext) {
                return Err(unsupported(op, ext));
            }
            v.extend(h264(enc, p.crf.unwrap_or(28), "2M")?);
            let max = p.max_height.unwrap_or(1080) / 2 * 2;
            let tall = info
                .first_video()
                .and_then(|s| s.height)
                .is_some_and(|h| h > max);
            let filter = if tall {
                format!("scale=-2:{max}")
            } else {
                EVEN_SIZE.to_string()
            };
            v.extend(["-vf".to_string(), filter]);
            v.extend(aac("128k"));
            if matches!(ext, "mp4" | "m4v" | "mov" | "3gp") {
                v.extend(faststart());
            }
            Ok(v)
        }
        _ => Err(EngineError::UnsupportedOp(op.to_string())),
    }
}

/// The full ffmpeg argument list for one run.
pub fn command_args(
    input: &Path,
    output: &Path,
    ext: &str,
    codec_args: Vec<String>,
) -> Result<Vec<OsString>, EngineError> {
    let muxer =
        muxer_for(ext).ok_or_else(|| EngineError::UnsupportedOp(format!("output .{ext}")))?;
    let mut v: Vec<OsString> = [
        "-hide_banner",
        "-nostdin",
        "-nostats",
        "-loglevel",
        "error",
        "-progress",
        "pipe:1",
        "-y",
        "-i",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    v.push(input_url(input));
    v.extend(codec_args.into_iter().map(OsString::from));
    v.push("-f".into());
    v.push(muxer.into());
    v.push(input_url(output));
    Ok(v)
}

/// Fraction done from one `-progress` line, or `None` for other lines.
pub fn progress_fraction(line: &str, duration: Option<f64>) -> Option<f32> {
    if line.trim() == "progress=end" {
        return Some(1.0);
    }
    let us: f64 = line.strip_prefix("out_time_us=")?.trim().parse().ok()?;
    let total = duration?;
    Some((us / 1_000_000.0 / total).clamp(0.0, 1.0) as f32)
}

/// Keeps the last `cap` lines.
pub fn push_tail(tail: &mut VecDeque<String>, line: String, cap: usize) {
    if tail.len() == cap {
        tail.pop_front();
    }
    tail.push_back(line);
}

fn read_tail(stream: impl Read) -> VecDeque<String> {
    let mut tail = VecDeque::new();
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if !line.trim().is_empty() {
            push_tail(&mut tail, line, STDERR_TAIL);
        }
    }
    tail
}

/// Reads progress, honours cancel, and turns the exit status into a result.
fn supervise(
    mut child: Child,
    duration: Option<f64>,
    progress: Progress,
    cancel: &CancelToken,
) -> Result<(), EngineError> {
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let err_thread = std::thread::spawn(move || read_tail(stderr));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let mut last = 0.0f32;
    loop {
        if cancel.is_cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Err(EngineError::Cancelled);
        }
        match rx.recv_timeout(CANCEL_POLL) {
            Ok(line) => {
                if let Some(f) = progress_fraction(&line, duration) {
                    // 1.0 is reserved for a zero exit status.
                    let f = f.min(0.999);
                    if f > last {
                        last = f;
                        progress(f);
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    let status = child.wait()?;
    let tail = err_thread.join().unwrap_or_default();
    if status.success() {
        progress(1.0);
        Ok(())
    } else {
        let mut msg = format!("ffmpeg failed ({status})");
        for line in tail {
            msg.push('\n');
            msg.push_str(&line);
        }
        Err(EngineError::Failed(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // S-028a AC2 (backlog/S-028-review-fixes.md): `located()` looks up
    // ffmpeg and ffprobe once per process, and `default()` uses that result.

    #[test]
    fn s028_ac2_located_is_one_value_per_process() {
        let a: &'static (PathBuf, PathBuf) = located();
        let b: &'static (PathBuf, PathBuf) = located();
        assert!(std::ptr::eq(a, b), "two calls must return the same value");
    }

    #[test]
    fn s028_ac2_default_engine_has_the_located_paths() {
        let e = FfmpegEngine::default();
        assert_eq!(e.ffmpeg, located().0);
        assert_eq!(e.ffprobe, located().1);
    }

    fn all_encoders() -> Encoders {
        [
            "libx264",
            "libopenh264",
            "libvpx-vp9",
            "libopus",
            "libvorbis",
            "libmp3lame",
            "aac",
            "flac",
            "pcm_s16le",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn without(names: &[&str]) -> Encoders {
        let mut e = all_encoders();
        for n in names {
            e.remove(*n);
        }
        e
    }

    fn video(w: i64, h: i64, codec: &str) -> MediaInfo {
        MediaInfo {
            duration: Some(2.0),
            streams: vec![
                StreamInfo {
                    kind: "video".into(),
                    codec: codec.into(),
                    width: Some(w),
                    height: Some(h),
                },
                StreamInfo {
                    kind: "audio".into(),
                    codec: "aac".into(),
                    ..Default::default()
                },
            ],
        }
    }

    fn joined(v: &[String]) -> String {
        v.join(" ")
    }

    fn plan_ok(op: &str, ext: &str, p: &Params, info: &MediaInfo) -> String {
        joined(&plan(op, ext, p, info, &all_encoders()).unwrap())
    }

    #[test]
    fn mp4_and_mov_use_x264_aac_faststart() {
        let info = video(320, 240, "mpeg4");
        for ext in ["mp4", "m4v", "mov"] {
            let a = plan_ok("convert-video", ext, &Params::default(), &info);
            assert!(
                a.contains("-c:v libx264 -preset medium -crf 23 -pix_fmt yuv420p"),
                "{a}"
            );
            assert!(a.contains("-vf scale=trunc(iw/2)*2:trunc(ih/2)*2"), "{a}");
            assert!(a.contains("-c:a aac -b:a 160k"), "{a}");
            assert!(a.contains("-movflags +faststart"), "{a}");
            assert!(a.starts_with("-map 0:v:0 -map 0:a? -sn -dn"), "{a}");
        }
        let p = Params {
            crf: Some(18),
            ..Default::default()
        };
        assert!(plan_ok("convert-video", "mp4", &p, &info).contains("-crf 18"));
    }

    #[test]
    fn webm_uses_vp9_crf_32_and_opus() {
        let a = plan_ok(
            "convert-video",
            "webm",
            &Params::default(),
            &video(1, 1, "h264"),
        );
        assert!(
            a.contains("-c:v libvpx-vp9 -crf 32 -b:v 0 -row-mt 1 -deadline good -cpu-used 2"),
            "{a}"
        );
        assert!(a.contains("-c:a libopus -b:a 128k"), "{a}");
        assert!(!a.contains("faststart"));
    }

    #[test]
    fn mkv_copies_only_compatible_streams() {
        let p = Params::default();
        let copy = plan_ok("convert-video", "mkv", &p, &video(1, 1, "h264"));
        assert!(copy.ends_with("-c copy"), "{copy}");
        let reencode = plan_ok("convert-video", "mkv", &p, &video(1, 1, "wmv3"));
        assert!(reencode.contains("libx264") && reencode.contains("-c:a aac"));
        assert!(!reencode.contains("faststart"));
        let unknown = plan_ok("convert-video", "mkv", &p, &MediaInfo::default());
        assert!(unknown.contains("libx264"), "unprobed input is re-encoded");
        let mut subs = video(1, 1, "h264");
        subs.streams.push(StreamInfo {
            kind: "subtitle".into(),
            codec: "mov_text".into(),
            ..Default::default()
        });
        assert!(mkv_can_copy(&subs), "subtitles are dropped, not checked");
        let mut bad_audio = video(1, 1, "h264");
        bad_audio.streams[1].codec = "wmav2".into();
        assert!(!mkv_can_copy(&bad_audio));
    }

    #[test]
    fn convert_video_rejects_other_targets() {
        let r = plan(
            "convert-video",
            "gif",
            &Params::default(),
            &video(1, 1, "h264"),
            &all_encoders(),
        );
        assert!(matches!(r, Err(EngineError::UnsupportedOp(_))));
    }

    #[test]
    fn h264_falls_back_to_openh264_then_unavailable() {
        let info = video(1, 1, "h264");
        let p = Params::default();
        let a = joined(&plan("convert-video", "mp4", &p, &info, &without(&["libx264"])).unwrap());
        assert!(
            a.contains("-c:v libopenh264 -rc_mode bitrate -b:v 4M -pix_fmt yuv420p"),
            "{a}"
        );
        let c = joined(&plan("compress-video", "mp4", &p, &info, &without(&["libx264"])).unwrap());
        assert!(c.contains("-b:v 2M"), "{c}");
        match plan(
            "convert-video",
            "mov",
            &p,
            &info,
            &without(&["libx264", "libopenh264"]),
        ) {
            Err(EngineError::Unavailable(m)) => assert!(m.contains("libx264"), "{m}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn missing_encoders_are_named() {
        let info = video(1, 1, "h264");
        let p = Params::default();
        let cases = [
            ("convert-video", "webm", "libvpx-vp9", "libvpx-vp9"),
            ("convert-video", "webm", "libopus", "libopus"),
            ("extract-audio", "mp3", "libmp3lame", "libmp3lame"),
            ("convert-audio", "opus", "libopus", "libopus"),
            ("compress-video", "webm", "libvpx-vp9", "libvpx-vp9"),
        ];
        for (op, ext, gone, named) in cases {
            match plan(op, ext, &p, &info, &without(&[gone])) {
                Err(EngineError::Unavailable(m)) => assert!(m.contains(named), "{m}"),
                other => panic!("{op} {ext}: {other:?}"),
            }
        }
        let ogg =
            joined(&plan("convert-audio", "ogg", &p, &info, &without(&["libvorbis"])).unwrap());
        assert!(ogg.contains("-c:a libopus"), "{ogg}");
        let r = plan(
            "convert-audio",
            "ogg",
            &p,
            &info,
            &without(&["libvorbis", "libopus"]),
        );
        assert!(matches!(r, Err(EngineError::Unavailable(_))));
    }

    #[test]
    fn audio_targets() {
        let info = video(1, 1, "h264");
        let p = Params::default();
        let expect = [
            ("mp3", "-c:a libmp3lame -q:a 2"),
            ("ogg", "-c:a libvorbis -q:a 5"),
            ("oga", "-c:a libvorbis -q:a 5"),
            ("opus", "-c:a libopus -b:a 128k"),
            ("m4a", "-c:a aac -b:a 192k -movflags +faststart"),
            ("flac", "-c:a flac"),
            ("wav", "-c:a pcm_s16le"),
        ];
        for op in ["extract-audio", "convert-audio"] {
            for (ext, codec) in expect {
                let a = plan_ok(op, ext, &p, &info);
                assert_eq!(a, format!("-map 0:a:0 -vn -sn -dn {codec}"));
            }
            assert!(matches!(
                plan(op, "mp4", &p, &info, &all_encoders()),
                Err(EngineError::UnsupportedOp(_))
            ));
        }
    }

    #[test]
    fn gif_graph_uses_params_and_never_upscales() {
        let p = Params {
            fps: Some(10.0),
            width: Some(160),
            ..Default::default()
        };
        let a = plan_ok("to-gif", "gif", &p, &video(320, 240, "h264"));
        assert_eq!(
            a,
            "-map 0:v:0 -an -sn -dn -vf fps=10,scale=160:-1:flags=lanczos,split[a][b];\
             [a]palettegen[p];[b][p]paletteuse -loop 0"
        );
        let d = plan_ok(
            "to-gif",
            "gif",
            &Params::default(),
            &video(1920, 1080, "h264"),
        );
        assert!(d.contains("fps=12,scale=480:-1"), "{d}");
        let small = plan_ok(
            "to-gif",
            "gif",
            &Params::default(),
            &video(320, 240, "h264"),
        );
        assert!(small.contains("scale=320:-1"), "{small}");
        let unprobed = plan_ok("to-gif", "gif", &Params::default(), &MediaInfo::default());
        assert!(unprobed.contains("scale=480:-1"), "{unprobed}");
        let half = Params {
            fps: Some(12.5),
            ..Default::default()
        };
        assert!(plan_ok("to-gif", "gif", &half, &MediaInfo::default()).contains("fps=12.5,"));
        assert!(matches!(
            plan("to-gif", "mp4", &p, &MediaInfo::default(), &all_encoders()),
            Err(EngineError::UnsupportedOp(_))
        ));
    }

    #[test]
    fn compress_scales_only_tall_inputs() {
        let p = Params::default();
        let tall = plan_ok("compress-video", "mp4", &p, &video(3840, 2160, "h264"));
        assert!(tall.contains("-crf 28"), "{tall}");
        assert!(tall.contains("-vf scale=-2:1080"), "{tall}");
        assert!(tall.contains("-c:a aac -b:a 128k"), "{tall}");
        assert!(tall.contains("+faststart"), "{tall}");
        let exact = plan_ok("compress-video", "mp4", &p, &video(1920, 1080, "h264"));
        assert!(exact.contains(EVEN_SIZE), "{exact}");
        let odd = Params {
            max_height: Some(181),
            crf: Some(30),
            ..Default::default()
        };
        let a = plan_ok("compress-video", "mkv", &odd, &video(320, 360, "h264"));
        assert!(a.contains("scale=-2:180") && a.contains("-crf 30"), "{a}");
        assert!(!a.contains("faststart"), "{a}");
        let just_over = plan_ok("compress-video", "mkv", &odd, &video(320, 181, "h264"));
        assert!(just_over.contains("scale=-2:180"), "{just_over}");
        let unprobed = plan_ok("compress-video", "avi", &p, &MediaInfo::default());
        assert!(unprobed.contains(EVEN_SIZE), "{unprobed}");
        for ext in ["mov", "m4v", "3gp"] {
            assert!(
                plan_ok("compress-video", ext, &p, &video(1, 1, "h264")).contains("+faststart")
            );
        }
    }

    #[test]
    fn compress_webm_and_unsupported_containers() {
        let p = Params::default();
        let w = plan_ok("compress-video", "webm", &p, &video(1, 1, "vp9"));
        assert!(
            w.contains("-c:v libvpx-vp9 -crf 36") && w.contains("-c:a libopus -b:a 96k"),
            "{w}"
        );
        let q = Params {
            webm_crf: Some(40),
            ..Default::default()
        };
        assert!(plan_ok("compress-video", "webm", &q, &video(1, 1, "vp9")).contains("-crf 40"));
        for ext in ["wmv", "mpeg", "mpg", "gif"] {
            assert!(matches!(
                plan(
                    "compress-video",
                    ext,
                    &p,
                    &video(1, 1, "h264"),
                    &all_encoders()
                ),
                Err(EngineError::UnsupportedOp(_))
            ));
        }
        assert!(matches!(
            plan(
                "trim-silence",
                "wav",
                &p,
                &MediaInfo::default(),
                &all_encoders()
            ),
            Err(EngineError::UnsupportedOp(_))
        ));
    }

    #[test]
    fn muxers() {
        let cases = [
            ("mp4", "mp4"),
            ("m4v", "mp4"),
            ("m4a", "mp4"),
            ("mov", "mov"),
            ("mkv", "matroska"),
            ("webm", "webm"),
            ("avi", "avi"),
            ("flv", "flv"),
            ("ts", "mpegts"),
            ("3gp", "3gp"),
            ("gif", "gif"),
            ("mp3", "mp3"),
            ("ogg", "ogg"),
            ("oga", "ogg"),
            ("opus", "opus"),
            ("flac", "flac"),
            ("wav", "wav"),
        ];
        for (ext, m) in cases {
            assert_eq!(muxer_for(ext), Some(m), "{ext}");
        }
        assert_eq!(muxer_for("wmv"), None);
    }

    #[test]
    fn command_line_uses_file_urls_and_explicit_muxer() {
        let a = command_args(
            Path::new("/v/-in.mp4"),
            Path::new("/v/.tmp-1.webm"),
            "webm",
            vec!["-c".into(), "copy".into()],
        )
        .unwrap();
        let s: Vec<_> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
        assert_eq!(
            s,
            [
                "-hide_banner",
                "-nostdin",
                "-nostats",
                "-loglevel",
                "error",
                "-progress",
                "pipe:1",
                "-y",
                "-i",
                "file:/v/-in.mp4",
                "-c",
                "copy",
                "-f",
                "webm",
                "file:/v/.tmp-1.webm",
            ]
        );
        assert!(matches!(
            command_args(Path::new("/a"), Path::new("/b.xyz"), "xyz", vec![]),
            Err(EngineError::UnsupportedOp(_))
        ));
    }

    #[test]
    fn relative_paths_get_dot_slash() {
        assert_eq!(
            input_url(Path::new("-x.mp4")),
            OsString::from("file:./-x.mp4")
        );
        assert_eq!(
            input_url(Path::new("/abs/x")),
            OsString::from("file:/abs/x")
        );
    }

    #[test]
    fn progress_lines() {
        assert_eq!(
            progress_fraction("out_time_us=1000000", Some(2.0)),
            Some(0.5)
        );
        assert_eq!(
            progress_fraction("out_time_us=5000000", Some(2.0)),
            Some(1.0)
        );
        assert_eq!(progress_fraction("out_time_us=-30", Some(2.0)), Some(0.0));
        assert_eq!(progress_fraction("out_time_us=N/A", Some(2.0)), None);
        assert_eq!(progress_fraction("out_time_us=1000000", None), None);
        assert_eq!(progress_fraction("out_time_ms=1000000", Some(2.0)), None);
        assert_eq!(progress_fraction("progress=continue", Some(2.0)), None);
        assert_eq!(progress_fraction("progress=end", None), Some(1.0));
    }

    #[test]
    fn stderr_tail_keeps_last_lines() {
        let mut t = VecDeque::new();
        for i in 0..25 {
            push_tail(&mut t, i.to_string(), STDERR_TAIL);
        }
        assert_eq!(t.len(), 20);
        assert_eq!(t.front().map(String::as_str), Some("5"));
        assert_eq!(t.back().map(String::as_str), Some("24"));
        let read = read_tail("a\n\n  \nb\n".as_bytes());
        assert_eq!(read, ["a", "b"]);
    }

    #[test]
    fn parses_probe_output() {
        let text = "streams.stream.0.codec_name=\"h264\"\n\
                    streams.stream.0.codec_type=\"video\"\n\
                    streams.stream.0.width=320\n\
                    streams.stream.0.height=240\n\
                    streams.stream.1.codec_name=\"aac\"\n\
                    streams.stream.1.codec_type=\"audio\"\n\
                    streams.stream.x.codec_name=\"bad\"\n\
                    streams.stream.1\n\
                    junk\n\
                    other.key=1\n\
                    format.duration=\"2.040000\"\n";
        let info = parse_probe(text);
        assert_eq!(info.duration, Some(2.04));
        assert_eq!(info, video_like(&info));
        assert_eq!(info.streams.len(), 2);
        assert_eq!(info.first_video().and_then(|s| s.height), Some(240));
        assert_eq!(info.first_video().and_then(|s| s.width), Some(320));
        assert_eq!(info.streams[1].codec, "aac");
        assert_eq!(parse_probe("format.duration=\"N/A\"\n").duration, None);
        assert_eq!(parse_probe("format.duration=\"0.000\"\n").duration, None);
        let audio_first = parse_probe(
            "streams.stream.0.codec_type=\"audio\"\nstreams.stream.1.codec_type=\"video\"\nstreams.stream.1.width=8\n",
        );
        assert_eq!(audio_first.first_video().and_then(|s| s.width), Some(8));
    }

    fn video_like(info: &MediaInfo) -> MediaInfo {
        let mut v = video(320, 240, "h264");
        v.duration = info.duration;
        v
    }

    #[test]
    fn parses_encoder_list() {
        let text =
            "Encoders:\n V..... = Video\n ------\n V....D libx264   H.264\n A....D aac  AAC\n";
        let e = parse_encoders(text);
        assert_eq!(e.len(), 2);
        assert!(e.contains("libx264") && e.contains("aac"));
        assert!(!e.contains("="), "legend lines are skipped");
    }

    fn task_with(params: &str) -> Task {
        Task {
            action_id: "t".into(),
            op: "convert-video".into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![],
            output: PathBuf::from("o.mp4"),
        }
    }

    #[test]
    fn params_are_validated_at_both_ends() {
        let ok = Params::from_task(&task_with(
            "crf = 0\nwebm_crf = 63\nfps = 60\nwidth = 16\nmax_height = 8640",
        ))
        .unwrap();
        assert_eq!(
            ok,
            Params {
                crf: Some(0),
                webm_crf: Some(63),
                fps: Some(60.0),
                width: Some(16),
                max_height: Some(8640),
            }
        );
        let low = Params::from_task(&task_with("fps = 1\nwidth = 4096\nmax_height = 16")).unwrap();
        assert_eq!(
            (low.fps, low.width, low.max_height),
            (Some(1.0), Some(4096), Some(16))
        );
        for bad in [
            "crf = -1",
            "crf = 64",
            "crf = \"x\"",
            "webm_crf = 64",
            "fps = 0.5",
            "fps = 61",
            "fps = \"x\"",
            "width = 15",
            "width = 4097",
            "max_height = 15",
            "max_height = 8641",
        ] {
            assert!(
                matches!(
                    Params::from_task(&task_with(bad)),
                    Err(EngineError::BadParam { .. })
                ),
                "{bad}"
            );
        }
        assert_eq!(
            Params::from_task(&task_with("")).unwrap(),
            Params::default()
        );
    }

    #[test]
    fn engine_wrong_input_count_is_bad_param() {
        let e = FfmpegEngine::default();
        let t = task_with("");
        let r = e.run(&t, &|_| {}, &CancelToken::new());
        assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    }

    // S-030 helpers (backlog/S-030-process-tree.md): the tree is
    // `sh -c 'sleep 30 & echo $! > "$1"; wait' sh <pidfile>`.

    #[cfg(unix)]
    fn s030_pidfile_tree(pidfile: &Path) -> Command {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "sleep 30 & echo $! > \"$1\"; wait", "sh"])
            .arg(pidfile);
        cmd
    }

    /// Polls `pidfile` up to 3 s for a pid.
    #[cfg(unix)]
    fn s030_read_pidfile(pidfile: &Path) -> Option<i32> {
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if let Ok(text) = std::fs::read_to_string(pidfile) {
                if let Ok(pid) = text.trim().parse() {
                    return Some(pid);
                }
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// True when `/proc/<pid>` stays absent or a zombie within 2 s.
    #[cfg(unix)]
    fn s030_gone_soon(pid: i32) -> bool {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let gone = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(_) => true,
                Ok(stat) => stat
                    .rsplit_once(')')
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    .is_some_and(|state| state == "Z"),
            };
            if gone {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn s030_ac6_supervise_cancel_kills_the_grandchild() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let mut cmd = s030_pidfile_tree(&pidfile);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let tree = crate::process::ProcessTree::spawn(&mut cmd).expect("spawn the tree");
        let grandchild = s030_read_pidfile(&pidfile).expect("no grandchild pid within 3 s");
        let cancel = CancelToken::new();
        cancel.cancel();
        let r = supervise(tree, None, &|_| {}, &cancel);
        assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
        assert!(
            s030_gone_soon(grandchild),
            "grandchild {grandchild} survived the cancel"
        );
    }
}
