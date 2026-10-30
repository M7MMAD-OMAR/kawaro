//! S-108 part A (backlog/S-108-s096b-hive-cells.md), the daemon side that
//! needs no window: AC1 centre and message as two values, AC2 no side
//! effects, AC3 the greyed cells, AC4 the choice and the pending files, AC5
//! the request cell and the opener.
//!
//! Hooks these tests pin:
//! - module `crate::plan` (file `plan.rs`, pure, re-exported by `server`):
//!   `plan(&Outcome, &[PathBuf], &State) -> HivePlan`, with `HivePlan {
//!   centre: String, message: String, suggestion: Option<String>, cells:
//!   Vec<HiveCell>, pending: Option<Vec<PathBuf>> }`. `centre` and `message`
//!   never hold a newline. For a Supported plan `message` is empty. `pending`
//!   holds the supported files for Supported and Mixed, `None` otherwise.
//! - `enum Choice { Run(String), Request(String), Ignore }` (Debug, Clone,
//!   PartialEq, Eq) and `choice(&str) -> Choice`: `skipped:<n>` and `more`
//!   give `Ignore`, `request:<ext>` gives `Request(request_url(ext))`, any
//!   other id gives `Run(id)`.
//! - `on_choice(&str, &dyn Fn(&str)) -> Choice`: `choice`, plus exactly one
//!   call of the opener with the URL for `Request`, none otherwise. The
//!   `on_choose` handler in `ui.rs` goes through it.
//! - cell ids: the real action id, `skipped:<n>` (n from 0), `more`,
//!   `request:<ext>`. Codes: the upper case extension (`?` when there is
//!   none), `+K`, `REQ`. Labels: `skipped_label(ext, reason, lang)`,
//!   `more_label(k, lang)`, `request_label(lang)`. `reason`: the file's own
//!   reason for a skipped cell, `more_label` for `+K`, `request_hover(lang)`
//!   for `REQ`. `enabled` is false only for skipped and `+K` cells.
//! - `kawaro_ui::HiveCell` gains `enabled: bool` and `reason: String`.
//! - `notify_logic::open_url_command(Os, &str) -> (String, Vec<String>)`.

use crate::notify_logic::{Os, open_url_command};
use crate::plan::{Choice, choice, on_choice, plan};
use crate::state::State;
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{
    centre_text, missing_engine_message, more_label, request_hover, request_label, request_url,
    skipped_label, unsupported_message,
};
use kawaro_ui::HiveCell;
use std::cell::Cell;
use std::path::PathBuf;

const ENGINES: [&str; 5] = ["image", "ffmpeg", "office", "ghostscript", "pdf"];

fn state(lang: Lang, engines: &[&str]) -> State {
    let mut st = State::new(Config::default(), &[]);
    st.lang = lang;
    st.available = engines.iter().map(|e| (*e).to_string()).collect();
    st
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

fn planned(st: &State, names: &[&str]) -> (Vec<PathBuf>, crate::plan::HivePlan) {
    let files = paths(names);
    let outcome = st.classify(&files);
    let p = plan(&outcome, &files, st);
    (files, p)
}

fn is_extra(id: &str) -> bool {
    id.starts_with("skipped:") || id == "more" || id.starts_with("request:")
}

fn real(cells: &[HiveCell]) -> Vec<&HiveCell> {
    cells.iter().filter(|c| !is_extra(&c.id)).collect()
}

fn skipped(cells: &[HiveCell]) -> Vec<&HiveCell> {
    cells
        .iter()
        .filter(|c| c.id.starts_with("skipped:"))
        .collect()
}

fn more(cells: &[HiveCell]) -> Option<&HiveCell> {
    cells.iter().find(|c| c.id == "more")
}

fn req(cells: &[HiveCell]) -> Vec<&HiveCell> {
    cells.iter().filter(|c| c.code == "REQ").collect()
}

fn more_count(cells: &[HiveCell]) -> usize {
    more(cells).map_or(0, |c| c.code.trim_start_matches('+').parse().unwrap())
}

// AC1

#[test]
fn ac1_plan_gives_centre_and_message_as_two_values_in_english() {
    let st = state(Lang::En, &ENGINES);
    let (_, p) = planned(&st, &["a.xyz"]);
    assert_eq!(p.centre, "File .xyz");
    assert_eq!(p.message, "This format is not supported yet");
    assert!(!p.centre.contains('\n') && !p.message.contains('\n'));
}

#[test]
fn ac1_plan_gives_centre_and_message_as_two_values_in_arabic() {
    let st = state(Lang::Ar, &ENGINES);
    let (files, p) = planned(&st, &["a.xyz"]);
    assert_eq!(p.centre, "ملف \u{200e}.xyz");
    assert_eq!(p.centre, centre_text(&files, Lang::Ar));
    assert_eq!(p.message, "هذه الصيغة غير مدعومة بعد");
    assert!(!p.centre.contains('\n') && !p.message.contains('\n'));
}

#[test]
fn ac1_a_supported_plan_has_no_message() {
    let st = state(Lang::En, &ENGINES);
    let (_, p) = planned(&st, &["a.png", "b.png"]);
    assert_eq!(p.message, "");
    assert!(!p.cells.is_empty());
}

// AC2

#[test]
fn ac2_missing_engine_message_holds_the_tool_name_exactly_once() {
    let st = state(Lang::En, &["image"]);
    let (_, p) = planned(&st, &["a.mp4"]);
    assert_eq!(p.message, missing_engine_message("FFmpeg", Lang::En));
    assert_eq!(p.message.matches("FFmpeg").count(), 1, "{}", p.message);
}

#[test]
fn ac2_choices_other_than_the_request_make_no_opener_call() {
    let calls = Cell::new(0);
    let spy = |_: &str| calls.set(calls.get() + 1);
    for id in ["skipped:0", "more", "image.rotate-90"] {
        on_choice(id, &spy);
    }
    assert_eq!(calls.get(), 0);
}

// AC3

fn cells_of(names: &[&str]) -> Vec<HiveCell> {
    planned(&state(Lang::En, &ENGINES), names).1.cells
}

#[test]
fn ac3_one_skipped_file_among_supported_ones_gives_one_xyz_cell() {
    let st = state(Lang::En, &ENGINES);
    let (_, p) = planned(&st, &["a.png", "b.png", "c.xyz"]);
    let expected: Vec<String> = st
        .matching(&paths(&["a.png", "b.png"]))
        .into_iter()
        .map(|a| a.id)
        .collect();
    let got: Vec<String> = real(&p.cells).iter().map(|c| c.id.clone()).collect();
    assert_eq!(got, expected, "all real actions of the supported files");
    let s = skipped(&p.cells);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].code, "XYZ");
    assert_eq!(s[0].id, "skipped:0");
    assert!(more(&p.cells).is_none());
}

#[test]
fn ac3_the_skipped_cells_table() {
    // (files, individual skipped cells, `+K` count)
    let rows: [(usize, usize, usize); 4] = [(1, 1, 0), (6, 6, 0), (7, 6, 1), (9, 6, 3)];
    for (n, shown, k) in rows {
        let names: Vec<String> = (0..n).map(|i| format!("f{i}.xyz")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let cells = cells_of(&refs);
        assert_eq!(skipped(&cells).len(), shown, "{n} files");
        assert_eq!(more_count(&cells), k, "{n} files");
        for (i, c) in skipped(&cells).iter().enumerate() {
            assert_eq!(c.id, format!("skipped:{i}"));
            assert_eq!(c.code, "XYZ");
        }
        if k > 0 {
            let m = more(&cells).unwrap();
            assert_eq!(m.code, format!("+{k}"));
            assert_eq!(m.label, more_label(k, Lang::En));
            assert!(!m.enabled);
        }
        assert_eq!(skipped(&cells).len() + more_count(&cells), n);
    }
}

#[test]
fn ac3_a_file_without_extension_gets_a_question_mark_code() {
    let cells = cells_of(&["README"]);
    let s = skipped(&cells);
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].code, "?");
}

#[test]
fn ac3_skipped_cells_are_disabled_and_have_their_own_reason() {
    let st = state(Lang::En, &["image"]);
    let (_, p) = planned(&st, &["a.png", "b.xyz", "c.mp4"]);
    let s = skipped(&p.cells);
    assert_eq!(s.len(), 2);
    assert!(s.iter().all(|c| !c.enabled));
    assert_eq!(s[0].code, "XYZ");
    assert_eq!(s[0].reason, unsupported_message(Lang::En));
    assert_eq!(s[0].label, skipped_label("xyz", &s[0].reason, Lang::En));
    assert_eq!(s[1].code, "MP4");
    assert_eq!(s[1].reason, missing_engine_message("FFmpeg", Lang::En));
    assert!(real(&p.cells).iter().all(|c| c.enabled));
}

#[test]
fn ac3_cap_rows_keep_every_real_action_and_stay_within_18_cells() {
    let st = state(Lang::En, &ENGINES);
    let n_real = st.matching(&paths(&["a.png"])).len();
    assert!(n_real >= 10, "the registry has {n_real} image actions");
    let mut names = vec!["a.png".to_string()];
    names.extend((0..9).map(|i| format!("f{i}.xyz")));
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (_, p) = planned(&st, &refs);
    let ids: Vec<String> = st
        .matching(&paths(&["a.png"]))
        .into_iter()
        .map(|a| a.id)
        .collect();
    let got: Vec<String> = real(&p.cells).iter().map(|c| c.id.clone()).collect();
    assert_eq!(got, ids, "real actions are never dropped");
    assert!(p.cells.len() <= 18, "{} cells", p.cells.len());
    // Supported files (1) plus the skipped ones represented equal the selection.
    assert_eq!(1 + skipped(&p.cells).len() + more_count(&p.cells), 10);
    if skipped(&p.cells).len() < 9 {
        assert!(more(&p.cells).is_some(), "the rest collapse into `+K`");
    }
    // REQ is omitted only when real actions plus one `+K` fill the 18.
    assert_eq!(req(&p.cells).len(), usize::from(n_real + 2 <= 18));
}

#[test]
fn ac3_pending_holds_only_the_supported_files() {
    let st = state(Lang::En, &ENGINES);
    let (_, p) = planned(&st, &["a.png", "b.png", "c.xyz"]);
    assert_eq!(p.pending, Some(paths(&["a.png", "b.png"])));
    let (_, none) = planned(&st, &["a.xyz"]);
    assert_eq!(none.pending, None);
}

// AC4

#[test]
fn ac4_skipped_and_more_cells_choose_nothing() {
    assert_eq!(choice("skipped:0"), Choice::Ignore);
    assert_eq!(choice("skipped:5"), Choice::Ignore);
    assert_eq!(choice("more"), Choice::Ignore);
}

#[test]
fn ac4_a_real_action_id_runs() {
    assert_eq!(
        choice("image.rotate-90"),
        Choice::Run("image.rotate-90".into())
    );
}

// AC5

#[test]
fn ac5_every_unsupported_plan_has_exactly_one_request_cell() {
    for lang in [Lang::En, Lang::Ar] {
        let st = state(lang, &["image"]);
        let selections: [(&str, &[&str]); 3] = [
            ("unsupported", &["a.xyz"]),
            ("missing engine", &["a.mp4"]),
            ("mixed", &["a.png", "b.xyz"]),
        ];
        for (what, names) in selections {
            let (_, p) = planned(&st, names);
            let r = req(&p.cells);
            assert_eq!(r.len(), 1, "{what} {lang:?}");
            assert_eq!(r[0].label, request_label(lang));
            assert_eq!(r[0].reason, request_hover(lang));
            assert!(r[0].enabled);
            assert!(r[0].code.chars().count() <= 16 && !r[0].code.contains(' '));
        }
    }
}

#[test]
fn ac5_the_request_cell_carries_the_first_skipped_extension() {
    let st = state(Lang::En, &["image"]);
    let (_, p) = planned(&st, &["a.xyz"]);
    assert_eq!(req(&p.cells)[0].id, "request:xyz");
    let (_, mixed) = planned(&st, &["a.png", "b.abc", "c.xyz"]);
    assert_eq!(req(&mixed.cells)[0].id, "request:abc");
    let (_, engine) = planned(&st, &["a.mp4"]);
    assert_eq!(req(&engine.cells)[0].id, "request:mp4");
}

#[test]
fn ac5_a_supported_plan_has_no_request_cell() {
    let st = state(Lang::En, &ENGINES);
    let (_, p) = planned(&st, &["a.png"]);
    assert!(req(&p.cells).is_empty());
    assert!(skipped(&p.cells).is_empty() && more(&p.cells).is_none());
}

#[test]
fn ac5_choice_of_the_request_cell_gives_the_request_url() {
    assert_eq!(choice("request:xyz"), Choice::Request(request_url("xyz")));
    // An unsafe extension goes through the same sanitising as `request_url`.
    assert_eq!(
        choice("request:../x y"),
        Choice::Request(request_url("../x y"))
    );
}

#[test]
fn ac5_the_opener_is_called_once_per_request_choice_only() {
    let st = state(Lang::En, &["image"]);
    let (_, p) = planned(&st, &["a.xyz"]);
    let urls = std::cell::RefCell::new(Vec::<String>::new());
    let spy = |u: &str| urls.borrow_mut().push(u.to_string());
    assert!(urls.borrow().is_empty(), "plan opens nothing");
    assert_eq!(
        on_choice(&req(&p.cells)[0].id, &spy),
        Choice::Request(request_url("xyz"))
    );
    assert_eq!(urls.borrow().as_slice(), [request_url("xyz")]);
    on_choice("request:xyz", &spy);
    assert_eq!(urls.borrow().len(), 2);
    on_choice("skipped:0", &spy);
    on_choice("more", &spy);
    assert_eq!(urls.borrow().len(), 2);
}

#[test]
fn ac5_open_url_command_keeps_the_url_as_one_argument() {
    let url = request_url("xyz");
    assert!(url.contains('&'));
    assert_eq!(
        open_url_command(Os::Linux, &url),
        (concat!("xdg", "-open").to_string(), vec![url.clone()])
    );
    assert_eq!(
        open_url_command(Os::Windows, &url),
        ("explorer.exe".to_string(), vec![url])
    );
}

// AC10, unit part

#[test]
fn ac10_plan_for_1000_paths_takes_20_ms_or_less() {
    let st = state(Lang::En, &ENGINES);
    let files: Vec<PathBuf> = (0..1000)
        .map(|i| PathBuf::from(format!("f{i}.{}", if i % 2 == 0 { "png" } else { "xyz" })))
        .collect();
    let outcome = st.classify(&files);
    let start = std::time::Instant::now();
    let p = plan(&outcome, &files, &st);
    let took = start.elapsed();
    assert!(!p.cells.is_empty());
    assert!(took.as_millis() <= 20, "{took:?}");
}
