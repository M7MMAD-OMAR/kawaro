# S-076 Settings without technical words, with an accessible drag-key picker
Status: approved (2026-10-26, manager), re-baselined against main 25f9254

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
