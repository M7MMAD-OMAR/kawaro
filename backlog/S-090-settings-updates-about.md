# S-090 Settings: current version, Check for updates, Update now
Status: draft (2026-10-28, owner priority: very important)

## Story
As a user, I want the settings window to show my current version, a "Check for updates" button and an "Update now" button, so that I can see and run an update without knowing the tray menu or `config.toml` exist.

## Why this story exists
ADR 0006 (section "Settings window rows" and the Consequences list) and S-054b ("Out of scope") deferred every update row of the settings window to "a later ui task", and no story was ever written for it. The update engine is complete (S-053a to S-054c, S-067), but its only surfaces are the tray menu item, the tooltip, a notification and `kawaro update`. On GNOME, where the tray needs an extension, and on Windows, where the tray icon can be hidden, the user has no visible way to learn the version, check, or install. The three keys `update_check`, `update_channel` and `update_mode` are still edited by hand in `config.toml`.

## Acceptance criteria
- AC1: the settings window has an "Updates" section with the installed version (for example `0.4.0`), a "Check for updates" button, and the state line of the update machine: up to date, checking, downloading with percent, ready, failed with a short reason, unavailable (no curl or offline).
- AC2: "Check for updates" sends `Request::UpdateCheck` over the existing IPC and shows the result within the state line; it never opens another window and never takes focus. While a check runs the button is disabled.
- AC3: an "Update now" button is enabled only in the `Ready` state and sends `Request::UpdateApply`; on a running job it shows the deferred state from S-054b AC7 instead of an error.
- AC4: three controls bound to the existing keys: "Check automatically" (`update_check`), channel stable or beta (`update_channel`), and "Install updates" ask or automatic (`update_mode`, automatic only where S-054b AC4 allows it). Saving writes the same `config.toml` keys; `update_channel_present` semantics are unchanged.
- AC5: when the daemon is not running the section still shows the version and says the background service is off, with the settings Save still working.
- AC6: the same version, check and update actions are reachable from the first-run welcome window footer or the tray "About" item, so the version is visible without opening settings.
- AC7: every new string exists in Arabic and English with full RTL, every control has a descriptive label, all colours and spacing come from tokens, no shadows, no long dashes.
- AC8 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed files, one CHANGELOG line in English and Arabic, screenshots of the section in both languages and in the five states, one on Windows and one on Linux.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | The section renders the version string from the build and each of the six states from a fake update state, in both languages | ui, snapshot |
| AC2 | Click "Check for updates" with a fake IPC: one `update-check` request, button disabled until the response, no new window | integration |
| AC3 | "Update now" disabled in every state except `Ready`; one `update-apply` request in `Ready`; the deferred answer shows the waiting text | integration |
| AC4 | Toggle each control, Save, read `config.toml`: the three keys match; a Cancel leaves the file unchanged | integration |
| AC5 | No daemon socket: the section shows the version and the service-off text, no panic, Save works | integration |
| AC6 | The welcome footer and the tray About item show the same version string | ui |
| AC7 | Arabic and English strings present and non-empty; the RTL snapshot mirrors the order | snapshot |

## Out of scope
- A new update mechanism or a new feed; this story only exposes the existing one.
- Release notes viewer and "skip this version".
- macOS.

## CHANGELOG plan
- Updates in settings (S-090): the settings window shows the installed version, a "Check for updates" button and "Update now", with the automatic check, channel and install mode. «التحديثات في الإعدادات: تعرض النافذة النسخة المثبّتة وزر «ابحث عن تحديث» و«حدّث الآن» مع الفحص التلقائي والقناة ونمط التثبيت».

## Review log
- 
