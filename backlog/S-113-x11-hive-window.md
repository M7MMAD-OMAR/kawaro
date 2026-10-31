# S-113 The hive window at the pointer on X11, decided by pure functions
Status: review

## Story
As a user on an X11 session, I want the hive to open at my pointer, kept inside the screen, so that it appears where I work. Slice 1 of S-015 (decisions 1, 4 and 5 there). Everything here runs in the fast loop with no display; the real `XQueryPointer` call is checked by S-114.

## Acceptance criteria
- AC1 (session detection): a pure `session::detect(wayland_display, display, is_windows)` returns `Wayland` if `WAYLAND_DISPLAY` is non-empty, else `X11` if `DISPLAY` is non-empty, else `Windows` if `is_windows`, else `None`. Table of 8 rows, including: both set gives `Wayland`; only `DISPLAY=:0` gives `X11`; `DISPLAY=""` and `WAYLAND_DISPLAY=""` give `None`; `is_windows` with neither gives `Windows`; `is_windows` with `DISPLAY` set gives `X11`.
- AC2 (placement): a pure function takes the pointer, the root size and the cells, and returns `layout::plan_at` output (no second copy of the maths) with scale 120. On a 1280 x 800 root: pointer (400, 500) puts the centre cell centre at (400, 500) within 1 px; pointers (0, 0), (1279, 799), (100, 700) and (1200, 80) give a window box with x >= 0, y >= 0, x + w <= 1280, y + h <= 800, and a cell count equal to `plan_at` for the same clamped anchor. A pointer outside the root (-5, 900) is clamped like (0, 799), never a panic.
- AC3 (pointer seam): a trait `PointerSource` returns `Option<Pointer { x, y, root_w, root_h }>`. With a fake returning `Some`, the plan equals AC2 for that point. With a fake returning `None` the plan equals `plan_at` with no anchor (centred on a 1280 x 800 root), and exactly 1 stderr line `hive pointer: unavailable` is produced, not repeated for 5 shows in a row.
- AC4 (choice line): on a non Wayland session the process prints `hive surface choice: window, not a Wayland session` once. A pure dedupe helper given the previous line returns `Some(line)` for the first of 20 identical calls and `None` for the other 19; a changed reason prints again. The Wayland lines (existing tests) are unchanged.
- AC5 (window spec): a pure `x11_window_spec()` returns no decorations, always on top, not transparent. The Slint hive gets an opaque `surface-0` plate on X11 only (decision 1), colour read from `design/tokens.json` through the existing token global; on the layer and Windows paths the plate is absent (S-074 tests pass unedited). A headless render of the X11 hive at 1280 x 800 has the pixel at the window box corner equal to the `surface-0` value of the current scheme, light and dark.
- AC6 (wiring): `Hive::show` on a session of `X11` calls the AC2 function once and sets the window position before `show()`; on `Wayland` and `Windows` it calls it 0 times (counter in the fake).
- AC7 (dependency): `x11rb` 0.13.2 is a direct dependency of `crates/ui` (Linux only) with no extra feature; `Cargo.lock` has the same number of `[[package]]` entries before and after. `docs/engines.md` has a row for it: version, MIT or Apache 2.0, ui (Linux), purpose, already in the tree through winit.
- AC8 (mutants): the real `XQueryPointer` adapter is excluded in `.cargo/mutants.toml` with the reason "live check S-114"; AC1 to AC4 functions are not excluded.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | 8 row table, assert the exact enum value | unit |
| AC2 | 5 pointers on 1280 x 800, assert box bounds, centre cell centre and cell count against `plan_at` | unit |
| AC3 | Two fakes, assert plans and the captured stderr lines (1, then 1 after 5 shows) | unit |
| AC4 | Feed 20 identical lines, count `Some`; feed a changed reason, expect `Some` | unit |
| AC5 | Spec struct values; headless render and corner pixel against `tokens.json` in both schemes | unit, snapshot |
| AC6 | Fake source with a call counter, per session value | unit |
| AC7 | Test reads `crates/ui/Cargo.toml`, `docs/engines.md` and counts `[[package]]` in `Cargo.lock` (before value pinned in the test) | unit |
| AC8 | Test reads `.cargo/mutants.toml` for the exclusion and the reason text | unit |

## Out of scope
Focus, Esc, live geometry and timing (S-114); the drop cell (S-115); several monitors; scale other than 1; Shape clipping.

## Review log
- 2026-10-31, planner: split from S-015. Reuses `plan_at` as the Windows window does; the Windows file is not changed. Draft awaiting the manager.
- 2026-10-31, manager: S-015 settled decisions and S-113 approved. S-114 and S-115 stay draft.
- 2026-10-31, tester: tests written from AC1 to AC8 in `crates/ui/src/s113_tests.rs` (AC1 to AC6, registered in `lib.rs` as `#[cfg(all(test, target_os = "linux"))] mod s113_tests;`, the one mechanical edit to an existing file) and `crates/ui/tests/s113_sources.rs` (AC7, AC8). Hooks the implementer must create (full signatures in the header of `s113_tests.rs`):
  - `crates/ui/src/session.rs`: `Session { Wayland, X11, Windows, None }`, `detect(Option<&str>, Option<&str>, bool)`.
  - `crates/ui/src/x11.rs` (pure): `Pointer { x: i32, y: i32, root_w: u32, root_h: u32 }`, `trait PointerSource { fn pointer(&self) -> Option<Pointer> }`, `plan_for_pointer`, `PointerLog`, `plan_with` (prints `hive pointer: unavailable` once through a sink closure), `WindowSpec`, `x11_window_spec`, `session_choice_line`.
  - `crates/ui/src/x11_pointer.rs`: the live `XQueryPointer` adapter, excluded in `.cargo/mutants.toml` with the text `live check S-114`.
  - `overlay::next_line(previous: Option<&str>, line: &str) -> Option<String>`.
  - `hive.slint`: `in property <bool> x11-plate`, opaque `Tokens.surface-0`.
  - `Hive::set_session(session, Box<dyn PointerSource>)`, `Hive::x11_position() -> Option<(i32, i32)>`, and `show` on X11 calling `plan_with` once, setting the position and `x11-plate`.
  - `crates/ui/Cargo.toml` Linux table: `x11rb = "0.13.2"` (no features), `Cargo.lock` kept at 674 packages with `x11rb` in the `kawaro-ui` dependencies, a `docs/engines.md` row.
- 2026-10-31, tester: failing run (fast loop, `systemd-run` scope, `-j 1`, 2 test threads).
  - `cargo test -p kawaro-ui --lib s113`: does not compile, `error[E0432]: unresolved import crate::session` and `crate::x11` (the hooks do not exist yet). The signatures of `next_line`, `Hive::set_session`, `Hive::x11_position`, `get_x11_plate` are checked only once those modules exist.
  - `cargo test -p kawaro-ui --test s113_sources`: 4 failed, 0 passed: `ac7_x11rb_is_a_plain_linux_only_direct_dependency_of_ui` (no x11rb in the Linux table), `ac7_cargo_lock_keeps_its_package_count_and_x11rb_0_13_2` (kawaro-ui does not depend on x11rb), `ac7_engines_md_has_the_x11rb_row` (no row), `ac8_only_the_live_pointer_adapter_is_excluded_from_mutants` (no `live check S-114` reason).
  - `cargo fmt --check` clean. Clippy `-D warnings` on the new files runs after the hooks exist.
  - Existing tests the implementer must adapt: none by content. Mechanical: none expected (S-074 tests must pass unedited).
- 2026-10-31, implementer: STOPPED, not committed. Everything is built (session.rs, x11.rs, x11_pointer.rs, `next_line`, `x11-plate`, `set_session`, `x11_position`, the X11 window attributes in `token_hook`, the Linux only `x11rb` dependency with an unchanged Cargo.lock count, the engines.md row, the mutants exclusion), and 265 of 266 lib tests pass. Two test problems:
  - `s113_tests::ac2_every_pointer_keeps_the_box_on_the_root_and_matches_plan_at` cannot pass with `anchor_tests` (S-019c AC4). At pointer (1279, 799) on 1280 x 800 `plan_at` itself returns `Placement { x: 888, y: 439, destination: (393, 362) }`, so the box ends at x = 1281. The test asserts both `x + w <= 1280` and equality with the unchanged `plan_at` shape, while `anchor_tests` pins the centre cell within 0.5 px of the clamped anchor. The overflow comes from rounding the position to the nearest pixel and the width up (the true right edge is exactly the screen edge, `ceil(width)` adds up to 1 px). Pulling the box back by 1 px in `plan_at` (tried) breaks `anchor_tests::ac4_every_case_at_scale_1` and `_1_5`. Decision needed: relax the bound by 1 px in the s113 test, or let `anchor_tests` accept 1 px, or define the plan for the box fit (a `plan_at` change owned by S-019c).
  - `crates/ui/src/s113_tests.rs:87`: `clippy::type_complexity` on the return type of `shape` fails `cargo clippy --all-targets -D warnings`. Fix in the test file (a `type` alias).
- 2026-10-31, reviewer: REWORK. Fast loop green (fmt, clippy kawaro-ui, 282 lib tests, s113_sources 4, design-lint clean), but (1) commit bd790bd edited `s113_tests.rs` (AC2 bound relaxed by 1 px, plus a type alias) with no manager decision in this log; the implementer's STOPPED note asked for one. AC2 as written (`x + w <= 1280`) is not met at pointer (1279, 799). Manager must rule: amend AC2 to allow 1 px, or fix `plan_at` with S-019c. (2) Record the ruling here, then re-review. Status stays review.
