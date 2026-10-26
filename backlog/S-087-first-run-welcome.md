# S-087 First-run welcome: the demo, Try on an image, and Settings
Status: approved under manager delegation (2026-10-26)

## Story
As a new user who just installed Kawaro, I want the first window to show me the drag flow, let me try it on a sample image with one click, and open the simplified Settings, so that I understand the app before I touch my own files.

## Design intent (owner, 2026-10-26)
The owner asked for a beautiful, systematically built first experience: a demonstration of the drag and the shortcut so the user understands it, a "Try it on an image" button, and a Settings entry that leads to the simplified settings page of S-076. Beautiful means the design system: tokens only, Material 3 controls, Arabic and English with RTL mirroring, reduced motion respected. Nothing technical in any visible string.

## Files
`crates/ui/ui/welcome.slint` (new), `crates/ui/src/lib.rs` (window construction alongside `SettingsWindow::open` in `crates/ui/src/settings.rs`), `crates/ui/build.rs` (the demo frames and the sample image as embedded resources), `crates/daemon/src/ui.rs` (a `present_welcome` beside `open_settings`, which lives here), `crates/daemon/src/server.rs` (the first-run branch of the `Settings` request), `crates/daemon/src/tray/` (a "Get started" item), `tests/fixtures/` (the sample image source), snapshot references under the existing snapshot directory.

## Split, one work day or less each
| Part | Scope | Criteria |
| --- | --- | --- |
| S-087a | The demo frames pipeline: a script that renders the frames from the hive components into `crates/ui` embedded resources, with the 2 MB budget and the licence note | AC2's pipeline and budget clause, the D2 asset decision |
| S-087b | The welcome window, first-run routing, tray item, Try on an image, snapshots and timing | AC1, AC3 to AC7 |

The frames pipeline (S-087a) has an explicit owner and producer: a checked-in script under `spikes/` or `scripts/` renders the frames once from the same hive components the demo shows, and the story name in the script header is S-087. No frame asset may be committed without that generator.

## Acceptance criteria
- AC1: the welcome window opens on a one-shot `welcome_seen` marker in the config, not on "no config file yet": a config written by any other path (a portal denial save, a Settings save, an updater write) must not hide the welcome, and the welcome sets the marker only after it is actually shown. With the marker set, `kawarod settings` and `kawaro settings` open the S-076 settings window exactly as S-086 AC1 requires. The tray menu gains a bilingual "Get started" / «ابدأ هنا» item that opens the welcome window at any time regardless of the marker. D1 below amends S-086 AC1 for the first-run case only.
- AC2: the welcome window shows a silent looping demonstration of the whole flow: dragging a file, activating the trigger, the hive appearing (at the pointer where the platform puts it there, centred where it does not, matching what the user will really see), dropping on a cell, the result appearing next to the untouched original. The demonstration is built from pre-rendered frames or shapes inside Slint and adds no runtime engine or decoder to the base install; the embedded frame set and sample image together stay under 2 MB in the shipped binary. With reduced motion on, it shows one static frame plus the caption steps instead of looping.
- AC3: the demonstration captions are Arabic and English and name the trigger exactly as the measured contracts define it: the S-084 winner for Windows and the S-075 measured contract for Linux, including the wording "drag with the right button" if that is the winner. When the configured key and the live registration differ (the portal fallback of S-075), the caption names the key that actually works on that desktop, read from the registration result, not the config value. Until both contracts exist in their story files the demonstration uses key-free wording (drag, drop on the cell). Captions are available as accessible text, not only drawn pixels.
- AC4: "Try on an image" / «جرّب على صورة» copies a bundled sample image to a writable temporary folder and opens the hive for exactly that one file through the same request path as the Explorer menu entry, so one click lands the user on a live hive and converting writes the result next to the untouched sample. The button is invokable by pointer and by keyboard and has a descriptive bilingual accessible label.
- AC5: "Open Settings" / «افتح الإعدادات» opens the S-076 settings window; no string in the welcome window contains portal, daemon, registry, config, IPC or token.
- AC6: four approved snapshots `welcome-{ar,en}-{light,dark}.png` show the demonstration area, both buttons and no technical wording; element order is mirrored in Arabic; every colour, spacing, radius and duration comes from `design/tokens.json`; no shadow, no gradient, no one-sided border; `scripts/design-lint.sh` and the no-long-dash check pass.
- AC7: the window appears within 1 s of the click that opens it on the S-079 reference machine (the same budget as S-086 AC2; the 150 ms budget of the hive does not apply to a cold-started settings-class window), with the first frame and captions already drawn; one CHANGELOG line in both languages.

## Manager decisions for the tester
- D1: S-087 amends S-086 AC1 for the first-run case only: with no `welcome_seen` marker the `settings` entry opens the welcome window, and S-086's tests asserting "always the settings window" run with the marker set. Every other S-086 acceptance criterion stands unchanged.
- D2: the owner asked for a demo video in the app; the substitution with pre-rendered frames is recorded here because Slint has no video decoder and the base install stays light. The film itself belongs to the download page (S-088), which plays the real clip. One sample image is the source for both this window and the S-088 download button: S-088 generates its site asset from the same fixture this story embeds.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC5 | request routing tests: no `welcome_seen` marker opens welcome, marker set opens settings; a config written by another path does not hide the welcome; tray item label in both languages; forbidden-words scan of the welcome strings | unit, static (CI) |
| AC2, AC3 | demonstration frame set is embedded and non-empty and within the 2 MB budget; reduced motion shows one frame; captions read the registration result in both languages and fall back to key-free wording without a measured contract | unit (CI), UI (private compositor) |
| AC4 | one-click flow: sample file created in a writable folder, hive opened for that exact path, conversion result beside an unchanged sample (hash equal before and after) | integration (private compositor, owner-run) |
| AC6 | four snapshots; design lint; RTL mirroring check | snapshot (private compositor), static (CI) |
| AC7 | timing measurement on the S-079 reference machine (three trials, within 1 s) | performance (owner-run) |

CI runs only the unit and static rows; the UI, snapshot, integration and timing rows are private-compositor or owner-run, and the story never claims a CI green covers them.

## Out of scope
- Video playback or a video engine in the app (the base install stays light; the film lives on the download page, S-088).
- Changing the simplified settings content (S-076 owns it), the drag key registration contract (S-075 and S-084 own it).
- Any auto-opened window without a user click (focus stealing is the top risk; the welcome window opens only from an icon click or the tray item).

## Review log
- 2026-10-26, planner: written from the owner Windows beta report; three goldfish reviews (design, readiness, zoom-out) applied to the text.
