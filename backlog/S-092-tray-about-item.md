# S-092 Tray "About" item: version, Check for updates, Settings
Status: draft (2026-10-28)

## Story
As a user, I want an "About" item in the tray menu that shows my installed version and offers "Check for updates" and "Settings", so that I can learn the version and look for an update without opening the settings window first.

## Why this story exists
S-090 AC6 allowed "welcome footer or tray About item" and the implementer shipped only the welcome footer (`about_line`). The tray menu today has Update now, Get started, Cancel all, Settings and Quit (`crates/daemon/src/tray/linux.rs`, `shell_logic.rs`), with no place that states the version.

## Ordering
Implement after S-091, which comes after S-090 passes review. Both S-091 and S-092 edit `crates/core/src/ipc.rs` and the daemon (`update.rs`, `ui.rs`, `tray/`), so they must be built one after the other, never in parallel.

## Acceptance criteria
- AC1 (Linux menu): the StatusNotifierItem menu gains one item "About" (Arabic `حول كوارو`) between Get started and Cancel all. The menu has 7 entries (6 today, counting the separator before Quit, plus this one).
- AC2 (what About shows): activating it opens a small About popup (the existing UI crate window pattern, shown with the same raise rules as the welcome window) containing exactly: `about_line(lang)` from S-090 (`Kawaro version 0.4.0`, `كوارو النسخة 0.4.0`), the S-090 update state line, a "Check for updates" button and a "Settings" button. The version text is the output of `about_line`, not a second copy of the string.
- AC3 (actions): "Check for updates" sends one `Request::UpdateCheck`, disables itself until the reply, and shows the result in the state line (with the S-091 percent when downloading). "Settings" calls the same `open_settings` as the tray Settings item and closes the popup. Neither opens a further window.
- AC4 (no background focus): the popup appears only from a user click on the item. No background event (update found, download done, a job finished) opens it or any window; those stay a tray tooltip and one notification (`docs/PLAN.md`, decision of 2026-10-20). Count of windows opened by a background update tick in the test: 0.
- AC5 (Windows): the Windows tray menu (`shell_logic::menu`, S-048 surface exists) gets the same item with the same two actions, as a new `Command::About` placed after `GetStarted`; its popup is the same window. Tested by menu content on any platform; the Windows live check stays on the S-079 owner checklist.
- AC6 (language and labels): all strings exist in Arabic and English with full RTL (menu text direction follows `lang.rtl()`); every control has a descriptive accessible label; colours and spacing come from tokens; no shadows; no long dashes.
- AC7 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed files, one CHANGELOG line in English and Arabic, a screenshot of the popup in both languages.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Build the Linux menu for En and Ar: the item list has the About label at index 2 (`About` / `حول كوارو`), the total is 7 entries including the separator, and the other labels are unchanged | unit (daemon) |
| AC2 | The popup version text equals `about_line(Lang::En)` and `about_line(Lang::Ar)` exactly; the state line equals the S-090 line for the same fake state; button labels equal the exact English and Arabic strings | integration (ui) |
| AC3 | Press Check: one `update-check` request on a fake IPC, button disabled until the reply, state line shows `Downloading update 42%` after a downloading reply with percent 42. Press Settings: `open_settings` spy called once, popup closed | integration |
| AC4 | Run an update tick that finds a new version with a fake fetcher: window-open spy count is 0 and the notification spy count is 1 | integration (daemon) |
| AC5 | `shell_logic::menu(Lang::En, ..)` contains `Command::About` after `GetStarted` with the English label, and the Arabic menu with the Arabic label; the command handler maps it to the popup | unit (daemon) |
| AC6 | Strings non-empty and different in the two languages; RTL snapshot mirrors the button order | snapshot |
| AC7 | Reviewer runs the gates | gate |

## Out of scope
- Licence text, credits, links and release notes in the popup.
- GNOME-specific changes to how the tray is shown (needs the extension anyway).
- macOS.

## CHANGELOG plan
- Tray About item (S-092): the tray menu shows the installed version with Check for updates and Settings. «عنصر «حول» في قائمة الأيقونة: يعرض النسخة المثبّتة مع «ابحث عن تحديث» و«الإعدادات»».

## Review log
-
