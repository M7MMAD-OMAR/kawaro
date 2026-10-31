//! S-111 (backlog/S-111-hyprland-alt-bind-generator.md): the pure parts, AC1
//! to AC3, AC4 (pure half), AC6 and AC8, and the source and docs scans AC9
//! and AC10. The module `kawaro_core::hyprland_bind` is the contract the
//! story fixes. Paths shown are normalised with `.replace('\\', "/")`.

use kawaro_core::hold::BindState;
use kawaro_core::hyprland_bind::{
    LauncherPlan, MergeError, State, StateInput, Syntax, drop_key_state, hyprland_bind_text,
    merge_block, message, message_keys, normalise_binary, stale_launchers, state_text,
};
use kawaro_core::i18n::Lang;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn golden(name: &str) -> String {
    read(&format!("tests/fixtures/hyprland-bind/{name}")).replace("\r\n", "\n")
}

fn count(hay: &str, needle: &str) -> usize {
    hay.matches(needle).count()
}

// AC1
#[test]
fn ac1_bind_text_equals_the_golden_for_four_cases() {
    let cases = [
        ("Alt", Syntax::Lua, "alt.lua.golden"),
        ("Alt", Syntax::Hyprlang, "alt.conf.golden"),
        ("Super+Z", Syntax::Lua, "super-z.lua.golden"),
        ("Super+Z", Syntax::Hyprlang, "super-z.conf.golden"),
    ];
    for (key, syntax, file) in cases {
        let got = hyprland_bind_text(key, syntax, "/usr/bin/kawaro");
        assert_eq!(got, golden(file), "{key} {file}");
    }
}

#[test]
fn ac1_hook_and_bind_line_counts() {
    let hook = |t: &str| count(t, "drop --other-key");
    let alt_lua = hyprland_bind_text("Alt", Syntax::Lua, "/usr/bin/kawaro");
    assert_eq!(hook(&alt_lua), 1);
    let alt_conf = hyprland_bind_text("Alt", Syntax::Hyprlang, "/usr/bin/kawaro");
    assert_eq!(hook(&alt_conf), 0);
    let binds = alt_conf.lines().filter(|l| l.starts_with("bind")).count();
    assert_eq!(binds, 2, "{alt_conf}");
    for syntax in [Syntax::Lua, Syntax::Hyprlang] {
        let t = hyprland_bind_text("Super+Z", syntax, "/usr/bin/kawaro");
        assert_eq!(hook(&t), 0, "Super+Z has no chord hook");
        assert!(
            t.contains("--at \"$(hyprctl cursorpos)\"")
                || t.contains("--at \\\"$(hyprctl cursorpos)\\\"")
        );
    }
}

// AC2
#[test]
fn ac2_no_local_and_no_home_in_the_text() {
    let rows = [
        ("/usr/bin/kawaro", "/usr/bin/kawaro"),
        ("kawaro", "kawaro"),
        ("/home/u/.local/bin/kawaro", "kawaro"),
        ("", "/usr/bin/kawaro"),
    ];
    for (input, mapped) in rows {
        assert_eq!(normalise_binary(input), mapped, "input [{input}]");
        for key in ["Alt", "Super+Z"] {
            for syntax in [Syntax::Lua, Syntax::Hyprlang] {
                let t = hyprland_bind_text(key, syntax, input);
                assert_eq!(count(&t, ".local"), 0, "[{input}] {key}");
                assert_eq!(count(&t, "$HOME"), 0, "[{input}] {key}");
                assert!(
                    t.contains(&format!("{mapped} drop --hold")),
                    "[{input}] {t}"
                );
            }
        }
    }
}

// AC3
const BLOCK: &str = "-- kawaro:begin\nA\n-- kawaro:end\n";

#[test]
fn ac3_merge_block_six_inputs() {
    // (a) no block: appended once, user text kept.
    let a = merge_block("-- mine\n", BLOCK).unwrap();
    assert_eq!(a, format!("-- mine\n{BLOCK}"));
    assert_eq!(merge_block("", BLOCK).unwrap(), BLOCK);
    // (b) an equal block: unchanged.
    let b_in = format!("x\n{BLOCK}y\n");
    assert_eq!(merge_block(&b_in, BLOCK).unwrap(), b_in);
    // (c) a stale block: only the lines between the markers change.
    let c_in = "-- kawaro:begin\nOLD1\nOLD2\n-- kawaro:end\n";
    assert_eq!(merge_block(c_in, BLOCK).unwrap(), BLOCK);
    // (d) text above and below is byte identical, with odd bytes.
    let d_in = "top \t\n\n-- kawaro:begin\nOLD\n-- kawaro:end\n  bottom no newline";
    let d = merge_block(d_in, BLOCK).unwrap();
    assert_eq!(
        d,
        "top \t\n\n-- kawaro:begin\nA\n-- kawaro:end\n  bottom no newline"
    );
    // (e) begin without end: error.
    let e_in = "top\n-- kawaro:begin\nOLD\n";
    assert_eq!(merge_block(e_in, BLOCK), Err(MergeError::Unbalanced));
    // (f) two blocks: error.
    let f_in = format!("{BLOCK}mid\n{BLOCK}");
    assert_eq!(merge_block(&f_in, BLOCK), Err(MergeError::Duplicate));
}

#[test]
fn ac3_merge_is_idempotent() {
    let inputs = [
        "-- mine\n".to_string(),
        format!("x\n{BLOCK}y\n"),
        "-- kawaro:begin\nOLD\n-- kawaro:end\n".to_string(),
        "top\n-- kawaro:begin\nOLD\n-- kawaro:end\nbottom".to_string(),
        String::new(),
    ];
    for i in &inputs {
        let once = merge_block(i, BLOCK).unwrap();
        let twice = merge_block(&once, BLOCK).unwrap();
        assert_eq!(once, twice, "input {i:?}");
    }
}

#[test]
fn ac3_hyprlang_markers_merge_too() {
    let block = hyprland_bind_text("Alt", Syntax::Hyprlang, "/usr/bin/kawaro");
    let stale = "# mine\n# kawaro:begin\nbind = old\n# kawaro:end\n# tail\n";
    let got = merge_block(stale, &block).unwrap();
    assert_eq!(got, format!("# mine\n{block}# tail\n"));
}

// AC4, pure half
#[test]
fn ac4_stale_launchers_plan() {
    let local = ["kawaro", "kawarod", "kawaro-notes.txt", "other"];
    let plan = stale_launchers(&local, &["kawaro", "kawarod"]);
    assert_eq!(
        plan,
        LauncherPlan {
            remove: vec!["kawaro".into(), "kawarod".into()],
            keep: vec![],
        }
    );
    // No installed counterpart for kawarod: kept, never removed.
    let plan = stale_launchers(&local, &["kawaro"]);
    assert_eq!(plan.remove, vec!["kawaro".to_string()]);
    assert_eq!(plan.keep, vec!["kawarod".to_string()]);
    // Nothing installed: nothing removed.
    let plan = stale_launchers(&local, &[]);
    assert!(plan.remove.is_empty());
    assert_eq!(plan.keep, vec!["kawaro".to_string(), "kawarod".to_string()]);
    // The notes file is in neither list in any case.
    for installed in [&["kawaro", "kawarod", "kawaro-notes.txt"][..], &[]] {
        let p = stale_launchers(&local, installed);
        assert!(
            !p.remove
                .iter()
                .chain(p.keep.iter())
                .any(|n| n == "kawaro-notes.txt")
        );
    }
}

// AC6
fn input(bind: BindState, foreign: bool, denied: bool, fallback: Option<&str>) -> StateInput {
    StateInput {
        key: "Alt".into(),
        bind,
        foreign,
        denied,
        fallback: fallback.map(str::to_string),
    }
}

#[test]
fn ac6_drop_key_state_table() {
    let present = || BindState::Present("ALT_L".into());
    let native = Some(State::Native("Alt".into()));
    let conflict = Some(State::Conflict("Alt".into()));
    let fb = Some(State::Fallback("Super+Z".into()));
    let rows: Vec<(StateInput, Option<State>)> = vec![
        (
            input(present(), true, true, Some("Super+Z")),
            conflict.clone(),
        ),
        (input(present(), true, false, None), conflict.clone()),
        (
            input(BindState::Missing, true, false, Some("Super+Z")),
            conflict.clone(),
        ),
        (input(BindState::Missing, true, true, None), conflict),
        (
            input(present(), false, true, Some("Super+Z")),
            native.clone(),
        ),
        (input(present(), false, false, None), native.clone()),
        (input(present(), false, true, None), native),
        (input(BindState::Missing, false, true, Some("Super+Z")), fb),
        (
            input(BindState::Missing, false, true, None),
            Some(State::Denied),
        ),
        (input(BindState::Unknown, false, false, None), None),
        (input(BindState::Missing, false, false, None), None),
    ];
    for (i, (inp, want)) in rows.iter().enumerate() {
        assert_eq!(&drop_key_state(inp), want, "row {i}");
    }
}

#[test]
fn ac6_state_values_in_english() {
    assert_eq!(
        state_text(&State::Native("Alt".into())),
        "native Alt active"
    );
    assert_eq!(
        state_text(&State::Fallback("Super+Z".into())),
        "fallback Super+Z active"
    );
    assert_eq!(
        state_text(&State::Conflict("Alt".into())),
        "conflict: Alt taken"
    );
    assert_eq!(state_text(&State::Denied), "unavailable (denied)");
}

// AC8
#[test]
fn ac8_message_table_parity() {
    let en = message_keys(Lang::En);
    let ar = message_keys(Lang::Ar);
    let mut a = en.clone();
    let mut b = ar.clone();
    a.sort_unstable();
    b.sort_unstable();
    assert_eq!(a, b, "same key set in both languages");
    for want in [
        "state.native",
        "state.fallback",
        "state.conflict",
        "state.denied",
        "repair.wrote",
        "repair.updated",
        "repair.nothing",
        "repair.conflict",
        "repair.not_hyprland",
        "launcher.removed",
        "launcher.keep",
    ] {
        assert!(en.contains(&want), "missing key {want}");
    }
    let long_dashes = ['\u{2013}', '\u{2014}'];
    for k in &ar {
        let s = message(Lang::Ar, k);
        assert!(!s.is_empty(), "{k}");
        assert!(!s.contains(".local"), "{k}");
        assert!(!s.contains(long_dashes), "{k}");
        assert!(
            !s.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
            "Arabic-Indic digit in {k}"
        );
        assert!(!message(Lang::En, k).contains(long_dashes), "{k}");
    }
    // The English repair strings are fixed by AC5.
    assert_eq!(message(Lang::En, "repair.nothing"), "repair: nothing to do");
    assert_eq!(
        message(Lang::En, "repair.not_hyprland"),
        "repair: not a Hyprland session"
    );
}

// AC9
#[cfg(unix)]
fn sections(text: &str) -> (String, String) {
    // The helper script part is the text before the first "## " heading.
    let idx = text
        .lines()
        .scan(0usize, |pos, l| {
            let start = *pos;
            *pos += l.len() + 1;
            Some((start, l))
        })
        .find(|(_, l)| l.starts_with("## "))
        .map(|(s, _)| s)
        .expect("a second level heading");
    (text[..idx].to_string(), text[idx..].to_string())
}

#[cfg(unix)]
#[test]
fn ac9_readmes_use_repair_and_source() {
    let switch = "[English](README.md) | [العربية](README.ar.md)";
    for name in ["README.md", "README.ar.md"] {
        let text = read(&format!("integrations/linux/hyprland/{name}"));
        assert_eq!(text.lines().next(), Some(switch), "{name} switch line");
        let (_helper, rest) = sections(&text);
        assert_eq!(
            count(&rest, ".local/bin"),
            0,
            "{name}: .local/bin outside the helper section"
        );
        assert!(text.contains("kawaro doctor --repair"), "{name}");
        assert!(
            text.contains("kawaro.lua") && text.contains("kawaro.conf"),
            "{name}"
        );
        assert!(
            text.lines().any(|l| l.trim_start().starts_with("source")),
            "{name}: a source line"
        );
    }
}

// AC10
#[cfg(unix)]
#[test]
fn ac10_changelog_line_and_no_long_dash() {
    let log = read("CHANGELOG.md");
    let lines: Vec<&str> = log.lines().filter(|l| l.contains("S-111")).collect();
    assert_eq!(lines.len(), 1, "one S-111 line");
    assert!(lines[0].contains(" | "), "English and Arabic parts");
    assert!(
        lines[0]
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
    );
    let files = [
        "backlog/S-111-hyprland-alt-bind-generator.md",
        "integrations/linux/hyprland/README.md",
        "integrations/linux/hyprland/README.ar.md",
        "tests/fixtures/hyprland-bind/alt.lua.golden",
        "tests/fixtures/hyprland-bind/alt.conf.golden",
        "tests/fixtures/hyprland-bind/super-z.lua.golden",
        "tests/fixtures/hyprland-bind/super-z.conf.golden",
        "crates/core/src/hyprland_bind.rs",
    ];
    for f in files {
        let t = read(f);
        assert!(!t.contains(['\u{2013}', '\u{2014}']), "long dash in {f}");
    }
    assert!(!lines[0].contains(['\u{2013}', '\u{2014}']));
}
