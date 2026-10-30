//! S-107 A2, A4, A5 (backlog/S-107-windows-foreground-verify.md): source scans.
//!
//! - `shell_open.rs::foreground`: the Plain and Topmost steps return
//!   `GetForegroundWindow() == hwnd` read after the call, never the return
//!   value of `SetForegroundWindow`; the settle is at most 50 ms.
//! - `ui.rs`: `show_settings`, `raise_welcome` and `present` each start one
//!   timer of 100 to 200 ms that calls `foreground`, gated by `should_retry`.

fn code_of(file: &str) -> String {
    let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Body of `fn name`: from its signature to the next item-level `fn`.
fn fn_body(code: &str, name: &str) -> String {
    let start = ["fn ", "pub fn ", "pub(crate) fn "]
        .iter()
        .filter_map(|p| {
            let sig = format!("{p}{name}");
            code.match_indices(&sig)
                .find(|(i, _)| {
                    matches!(code[i + sig.len()..].chars().next(), Some('(') | Some('<'))
                })
                .map(|(i, _)| i)
        })
        .min()
        .unwrap_or_else(|| panic!("fn {name} not found"));
    let rest = &code[start + 3..];
    let end = [
        "\nfn ",
        "\npub fn ",
        "\npub(crate) fn ",
        "\n    fn ",
        "\n    pub fn ",
    ]
    .iter()
    .filter_map(|m| rest.find(m))
    .min()
    .unwrap_or(rest.len());
    rest[..end].to_string()
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn millis(body: &str) -> Vec<u64> {
    body.match_indices("from_millis(")
        .filter_map(|(i, m)| {
            let digits: String = body[i + m.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '_')
                .filter(|c| *c != '_')
                .collect();
            digits.parse().ok()
        })
        .collect()
}

#[test]
fn s107_a2_a5_foreground_reads_get_foreground_window_after_each_call() {
    let code = code_of("shell_open.rs");
    let body = fn_body(&code, "foreground");
    assert!(
        body.matches("GetForegroundWindow").count() >= 2,
        "foreground must read GetForegroundWindow after the Plain and the Topmost call"
    );
    let flat = squash(&body);
    assert!(
        flat.contains("GetForegroundWindow()==hwnd")
            || flat.contains("hwnd==GetForegroundWindow()")
            || flat.contains("GetForegroundWindow()==") && flat.contains("==hwnd"),
        "the verified result compares GetForegroundWindow with hwnd"
    );
}

#[test]
fn s107_a2_set_foreground_window_return_value_is_never_the_result() {
    let code = code_of("shell_open.rs");
    let flat = squash(&fn_body(&code, "foreground"));
    for bad in [
        "SetForegroundWindow(hwnd)!=0",
        "SetForegroundWindow(hwnd).as_bool()",
        "SetForegroundWindow(hwnd).0!=0",
        "SetForegroundWindow(hwnd)==0",
    ] {
        assert!(
            !flat.contains(bad),
            "SetForegroundWindow return value used directly: {bad}"
        );
    }
}

#[test]
fn s107_a2_settle_delay_is_at_most_50_ms() {
    let code = code_of("shell_open.rs");
    let body = fn_body(&code, "foreground");
    let ms = millis(&body);
    assert!(
        !ms.is_empty(),
        "foreground must settle before reading GetForegroundWindow"
    );
    assert!(ms.iter().all(|m| *m <= 50), "settle > 50 ms: {ms:?}");
}

fn check_repeat(name: &str) {
    let code = code_of("ui.rs");
    let body = fn_body(&code, name);
    assert!(
        body.contains("should_retry"),
        "{name}: the repeat must be gated by should_retry"
    );
    assert!(
        body.contains("Timer") || body.contains("timer"),
        "{name}: the repeat runs on a UI thread timer"
    );
    assert!(
        body.contains("foreground"),
        "{name}: the timer must call foreground"
    );
    let ms = millis(&body);
    assert!(
        ms.iter().any(|m| (100..=200).contains(m)),
        "{name}: a timer of 100 to 200 ms is required, found {ms:?}"
    );
}

#[test]
fn s107_a4_show_settings_schedules_one_repeat() {
    check_repeat("show_settings");
}

#[test]
fn s107_a4_raise_welcome_schedules_one_repeat() {
    check_repeat("raise_welcome");
}

#[test]
fn s107_a4_present_schedules_one_repeat() {
    check_repeat("present");
}

#[test]
fn s107_a5_ui_calls_should_retry_and_shell_open_reads_foreground_window() {
    assert!(code_of("shell_open.rs").contains("GetForegroundWindow"));
    let ui = code_of("ui.rs");
    assert!(ui.contains("should_retry"));
    assert!(ui.contains("Timer"));
}
