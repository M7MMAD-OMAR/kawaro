# S-076 Settings without technical words, with an accessible drag-key picker
Status: S-076a: done (reviewer PASS, 2026-10-26); S-076b: approved (2026-10-26, manager), pending; re-baselined against main 25f9254

## Story
As an ordinary user, I want only plain choices in Settings and an accessible way to change the drag key, so that I can use Alt by default or choose another suitable key without understanding desktop registration.

## Current code and conflicts
- `crates/ui/ui/settings.slint` always renders the Folder name field, disabled unless In a subfolder is selected. S-076 requires it to be absent otherwise, not merely disabled.
- The visible portal switch and its tests came from S-014f1 AC23 and AC24. This story supersedes that visible UI only. `Config::portal_shortcut` still records a portal denial, so removing the switch must not turn `false` back into `true`.
- `FormState::to_config` starts from `Config::default()`. `Inner::save` restores only `hive_surface` and `hold_ms`. A Save therefore resets the custom Windows `shortcut` recorded as a follow-up in S-048, and can also reset `update_check`, `update_channel` and `update_mode`.
- There is no `drop_key` field or key picker today. S-019 AC21 records that a modifier alone is not a valid GlobalShortcuts trigger, while S-075 requires Alt by default. The separate real portal spike and S-075 own that desktop contract. S-076 must not prescribe portal, Hyprland or other desktop registration.

## Decisions
1. Remove the portal-registration row and all technical wording from the window. Keep `portal_shortcut` as hidden state and preserve its opened value, including `false`, on every Save. S-075 owns automatic registration and recovery from a remembered denial.
2. Show Folder name only while In a subfolder is selected. Changing away and back within one open window keeps the typed name. A Save with another output location still preserves `output_folder`.
3. Save starts from the complete Config passed to `open`, changes only fields exposed by the form, and preserves every unrelated field. This explicitly includes the custom Windows `shortcut`, `hive_surface`, `hold_ms`, `portal_shortcut`, all update settings and `update_channel_present`.
4. Add a Drag key row with the current key, a Change button, a Reset to Alt button and plain helper text. A fresh or missing `drop_key` means `Alt`. A captured choice is committed to `config.toml` by Save, consistently with the other controls. Cancel, window close and Escape outside capture save nothing.
5. Capture accepts Alt, Ctrl, Super or Shift alone, or a combination with at least one modifier. It refuses a plain letter alone and unsupported input without replacing the previous value. Escape cancels capture, restores the previous displayed value and leaves Settings open.
6. Change, Reset to Alt, the current value, capture instructions, errors and helper text have descriptive accessible labels in Arabic and English. Capture is usable by keyboard alone and in RTL. Material 3 components and design tokens only.
7. S-076 persists and hands a valid `drop_key` to the existing Save and reload path. S-075 alone defines how each desktop registers Alt or another key, detects conflicts and applies changes live. S-076 adds no Hyprland or portal binding test and no desktop-specific registration design.

## Split, one work day or less each
| Part | Scope | Criteria |
| --- | --- | --- |
| S-076a | Simplify the window, conditional Folder name, complete Config preservation, superseded tests | AC1 to AC4 |
| S-076b | `drop_key`, accessible bilingual capture and reset, S-075 handoff, snapshots and user docs | AC5 to AC10 |

## Acceptance criteria
- AC1: In English and Arabic, no accessible element has the S-014f1 portal-registration label and no visible Settings string contains portal, daemon, registry, config, IPC or token. The ordinary output, quality, notification and Open the folder when done controls remain.
- AC2: With Next to the original selected, zero accessible element labelled Folder name or «اسم المجلد» exists. Selecting In a subfolder creates exactly one enabled text input with that label; selecting Next to the original removes it again. Switching away and back retains the typed name, and Cancel saves nothing.
- AC3: Open a Config whose every persisted field differs from its default, including `shortcut = "Ctrl+Alt+F9"`, `hive_surface = "window"`, `hold_ms = 1500`, `portal_shortcut = false`, `update_check = false`, `update_channel = "beta"` and `update_mode = "auto"`. Save without edits emits a Config equal to the opened one and retains `update_channel_present`. Repeat in English and Arabic, then change one visible field and prove that only that field differs.
- AC4: Open with `portal_shortcut = false`, change any visible setting and Save. The emitted Config still has `portal_shortcut = false`; no portal switch or other visible registration control is needed for this preservation.
- AC5: A missing `drop_key` deserializes as `Alt`, and a fresh window shows Alt in both languages and directions. A valid captured choice followed by Save writes `drop_key` and preserves every unrelated value as in AC3. The saved value reaches the S-075 reload input without a second Settings-specific config file.
- AC6: Change is invokable by pointer and keyboard. During capture, an accessible bilingual instruction says to press a key or combination and that Escape cancels. The candidate and result are available through accessible text, not visual text alone. The helper says, in the window language, to hold this key while dragging files to open the hive.
- AC7: Alt, Ctrl, Super and Shift alone are accepted, as is at least one modified non-modifier key. A plain letter alone and an unsupported key each produce a plain bilingual accessible error, leave the previous value unchanged and emit no Config until a later valid Save. Escape leaves capture and Settings remains open with the previous value.
- AC8: Reset to Alt is keyboard accessible in both languages. After choosing another valid key, Reset shows Alt; Save persists Alt, while Cancel after Reset persists nothing.
- AC9: Four approved snapshots, `settings-{ar,en}-{light,dark}.png`, show no portal row, no Folder name in the default output mode, and the Drag key row. A second subfolder snapshot or UI test proves Folder name appears only in that mode. Design lint and the no-long-dash check pass.
- AC10: English and Arabic user docs describe the default Alt, changing and resetting it, and Escape cancellation without documenting a desktop registration mechanism as decided here. One CHANGELOG entry names S-076 in both languages. Existing S-075 acceptance tests remain the authority for live desktop application and conflict handling.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2, AC4 | Accessible-tree checks in both languages and directions; output-mode transitions and Cancel | UI |
| AC3 | Whole-Config equality with all fields non-default, custom Windows shortcut and one-field changes | unit, UI |
| AC5 | Config missing/default/round-trip tests; Save handoff with a valid `drop_key` | unit, integration |
| AC6 to AC8 | Keyboard-only capture, accessible labels and values, valid and invalid keys, Escape and Reset | unit, UI |
| AC9 | Four settings snapshots plus the conditional-folder case; design lint | snapshot, static |
| AC10 | Bilingual docs and CHANGELOG checks; S-075 tests unchanged | static, integration |

## Out of scope
- Choosing or implementing Alt registration on GNOME, KDE, Hyprland or another desktop; portal spike conclusions; live rebinding deadlines; desktop conflict detection. These belong to S-075.
- Changing the Windows `shortcut` through this drag-key picker. S-076 only stops Settings from resetting that independent value.
- X11 work from S-015.

## Manager decisions for the tester, 2026-10-26
- D1: S-076 supersedes S-014f1 AC23 and the visible-switch part of AC24. Replace `ac23_s014f1_portal_shortcut_switch`; do not preserve assertions that the switch exists, toggles, sits below Open the folder when done, or changes `portal_shortcut`. Replace them with AC1 and AC4 absence and hidden-preservation checks.
- D2: Keep `s014e1_save_keeps_a_remembered_portal_denial` as a behaviour guard, updating only its stale comment that refers to a visible switch. Rewrite `s029_ac7_form_round_trip_keeps_portal_shortcut_false` so it tests hidden preservation rather than requiring a portal field in the visible form.
- D3: Replace the `settings_window_behaviour` assertions that Folder name exists but is disabled beside the original. AC2 requires absence there and presence only for a subfolder. Update exhaustive `FormState` literals for the new internal state; these are mechanical test updates, not permission to reset Config defaults.
- D4: Add explicit regression coverage for S-048's confirmed reset: a custom Windows `shortcut` survives an unedited Save and a Save after changing another control. Cover every other Config field in the same whole-value test.
- D5: The four old settings references are expected to fail after the UI change. The tester rewrites the snapshot expectations from AC9, and the manager approves new references later. The old live-binding requirement in former AC3 is withdrawn, not moved into these tests.

## Review log
- 2026-10-26, planner: re-baselined against `settings.rs`, `settings.slint`, `config.rs`, S-014f1, S-048 and S-075. No code or tests changed by planning.
- 2026-10-26, tester: S-076a RED before implementation. Ran through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`. `cargo fmt --all --check` passed. Exact failure summaries:
  - `cargo test -p kawaro-ui s076a -- --nocapture`
    ```text
    failures:
        settings::tests::s076a_ac3_form_round_trip_preserves_the_complete_non_default_config
        settings::tests::window::s076a_ac1_settings_have_no_registration_or_technical_controls
        settings::tests::window::s076a_ac2_folder_name_is_conditional_retained_and_cancelled
        settings::tests::window::s076a_ac3_save_preserves_complete_config_and_changes_one_visible_field
        settings::tests::window::s076a_ac4_visible_change_preserves_hidden_portal_denial

    test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 177 filtered out; finished in 0.56s
    ```
  - `cargo test -p kawaro-ui settings_window_behaviour -- --nocapture`
    ```text
    failures:
        settings::tests::window::settings_window_behaviour

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 181 filtered out; finished in 0.25s
    ```
  - `cargo test -p kawaro-core --test s029_sources s029_ac7_form_state_has_no_visible_portal_switch -- --nocapture`
    ```text
    failures:
        s029_ac7_form_state_has_no_visible_portal_switch

    test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.00s
    ```
  - Runner statuses: `RED_STATUSES ui_s076a=101 ui_behaviour=101 core_source=101`.
  - The AC1 and AC4 failures found one superseded registration row. AC2 and the rewritten behaviour test found two Folder name accessible labels beside the original instead of zero. Both AC3 tests showed reset values for the custom Windows shortcut and update fields; the form conversion also reset `hive_surface` and `hold_ms`.
- 2026-10-26, implementer: stopped before implementation because the AC3 form test is incomplete as written. `s076a_ac3_form_round_trip_preserves_the_complete_non_default_config` calls `FormState::from_config(&start, rtl).to_config(rtl)`, so complete Config preservation requires the opened Config to be retained inside `FormState`. Adding that hidden field makes the exhaustive `FormState` literals in `form_state_of_the_defaults` fail to compile unless the tester makes the mechanical updates anticipated by D3. No implementation or test was changed. The tester needs to add the hidden complete-Config state to those literals, or revise the form conversion API and its calls consistently. A process-global or thread-local Config sidecar would make interleaved windows save the wrong Config and is not a valid implementation.
- 2026-10-26, tester: corrected the D3 scaffolding without changing any AC1 to AC4 assertion. The default-form fixtures now use struct update from `FormState::from_config` with the intended opened Config and explicitly set every visible expected field. The hidden portal round trip checks the saved Config rather than a required FormState field, and the S-029 source test checks only visible-switch absence rather than prescribing hidden storage. `form_state_of_the_defaults` and `s029_ac7_form_round_trip_keeps_portal_shortcut_false` pass. `cargo fmt --all --check` passes. Ran through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`; statuses: `ui_s076a=101 ui_behaviour=101 core_source=101`. Exact remaining RED failures:
  - `settings::tests::s076a_ac3_form_round_trip_preserves_the_complete_non_default_config`
  - `settings::tests::window::s076a_ac1_settings_have_no_registration_or_technical_controls`
  - `settings::tests::window::s076a_ac2_folder_name_is_conditional_retained_and_cancelled`
  - `settings::tests::window::s076a_ac3_save_preserves_complete_config_and_changes_one_visible_field`
  - `settings::tests::window::s076a_ac4_visible_change_preserves_hidden_portal_denial`
  - `settings::tests::window::settings_window_behaviour`
  - `s029_ac7_form_state_has_no_visible_portal_switch`
  These are behaviour failures only: hidden Config values still reset, the registration row and getter remain, and Folder name remains beside the original. There is no incomplete-fixture or compilation failure.
- 2026-10-26, tester: mechanical clippy correction in the S-076a test code. RED capture first, through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`: `cargo clippy --all-targets -- -D warnings` failed with `clippy::cloned_ref_to_slice_refs` ("unnecessary use of `clone` to create a slice from a reference") at `crates/ui/src/settings.rs:934` on `assert_eq!(*saved.borrow(), [start.clone()], "{lang:?}: unedited Save")` (`clippy_status=101`). Rewrote that one test expression as `assert_eq!(saved.borrow().as_slice(), std::slice::from_ref(&start), "{lang:?}: unedited Save")`, clippy's own `std::slice::from_ref` suggestion. The assertion is semantically identical: the complete saved list must equal exactly the opened `start` Config as a one-element slice, with the same message. No clone remains, no acceptance assertion was weakened, retargeted or removed, and no implementation code or other test was edited. The same file also carries the S-076a implementation already in the working tree (`FormState.opened`, `Inner::opened`), committed together with this fix unchanged. Gates re-run through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`; statuses: `fmt=0 clippy=0 ui_s076a=0 ui_behaviour=0 core_source=0`. Exact results:
  - `cargo fmt --all --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
  - `cargo test -p kawaro-ui s076a`: 5 passed; 0 failed; 177 filtered out.
  - `cargo test -p kawaro-ui settings_window_behaviour`: 1 passed; 0 failed; 181 filtered out.
  - `cargo test -p kawaro-core --test s029_sources`: 10 passed; 0 failed.
- 2026-10-26, implementer: S-076a finished in `crates/ui/ui/settings.slint`, the only remaining implementation file. The portal-registration row and the `portal-shortcut` string are gone from the window; Folder name is now a conditional `if output == subfolder-index` field whose text lives in the root `folder` property, so a typed name survives switching away and back, and Save still starts from the complete opened Config via the already committed `FormState.opened` / `Inner::opened`. No test or other file changed; no S-076b picker added. Gates re-run through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`; statuses: `fmt=0 clippy=0 ui_s076a=0 ui_behaviour=0 core_s029=0`. Exact results:
  - `cargo fmt --all --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
  - `cargo test -p kawaro-ui s076a`: 5 passed; 0 failed; 177 filtered out.
  - `cargo test -p kawaro-ui settings_window_behaviour`: 1 passed; 0 failed; 181 filtered out.
  - `cargo test -p kawaro-core --test s029_sources`: 10 passed; 0 failed.
- 2026-10-26, reviewer: PASS for S-076a (AC1 to AC4). Gates through the heavy lock and memory/CPU scope with `CARGO_BUILD_JOBS=2`; statuses: `fmt=0 clippy=0 ui_s076a=0 ui_behaviour=0 core_s029=0 design_lint=0`. Exact results:
  - `cargo fmt --all --check`: clean.
  - `cargo clippy --all-targets -- -D warnings`: clean, no warnings.
  - `cargo test -p kawaro-ui s076a`: 5 passed; 0 failed; 177 filtered out.
  - `cargo test -p kawaro-ui settings_window_behaviour`: 1 passed; 0 failed; 181 filtered out.
  - `cargo test -p kawaro-core --test s029_sources`: 10 passed; 0 failed.
  - `scripts/design-lint.sh`: clean.
  - AC1 verified by hand and by `s076a_ac1_settings_have_no_registration_or_technical_controls`: the registration row and its `portal-shortcut` string are gone from `settings.slint`, no visible string or accessible label in English or Arabic contains portal, daemon, registry, config, IPC or token, and the output, quality, notify and Open the folder when done controls remain with descriptive labels in both languages.
  - AC2 verified by hand and by `s076a_ac2_folder_name_is_conditional_retained_and_cancelled`: the field is `if root.output == root.subfolder-index` with `text <=> root.folder`, so Folder name exists exactly once and enabled only while In a subfolder is selected, the typed name survives removal and return, Cancel saves nothing, and a Save with another output location keeps `output_folder`.
  - AC3 verified by hand and by `s076a_ac3_form_round_trip_preserves_the_complete_non_default_config` and `s076a_ac3_save_preserves_complete_config_and_changes_one_visible_field`: `FormState.opened` and `Inner::opened` hold the complete opened Config, `to_config` starts from `..self.opened.clone()` and changes only the visible fields, the all-non-default Config (custom `shortcut = "Ctrl+Alt+F9"`, `hive_surface = window`, `hold_ms = 1500`, `portal_shortcut = false`, `update_check = false`, `update_channel = beta`, `update_mode = auto`) round trips unchanged in both languages and directions with `update_channel_present` retained, and a one-field edit differs in that field only.
  - AC4 verified by hand and by `s076a_ac4_visible_change_preserves_hidden_portal_denial` and `s014e1_save_keeps_a_remembered_portal_denial`: a visible change keeps `portal_shortcut = false` in both languages, with no registration control in the window.
  - The review log matches the commits and gate numbers exactly: the RED failure names and counts fit the pre-change code, the clippy fix sits at `settings.rs:934` with `std::slice::from_ref`, 3a062a3 changed only the fixtures and the S-029 absence checks, and 640c859 changed only `settings.slint` plus this log. No em or en dash in the story or the four changed files. No CHANGELOG line here: AC10 in S-076b names S-076 in both languages. S-076b (AC5 to AC10) is untouched and stays pending.
