//! S-008 integration tests: the FFmpeg engine against the real system
//! `ffmpeg`. Fixtures are generated with lavfi inside a temp dir, nothing is
//! committed. Every assertion reads the real output back with ffprobe.

mod ffmpeg {
    use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Task};
    use sbarhive_engines::ffmpeg::FfmpegEngine;
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// `ffmpeg -f lavfi` fixture: testsrc video plus a 440 Hz sine, encoded
    /// with encoders every FFmpeg build has (mpeg4 and aac).
    fn make_video(dir: &Path, name: &str, secs: u32, size: &str) -> PathBuf {
        let out = dir.join(name);
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "lavfi", "-i"])
            .arg(format!("testsrc=size={size}:rate=25"))
            .args(["-f", "lavfi", "-i", "sine=frequency=440"])
            .args(["-t", &secs.to_string()])
            .args(["-c:v", "mpeg4", "-q:v", "5", "-c:a", "aac"])
            .arg(&out)
            .status()
            .expect("ffmpeg runs");
        assert!(status.success(), "fixture {name} failed");
        out
    }

    fn make_wav(dir: &Path) -> PathBuf {
        let out = dir.join("tone.wav");
        let status = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args(["-f", "lavfi", "-i", "sine=frequency=440", "-t", "2"])
            .arg(&out)
            .status()
            .expect("ffmpeg runs");
        assert!(status.success());
        out
    }

    fn probe(path: &Path) -> Value {
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
            ])
            .arg(path)
            .output()
            .expect("ffprobe runs");
        assert!(out.status.success(), "ffprobe failed on {}", path.display());
        serde_json::from_slice(&out.stdout).unwrap()
    }

    fn duration(v: &Value) -> f64 {
        v["format"]["duration"].as_str().unwrap().parse().unwrap()
    }

    fn streams<'a>(v: &'a Value, kind: &str) -> Vec<&'a Value> {
        v["streams"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["codec_type"] == kind)
            .collect()
    }

    fn codec(v: &Value, kind: &str) -> String {
        streams(v, kind)
            .first()
            .unwrap_or_else(|| panic!("no {kind} stream"))["codec_name"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn task(op: &str, input: &Path, output: PathBuf, params: &str) -> Task {
        Task {
            action_id: format!("test.{op}"),
            op: op.into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![input.to_path_buf()],
            output,
        }
    }

    /// Runs a task, returning the result and every progress value reported.
    fn run(engine: &FfmpegEngine, t: &Task) -> (Result<(), EngineError>, Vec<f32>) {
        let seen = Mutex::new(Vec::new());
        let cb = |f: f32| seen.lock().unwrap().push(f);
        let r = engine.run(t, &cb, &CancelToken::new());
        (r, seen.into_inner().unwrap())
    }

    fn assert_progress(seen: &[f32]) {
        assert!(!seen.is_empty(), "no progress reported");
        assert!(
            seen.windows(2).all(|w| w[0] <= w[1]),
            "progress not monotonic: {seen:?}"
        );
        assert!(seen.iter().all(|f| (0.0..=1.0).contains(f)), "{seen:?}");
        assert_eq!(*seen.last().unwrap(), 1.0);
    }

    fn assert_duration(v: &Value, expected: f64) {
        let d = duration(v);
        assert!(
            (d - expected).abs() < 0.2,
            "duration {d}, expected {expected}"
        );
    }

    #[test]
    fn default_engine_is_available_and_named_ffmpeg() {
        let e = FfmpegEngine::default();
        assert_eq!(e.id(), "ffmpeg");
        assert!(e.available());
        assert!(e.available(), "cached second call agrees");
    }

    #[test]
    fn missing_binary_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 1, "320x240");
        let e = FfmpegEngine::with_binaries("/nonexistent/ffmpeg", "/nonexistent/ffprobe");
        assert!(!e.available());
        let t = task("convert-video", &input, dir.path().join("o.mp4"), "");
        let (r, _) = run(&e, &t);
        assert!(matches!(r, Err(EngineError::Unavailable(_))), "{r:?}");
    }

    #[test]
    fn missing_ffprobe_alone_is_unavailable() {
        let e = FfmpegEngine::with_binaries("ffmpeg", "/nonexistent/ffprobe");
        assert!(!e.available());
    }

    #[test]
    fn capabilities_list_the_five_ops() {
        let mut caps = FfmpegEngine::default().capabilities();
        caps.sort_by_key(|c| c.0);
        assert_eq!(
            caps,
            vec![
                ("compress-video", vec![MediaKind::Video]),
                ("convert-audio", vec![MediaKind::Audio]),
                ("convert-video", vec![MediaKind::Video]),
                ("extract-audio", vec![MediaKind::Video]),
                ("to-gif", vec![MediaKind::Video]),
            ]
        );
    }

    #[test]
    fn unknown_op_is_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_wav(dir.path());
        let t = task("trim-silence", &input, dir.path().join("o.wav"), "");
        let (r, _) = run(&FfmpegEngine::default(), &t);
        assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");
    }

    #[test]
    fn convert_video_to_mp4_h264_aac_with_progress() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mkv", 2, "320x240");
        let out = dir.path().join(".tmp-out.mp4");
        let (r, seen) = run(
            &FfmpegEngine::default(),
            &task("convert-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(v["format"]["format_name"].as_str().unwrap().contains("mp4"));
        assert_eq!(codec(&v, "video"), "h264");
        assert_eq!(codec(&v, "audio"), "aac");
        assert_duration(&v, 2.0);
        assert_progress(&seen);
    }

    #[test]
    fn convert_video_to_webm_vp9_opus() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 2, "320x240");
        let out = dir.path().join("out.webm");
        let (r, seen) = run(
            &FfmpegEngine::default(),
            &task("convert-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(
            v["format"]["format_name"]
                .as_str()
                .unwrap()
                .contains("webm")
        );
        assert_eq!(codec(&v, "video"), "vp9");
        assert_eq!(codec(&v, "audio"), "opus");
        assert_duration(&v, 2.0);
        assert_progress(&seen);
    }

    #[test]
    fn convert_video_to_mkv_copies_compatible_streams() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 2, "320x240");
        let out = dir.path().join("out.mkv");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("convert-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(
            v["format"]["format_name"]
                .as_str()
                .unwrap()
                .contains("matroska")
        );
        // mpeg4 survives only if the stream was copied, not re-encoded.
        assert_eq!(codec(&v, "video"), "mpeg4");
        assert_eq!(codec(&v, "audio"), "aac");
        assert_duration(&v, 2.0);
    }

    #[test]
    fn convert_video_to_mov_h264() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 2, "320x240");
        let out = dir.path().join("out.mov");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("convert-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(v["format"]["format_name"].as_str().unwrap().contains("mov"));
        assert_eq!(codec(&v, "video"), "h264");
        assert_eq!(codec(&v, "audio"), "aac");
        assert_duration(&v, 2.0);
    }

    #[test]
    fn to_gif_scales_to_width_and_keeps_aspect() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 2, "320x240");
        let out = dir.path().join("out.gif");
        let (r, seen) = run(
            &FfmpegEngine::default(),
            &task("to-gif", &input, out.clone(), "width = 160\nfps = 10"),
        );
        r.unwrap();
        let v = probe(&out);
        assert_eq!(codec(&v, "video"), "gif");
        let s = streams(&v, "video")[0];
        assert_eq!(
            (s["width"].as_i64(), s["height"].as_i64()),
            (Some(160), Some(120))
        );
        assert!(streams(&v, "audio").is_empty());
        assert_progress(&seen);
    }

    #[test]
    fn to_gif_never_upscales() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 1, "320x240");
        let out = dir.path().join("out.gif");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("to-gif", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        let s = streams(&v, "video")[0];
        assert_eq!(
            (s["width"].as_i64(), s["height"].as_i64()),
            (Some(320), Some(240))
        );
    }

    const AUDIO_TARGETS: &[(&str, &str)] = &[
        ("mp3", "mp3"),
        ("ogg", "vorbis"),
        ("opus", "opus"),
        ("m4a", "aac"),
        ("flac", "flac"),
        ("wav", "pcm_s16le"),
    ];

    #[test]
    fn extract_audio_to_every_format_has_no_video() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 2, "320x240");
        let engine = FfmpegEngine::default();
        for (ext, want) in AUDIO_TARGETS {
            let out = dir.path().join(format!("out.{ext}"));
            let (r, seen) = run(&engine, &task("extract-audio", &input, out.clone(), ""));
            r.unwrap_or_else(|e| panic!("{ext}: {e}"));
            let v = probe(&out);
            assert_eq!(&codec(&v, "audio"), want, "{ext}");
            assert!(streams(&v, "video").is_empty(), "{ext} has video");
            assert_duration(&v, 2.0);
            assert_progress(&seen);
        }
    }

    #[test]
    fn convert_audio_to_every_format() {
        let dir = tempfile::tempdir().unwrap();
        let wav = make_wav(dir.path());
        let engine = FfmpegEngine::default();
        for (ext, want) in AUDIO_TARGETS {
            let out = dir.path().join(format!("converted.{ext}"));
            let (r, seen) = run(&engine, &task("convert-audio", &wav, out.clone(), ""));
            r.unwrap_or_else(|e| panic!("{ext}: {e}"));
            let v = probe(&out);
            assert_eq!(&codec(&v, "audio"), want, "{ext}");
            assert!(streams(&v, "video").is_empty(), "{ext} has video");
            assert_duration(&v, 2.0);
            assert_progress(&seen);
        }
    }

    #[test]
    fn extract_audio_from_video_without_audio_fails() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("silent.mp4");
        let ok = Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=160x120:rate=25",
                "-t",
                "1",
            ])
            .args(["-c:v", "mpeg4"])
            .arg(&input)
            .status()
            .unwrap();
        assert!(ok.success());
        let t = task("extract-audio", &input, dir.path().join("o.mp3"), "");
        let (r, _) = run(&FfmpegEngine::default(), &t);
        match r {
            Err(EngineError::Failed(msg)) => assert!(!msg.trim().is_empty()),
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn compress_video_scales_down_tall_input_and_keeps_container() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "tall.mp4", 2, "320x360");
        let out = dir.path().join("tall-compressed.mp4");
        let (r, seen) = run(
            &FfmpegEngine::default(),
            &task("compress-video", &input, out.clone(), "max_height = 180"),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(v["format"]["format_name"].as_str().unwrap().contains("mp4"));
        assert_eq!(codec(&v, "video"), "h264");
        assert_eq!(codec(&v, "audio"), "aac");
        let s = streams(&v, "video")[0];
        assert_eq!(
            (s["width"].as_i64(), s["height"].as_i64()),
            (Some(160), Some(180))
        );
        assert_duration(&v, 2.0);
        assert_progress(&seen);
    }

    #[test]
    fn compress_video_keeps_short_input_size_and_mkv_container() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mkv", 2, "320x240");
        let out = dir.path().join("in-compressed.mkv");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("compress-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert!(
            v["format"]["format_name"]
                .as_str()
                .unwrap()
                .contains("matroska")
        );
        assert_eq!(codec(&v, "video"), "h264");
        let s = streams(&v, "video")[0];
        assert_eq!(
            (s["width"].as_i64(), s["height"].as_i64()),
            (Some(320), Some(240))
        );
    }

    #[test]
    fn compress_webm_uses_vp9() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "in.mp4", 1, "320x240");
        let out = dir.path().join("in-compressed.webm");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("compress-video", &input, out.clone(), ""),
        );
        r.unwrap();
        let v = probe(&out);
        assert_eq!(codec(&v, "video"), "vp9");
        assert_eq!(codec(&v, "audio"), "opus");
    }

    #[test]
    fn cancel_kills_ffmpeg_quickly() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "long.mkv", 20, "320x240");
        let out = dir.path().join("long.mp4");
        let token = CancelToken::new();
        let fired = Mutex::new(None::<Instant>);
        let cb = |_f: f32| {
            let mut at = fired.lock().unwrap();
            if at.is_none() {
                *at = Some(Instant::now());
                token.cancel();
            }
        };
        let t = task("convert-video", &input, out, "");
        let started = Instant::now();
        let r = FfmpegEngine::default().run(&t, &cb, &token);
        let returned = Instant::now();
        assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
        let at = fired.into_inner().unwrap().expect("progress was reported");
        assert!(
            returned - at < Duration::from_secs(1),
            "took {:?} after cancel",
            returned - at
        );
        assert!(returned - started < Duration::from_secs(20));
    }

    #[test]
    fn cancel_before_start_returns_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_wav(dir.path());
        let token = CancelToken::new();
        token.cancel();
        let t = task("convert-audio", &input, dir.path().join("o.mp3"), "");
        let r = FfmpegEngine::default().run(&t, &|_| {}, &token);
        assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    }

    #[test]
    fn corrupt_input_fails_with_message() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("broken.mp4");
        let junk: Vec<u8> = (0..4096u32).map(|i| (i * 7919 % 251) as u8).collect();
        std::fs::write(&input, junk).unwrap();
        let t = task("convert-video", &input, dir.path().join("o.webm"), "");
        let (r, _) = run(&FfmpegEngine::default(), &t);
        match r {
            Err(EngineError::Failed(msg)) => {
                assert!(!msg.trim().is_empty());
                assert!(msg.lines().count() <= 21, "{msg}");
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn missing_input_is_an_input_error() {
        let dir = tempfile::tempdir().unwrap();
        let t = task(
            "convert-video",
            &dir.path().join("nope.mp4"),
            dir.path().join("o.webm"),
            "",
        );
        let (r, _) = run(&FfmpegEngine::default(), &t);
        assert!(matches!(r, Err(EngineError::Input { .. })), "{r:?}");
    }

    #[test]
    fn bad_crf_is_a_bad_param() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_wav(dir.path());
        let t = task(
            "compress-video",
            &input,
            dir.path().join("o.mp4"),
            "crf = 99",
        );
        let (r, _) = run(&FfmpegEngine::default(), &t);
        assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    }

    #[test]
    fn paths_starting_with_dash_are_not_options() {
        let dir = tempfile::tempdir().unwrap();
        let input = make_video(dir.path(), "-in.mp4", 1, "160x120");
        let out = dir.path().join("-out.mp3");
        let (r, _) = run(
            &FfmpegEngine::default(),
            &task("extract-audio", &input, out.clone(), ""),
        );
        r.unwrap();
        assert_eq!(codec(&probe(&out), "audio"), "mp3");
    }
}
