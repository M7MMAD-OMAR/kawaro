//! S-008 AC12: the video and audio action files parse as `ActionDef`s and
//! only use operations the FFmpeg engine declares.

mod ffmpeg {
    use kawaro_core::{ActionDef, Engine, MediaKind, OutputMode};
    use kawaro_engines::ffmpeg::FfmpegEngine;
    use serde::Deserialize;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[derive(Deserialize)]
    struct File {
        action: Vec<ActionDef>,
    }

    fn actions_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../actions")
    }

    fn load(name: &str) -> (String, Vec<ActionDef>) {
        let text = std::fs::read_to_string(actions_dir().join(name)).unwrap();
        let parsed: File = toml::from_str(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        (text, parsed.action)
    }

    /// (ring, output ext, code en) for each action, sorted.
    fn layout(actions: &[ActionDef]) -> Vec<(u8, String, String)> {
        let mut v: Vec<_> = actions
            .iter()
            .map(|a| (a.ring, a.output.ext.clone(), a.code.en.clone()))
            .collect();
        v.sort();
        v
    }

    fn common_checks(text: &str, actions: &[ActionDef], kind: MediaKind) {
        assert!(!text.contains('\u{2014}') && !text.contains('\u{2013}'));
        let ops: HashSet<_> = FfmpegEngine::default()
            .capabilities()
            .into_iter()
            .filter(|(_, kinds)| kinds.contains(&kind))
            .map(|(op, _)| op)
            .collect();
        let mut ids = HashSet::new();
        for a in actions {
            assert!(ids.insert(a.id.clone()), "duplicate id {}", a.id);
            assert_eq!(a.engine, "ffmpeg", "{}", a.id);
            assert!(ops.contains(a.op.as_str()), "{}: op {}", a.id, a.op);
            assert_eq!(a.accepts, vec![kind], "{}", a.id);
            assert_eq!(a.output.mode, OutputMode::PerFile, "{}", a.id);
            assert!(!a.label.ar.is_empty() && !a.label.en.is_empty());
            assert!(!a.code.ar.is_empty() && a.code.en.chars().count() <= 6);
        }
    }

    #[test]
    fn video_actions_match_the_plan() {
        let (text, actions) = load("video.toml");
        common_checks(&text, &actions, MediaKind::Video);
        let s = |x: &str| x.to_string();
        assert_eq!(
            layout(&actions),
            vec![
                (1, s(""), s("SHRINK")),
                (1, s("gif"), s("GIF")),
                (1, s("mp3"), s("MP3")),
                (1, s("mp4"), s("MP4")),
                (1, s("webm"), s("WEBM")),
                (2, s("flac"), s("FLAC")),
                (2, s("m4a"), s("M4A")),
                (2, s("mkv"), s("MKV")),
                (2, s("mov"), s("MOV")),
                (2, s("ogg"), s("OGG")),
                (2, s("wav"), s("WAV")),
            ]
        );
        let compress = actions.iter().find(|a| a.op == "compress-video").unwrap();
        assert_eq!(compress.output.suffix, "-compressed");
        assert_eq!(compress.code.ar, "ضغط");
        for ext in ["wmv", "mpeg", "mpg"] {
            assert!(compress.skip_ext.iter().any(|e| e == ext), "{ext}");
        }
        for a in actions.iter().filter(|a| a.op == "convert-video") {
            assert!(a.skip_ext.contains(&a.output.ext), "{}", a.id);
        }
        let mp3 = actions.iter().find(|a| a.output.ext == "mp3").unwrap();
        assert_eq!(mp3.op, "extract-audio");
        assert!(mp3.label.ar.contains("استخراج الصوت"));
    }

    #[test]
    fn audio_actions_match_the_plan() {
        let (text, actions) = load("audio.toml");
        common_checks(&text, &actions, MediaKind::Audio);
        let mut exts: Vec<_> = actions.iter().map(|a| a.output.ext.as_str()).collect();
        exts.sort();
        assert_eq!(exts, ["flac", "m4a", "mp3", "ogg", "opus", "wav"]);
        for a in &actions {
            assert_eq!(a.ring, 1, "{}", a.id);
            assert_eq!(a.op, "convert-audio");
            assert!(a.skip_ext.contains(&a.output.ext), "{}", a.id);
            assert_eq!(a.code.en, a.output.ext.to_ascii_uppercase());
        }
    }
}
