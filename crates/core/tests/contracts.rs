//! Tests for the shared contracts in engine.rs, action.rs, i18n.rs and ipc.rs.

use sbarhive_core::MediaKind;
use sbarhive_core::action::Localized;
use sbarhive_core::engine::{CancelToken, Engine, EngineError, Estimate, Progress, Task};
use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::Event;
use sbarhive_core::queue::JobEvent;
use std::path::PathBuf;

fn task(params: &str, inputs: Vec<PathBuf>) -> Task {
    Task {
        action_id: "a".into(),
        op: "op".into(),
        params: toml::from_str(params).unwrap(),
        inputs,
        output: PathBuf::from("/tmp/out"),
    }
}

#[test]
fn task_params_are_typed() {
    let t = task("s = \"x\"\ni = 7\nf = 1.5\nb = true", vec![]);
    assert_eq!(t.param_str("s"), Some("x"));
    assert_eq!(t.param_str("i"), None);
    assert_eq!(t.param_i64("i"), Some(7));
    assert_eq!(t.param_i64("s"), None);
    assert_eq!(t.param_f64("f"), Some(1.5));
    assert_eq!(t.param_f64("i"), Some(7.0));
    assert_eq!(t.param_f64("s"), None);
    assert_eq!(t.param_bool("b"), Some(true));
    assert_eq!(t.param_bool("i"), None);
    assert_eq!(t.param_str("missing"), None);
}

#[test]
fn cancel_token_is_shared_between_clones() {
    let a = CancelToken::new();
    let b = a.clone();
    assert!(!b.is_cancelled());
    assert!(b.check().is_ok());
    a.cancel();
    assert!(b.is_cancelled());
    assert!(matches!(b.check(), Err(EngineError::Cancelled)));
}

struct Plain;
impl Engine for Plain {
    fn id(&self) -> &'static str {
        "plain"
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![]
    }
    fn run(&self, _: &Task, _: Progress, _: &CancelToken) -> Result<(), EngineError> {
        Ok(())
    }
}

#[test]
fn default_engine_is_available_and_estimates_from_input_size() {
    assert!(Plain.available());
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::write(&a, vec![0u8; 3_000_000]).unwrap();
    std::fs::write(&b, vec![0u8; 2_000_000]).unwrap();
    let e = Plain.estimate(&task("", vec![a, b, dir.path().join("missing")]));
    assert_eq!(
        e,
        Estimate {
            seconds: 0.1,
            memory_mb: 5 * 4 + 16
        }
    );
    let none = Plain.estimate(&task("", vec![]));
    assert_eq!(
        none,
        Estimate {
            seconds: 0.0,
            memory_mb: 16
        }
    );
}

#[test]
fn engine_errors_render_their_details() {
    let e = EngineError::BadParam {
        name: "q".into(),
        reason: "too big".into(),
    };
    assert_eq!(e.to_string(), "bad parameter `q`: too big");
    let e = EngineError::Input {
        path: "/x.png".into(),
        reason: "broken".into(),
    };
    assert_eq!(e.to_string(), "cannot read /x.png: broken");
    assert_eq!(
        EngineError::Unavailable("gs".into()).to_string(),
        "engine tool not available: gs"
    );
    assert_eq!(
        EngineError::UnsupportedOp("z".into()).to_string(),
        "unsupported operation `z`"
    );
    assert_eq!(EngineError::Cancelled.to_string(), "cancelled");
}

#[test]
fn localized_picks_the_language() {
    let l = Localized {
        ar: "تحويل".into(),
        en: "Convert".into(),
    };
    assert_eq!(l.get(Lang::Ar), "تحويل");
    assert_eq!(l.get(Lang::En), "Convert");
}

#[test]
fn lang_helpers() {
    assert!(Lang::Ar.rtl());
    assert!(!Lang::En.rtl());
    assert_eq!(Lang::Ar.code(), "ar");
    assert_eq!(Lang::En.code(), "en");
    assert_eq!(Lang::default(), Lang::En);
}

#[test]
fn lang_detect_prefers_the_override_then_the_locale() {
    // One test owns the environment so parallel tests do not race on it.
    let vars = ["SBARHIVE_LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"];
    let saved: Vec<_> = vars.iter().map(std::env::var_os).collect();
    let set = |pairs: &[(&str, &str)]| {
        for v in vars {
            unsafe { std::env::remove_var(v) };
        }
        for (k, v) in pairs {
            unsafe { std::env::set_var(k, v) };
        }
    };
    set(&[]);
    assert_eq!(Lang::detect(), Lang::En);
    set(&[("LANG", "ar_SY.UTF-8")]);
    assert_eq!(Lang::detect(), Lang::Ar);
    set(&[("LANG", "ar_SY.UTF-8"), ("LC_ALL", "en_US.UTF-8")]);
    assert_eq!(Lang::detect(), Lang::En);
    set(&[("LC_ALL", ""), ("LANG", "ar")]);
    assert_eq!(Lang::detect(), Lang::Ar, "empty values are skipped");
    set(&[("LANG", "en_US.UTF-8"), ("SBARHIVE_LANG", "ar")]);
    assert_eq!(Lang::detect(), Lang::Ar);
    for (v, old) in vars.iter().zip(saved) {
        match old {
            Some(o) => unsafe { std::env::set_var(v, o) },
            None => unsafe { std::env::remove_var(v) },
        }
    }
}

#[test]
fn every_job_event_maps_to_the_same_ipc_event() {
    let p = PathBuf::from("/a");
    let cases = [
        (
            JobEvent::Started {
                job: 1,
                action_id: "x".into(),
                tasks: 2,
            },
            Event::Started {
                job: 1,
                action_id: "x".into(),
                tasks: 2,
            },
        ),
        (
            JobEvent::Progress {
                job: 1,
                fraction: 0.5,
            },
            Event::Progress {
                job: 1,
                fraction: 0.5,
            },
        ),
        (
            JobEvent::TaskDone {
                job: 1,
                output: p.clone(),
            },
            Event::TaskDone {
                job: 1,
                output: p.clone(),
            },
        ),
        (
            JobEvent::TaskFailed {
                job: 1,
                input: p.clone(),
                error: "e".into(),
            },
            Event::TaskFailed {
                job: 1,
                input: p.clone(),
                error: "e".into(),
            },
        ),
        (
            JobEvent::Finished {
                job: 1,
                ok: 1,
                failed: 2,
                cancelled: true,
            },
            Event::Finished {
                job: 1,
                ok: 1,
                failed: 2,
                cancelled: true,
            },
        ),
    ];
    for (j, e) in cases {
        assert_eq!(Event::from(j), e);
    }
}

#[test]
fn socket_path_lives_in_the_runtime_dir() {
    let p = sbarhive_core::ipc::socket_path();
    assert_eq!(p.file_name().unwrap(), "sbarhive.sock");
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        assert_eq!(p.parent().unwrap(), std::path::Path::new(&dir));
    }
}
