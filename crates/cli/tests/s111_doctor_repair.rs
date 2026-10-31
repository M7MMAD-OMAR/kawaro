//! S-111 (backlog/S-111-hyprland-alt-bind-generator.md): AC4 (the repair
//! half), AC5, AC7 and the CLI half of AC8. A stub `hyprctl` first on PATH
//! logs every call to `$STUB_LOG` and prints `$STUB_BINDS` (a JSON file, or
//! `[]`) for `binds -j`. HOME and XDG_CONFIG_HOME are temp folders; no
//! network, no gpg and no real compositor are involved.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_kawaro");

const FOREIGN: &str = r#"[{"key":"ALT_L","modmask":0,"dispatcher":"exec","arg":"foot","description":"mine","non_consuming":false}]"#;
const OWNED: &str = r#"[{"key":"ALT_L","modmask":0,"dispatcher":"exec","arg":"/usr/bin/kawaro drop --hold --at x","description":"Kawaro: hold Alt to drop","non_consuming":true}]"#;
const FALLBACK: &str = r#"[{"key":"Z","modmask":64,"dispatcher":"global","arg":"io.github.m7mmad_omar.Kawaro:drop","description":""}]"#;

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        let dir = tempfile::tempdir().unwrap();
        for d in ["home", "cfg/hypr", "cfg/kawaro", "stub", "run", "inst"] {
            std::fs::create_dir_all(dir.path().join(d)).unwrap();
        }
        let stub = dir.path().join("stub/hyprctl");
        std::fs::write(
            &stub,
            "#!/bin/sh\necho \"$*\" >> \"$STUB_LOG\"\nif [ \"$1\" = binds ]; then cat \"$STUB_BINDS\"; fi\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(dir.path().join("binds.json"), "[]").unwrap();
        // The user's own Hyprland config: never edited by the repair.
        std::fs::write(dir.path().join("cfg/hypr/hyprland.lua"), "-- user config\n").unwrap();
        Env { dir }
    }
    fn p(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn binds(&self, json: &str) {
        std::fs::write(self.p("binds.json"), json).unwrap();
    }
    fn calls(&self) -> Vec<String> {
        std::fs::read_to_string(self.p("log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }
    fn reloads(&self) -> usize {
        self.calls()
            .iter()
            .filter(|l| l.as_str() == "reload")
            .count()
    }
    fn run(&self, args: &[&str], lang: &str, session: bool) -> Output {
        let path = format!("{}:{}", self.p("stub").display(), self.p("inst").display());
        let mut c = Command::new(BIN);
        c.args(args)
            .env_clear()
            .env("HOME", self.p("home"))
            .env("XDG_CONFIG_HOME", self.p("cfg"))
            .env("XDG_RUNTIME_DIR", self.p("run"))
            .env("PATH", path)
            .env("LANG", lang)
            .env("STUB_LOG", self.p("log"))
            .env("STUB_BINDS", self.p("binds.json"));
        if session {
            c.env("HYPRLAND_INSTANCE_SIGNATURE", "test");
        }
        c.output().unwrap()
    }
    fn repair(&self) -> Output {
        self.run(&["doctor", "--repair"], "en_US.UTF-8", true)
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn lines_with<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    text.lines().filter(|l| l.starts_with(prefix)).collect()
}

const STALE: &str = "-- kawaro:begin\nhl.bind(\"ALT_L\", hl.dsp.exec_cmd(\"$HOME/.local/bin/kawaro drop --hold\"), {})\n-- kawaro:end\n";

// AC5 a, b, c
#[test]
fn ac5_repair_creates_updates_and_settles() {
    let e = Env::new();
    let kfile = e.p("cfg/hypr/kawaro.lua");
    let user_before = std::fs::read(e.p("cfg/hypr/hyprland.lua")).unwrap();
    // (a) no file yet.
    let o = e.repair();
    assert!(o.status.success(), "{}", out(&o));
    assert!(
        out(&o).lines().any(|l| l == "repair: wrote 1 block"),
        "{}",
        out(&o)
    );
    let text = std::fs::read_to_string(&kfile).unwrap();
    assert!(text.contains("-- kawaro:begin") && text.contains("-- kawaro:end"));
    assert_eq!(text.matches(".local").count(), 0);
    assert_eq!(e.reloads(), 1, "{:?}", e.calls());
    // (c) a current file: nothing to do, mtime unchanged, no reload.
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    std::fs::File::options()
        .write(true)
        .open(&kfile)
        .unwrap()
        .set_modified(old)
        .unwrap();
    let before = std::fs::metadata(&kfile).unwrap().modified().unwrap();
    let o = e.repair();
    assert!(o.status.success());
    assert!(
        out(&o).lines().any(|l| l == "repair: nothing to do"),
        "{}",
        out(&o)
    );
    assert_eq!(
        std::fs::metadata(&kfile).unwrap().modified().unwrap(),
        before
    );
    assert_eq!(e.reloads(), 1, "no new reload: {:?}", e.calls());
    // (b) a stale block with user text around it.
    std::fs::write(&kfile, format!("-- above\n{STALE}-- below\n")).unwrap();
    let o = e.repair();
    assert!(o.status.success(), "{}", out(&o));
    assert!(
        out(&o).lines().any(|l| l == "repair: updated 1 block"),
        "{}",
        out(&o)
    );
    let text = std::fs::read_to_string(&kfile).unwrap();
    assert_eq!(text.matches(".local").count(), 0);
    assert!(text.starts_with("-- above\n-- kawaro:begin\n"));
    assert!(text.ends_with("-- kawaro:end\n-- below\n"));
    assert_eq!(e.reloads(), 2, "{:?}", e.calls());
    // The user's own config is never edited.
    assert_eq!(
        std::fs::read(e.p("cfg/hypr/hyprland.lua")).unwrap(),
        user_before
    );
}

#[test]
fn ac5_repair_uses_hyprlang_without_a_lua_config() {
    let e = Env::new();
    std::fs::remove_file(e.p("cfg/hypr/hyprland.lua")).unwrap();
    std::fs::write(e.p("cfg/hypr/hyprland.conf"), "# user\n").unwrap();
    let o = e.repair();
    assert!(
        out(&o).lines().any(|l| l == "repair: wrote 1 block"),
        "{}",
        out(&o)
    );
    let text = std::fs::read_to_string(e.p("cfg/hypr/kawaro.conf")).unwrap();
    assert!(text.contains("# kawaro:begin") && text.contains("# kawaro:end"));
    assert!(!e.p("cfg/hypr/kawaro.lua").exists());
    assert_eq!(
        std::fs::read_to_string(e.p("cfg/hypr/hyprland.conf")).unwrap(),
        "# user\n"
    );
}

// AC5 d
#[test]
fn ac5_repair_reports_a_foreign_bind_and_touches_nothing() {
    let e = Env::new();
    let kfile = e.p("cfg/hypr/kawaro.lua");
    std::fs::write(&kfile, STALE).unwrap();
    e.binds(FOREIGN);
    let o = e.repair();
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    assert!(
        out(&o).lines().any(|l| l == "conflict: Alt taken"),
        "{}",
        out(&o)
    );
    assert_eq!(std::fs::read_to_string(&kfile).unwrap(), STALE);
    assert_eq!(e.reloads(), 0, "{:?}", e.calls());
}

#[test]
fn ac5_repair_outside_hyprland_is_a_no_op() {
    let e = Env::new();
    let o = e.run(&["doctor", "--repair"], "en_US.UTF-8", false);
    assert!(o.status.success());
    assert!(
        out(&o)
            .lines()
            .any(|l| l == "repair: not a Hyprland session"),
        "{}",
        out(&o)
    );
    assert!(!e.p("cfg/hypr/kawaro.lua").exists());
    assert_eq!(e.reloads(), 0);
}

// AC4, repair half
fn installed_kawaro(e: &Env) {
    for n in ["kawaro", "kawarod"] {
        let f = e.p("inst").join(n);
        std::fs::write(&f, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn local_bin(e: &Env) -> PathBuf {
    let d = e.p("home/.local/bin");
    std::fs::create_dir_all(&d).unwrap();
    for n in ["kawaro", "kawarod", "kawaro-notes.txt"] {
        std::fs::write(d.join(n), "x").unwrap();
    }
    d
}

#[test]
fn ac4_repair_removes_only_stale_launchers() {
    let e = Env::new();
    let d = local_bin(&e);
    installed_kawaro(&e);
    let o = e.repair();
    assert!(o.status.success(), "{}", out(&o));
    assert!(!d.join("kawaro").exists());
    assert!(!d.join("kawarod").exists());
    assert!(d.join("kawaro-notes.txt").exists(), "user file kept");
    let text = out(&o).replace('\\', "/");
    let removed = lines_with(&text, "removed: ");
    assert_eq!(removed.len(), 2, "{text}");
    assert!(removed.iter().any(|l| l.ends_with(".local/bin/kawaro")));
    assert!(removed.iter().any(|l| l.ends_with(".local/bin/kawarod")));
}

#[test]
fn ac4_repair_keeps_a_launcher_with_no_installed_counterpart() {
    let e = Env::new();
    let d = local_bin(&e);
    let o = e.repair();
    assert!(o.status.success(), "{}", out(&o));
    assert!(d.join("kawaro").exists() && d.join("kawarod").exists());
    assert!(d.join("kawaro-notes.txt").exists());
    let text = out(&o);
    assert!(
        !lines_with(&text, "keep: no installed kawaro found").is_empty(),
        "{text}"
    );
    assert_eq!(lines_with(&text, "removed: ").len(), 0);
}

// AC7
fn state_line(e: &Env, lang: &str) -> Vec<String> {
    let o = e.run(&["doctor"], lang, true);
    let text = out(&o);
    let prefix = if lang.starts_with("ar") {
        "حالة مفتاح الإفلات:"
    } else {
        "drop key state:"
    };
    lines_with(&text, prefix)
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn denied(e: &Env) {
    std::fs::write(e.p("cfg/kawaro/config.toml"), "portal_shortcut = false\n").unwrap();
}

#[test]
fn ac7_doctor_prints_the_four_states() {
    let cases: [(&str, bool, &str); 4] = [
        (OWNED, false, "drop key state: native Alt active"),
        (FALLBACK, false, "drop key state: fallback Super+Z active"),
        (FOREIGN, false, "drop key state: conflict: Alt taken"),
        ("[]", true, "drop key state: unavailable (denied)"),
    ];
    for (json, deny, want) in cases {
        let e = Env::new();
        e.binds(json);
        if deny {
            denied(&e);
        }
        let got = state_line(&e, "en_US.UTF-8");
        assert_eq!(got, vec![want.to_string()], "{want}");
        // The S-102 line stays, once.
        let o = e.run(&["doctor"], "en_US.UTF-8", true);
        assert_eq!(lines_with(&out(&o), "drop key: ").len(), 1);
    }
}

#[test]
fn ac7_doctor_prints_no_state_line_without_a_state() {
    // Positive control: a state exists with an owned bind, so exactly one line.
    let c = Env::new();
    c.binds(OWNED);
    assert_eq!(state_line(&c, "en_US.UTF-8").len(), 1);
    let e = Env::new(); // binds [], not denied
    assert!(state_line(&e, "en_US.UTF-8").is_empty());
    let o = e.run(&["doctor"], "en_US.UTF-8", true);
    assert_eq!(lines_with(&out(&o), "drop key: ").len(), 1);
    // Binds unreadable: failing hyprctl.
    let e = Env::new();
    std::fs::write(e.p("stub/hyprctl"), "#!/bin/sh\nexit 1\n").unwrap();
    assert!(state_line(&e, "en_US.UTF-8").is_empty());
}

// AC8, CLI half
#[test]
fn ac8_arabic_doctor_states_equal_the_golden() {
    let cases: [(&str, bool, &str); 4] = [
        (OWNED, false, "حالة مفتاح الإفلات: Alt الأصلي فعّال"),
        (FALLBACK, false, "حالة مفتاح الإفلات: البديل Super+Z فعّال"),
        (FOREIGN, false, "حالة مفتاح الإفلات: تعارض: Alt مستخدم"),
        ("[]", true, "حالة مفتاح الإفلات: غير متاح (مرفوض)"),
    ];
    for (json, deny, want) in cases {
        let e = Env::new();
        e.binds(json);
        if deny {
            denied(&e);
        }
        assert_eq!(state_line(&e, "ar_EG.UTF-8"), vec![want.to_string()]);
        assert!(!want.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)));
    }
}

#[test]
fn ac8_arabic_repair_message_has_no_local_or_long_dash() {
    let e = Env::new();
    let o = e.run(&["doctor", "--repair"], "ar_EG.UTF-8", true);
    let text = out(&o);
    assert!(
        text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "{text}"
    );
    assert!(!text.contains(".local"));
    assert!(!text.contains(['\u{2013}', '\u{2014}']));
}
