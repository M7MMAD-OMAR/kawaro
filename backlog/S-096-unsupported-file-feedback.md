# S-096 Tell the user why a file has no actions, and what to do next
Status: approved (2026-10-29, manager)

## Story
As a user who drops or opens a file Kawaro cannot convert, I want the hive to say plainly why and what I can do, so that I do not think the app is broken and I can ask for the format without sending anything by accident.

## Today (read from the code)
- `crates/daemon/src/server.rs` `open` and `crates/daemon/src/ui.rs` `on_drop_files` treat "no actions" as one case: a notification "No actions for these files" (`text::no_actions`) and an `Err`, no hive, no reason, no next step.
- `State::matching` hides actions whose engine is not available, so "format unsupported" and "engine missing" look the same. A mixed selection is not handled at all.

## Acceptance criteria
- AC1 (pure classification): `kawaro_core` gets `classify(files, registry, engine_ok) -> Outcome` with no I/O. `Outcome` is exactly one of: `Supported`, `Unsupported { ext }`, `MissingEngine { tool }`, `Mixed { supported, unsupported }`, `NoSelection`. Rules: a file is "unsupported" when no registry action accepts its extension even if every engine were available; it is "missing engine" when an action accepts it but every such action's engine is unavailable. Extension is lowercase, without the dot; a file with no extension gives `ext = ""`. Tool names are exactly `FFmpeg`, `LibreOffice`, `Ghostscript`, mapped from engine ids `ffmpeg`, `office`, `ghostscript`.
- AC2 (hive opens, unsupported): for an unsupported selection of 1 file `a.xyz`, `server::open` returns `Ok`, the hive is presented, the centre text is `File .xyz` (en) and `ملف ‎.xyz` (ar), and the centre message below it says `This format is not supported yet` (en) and `هذه الصيغة غير مدعومة بعد` (ar). For N files of the same extension the centre reads `N files .xyz`; for 2 or more different extensions it lists at most 3 extensions then `+K`.
- AC3 (missing engine): the centre message names the tool and one install hint taken from a fixed table in core: FFmpeg, LibreOffice, Ghostscript, each with its official download link and the package name. The text contains the tool name exactly once. No download is started and no process is spawned (test asserts 0 child processes and 0 network calls).
- AC4 (mixed): for a selection of 2 supported and 1 unsupported file, the hive shows the actions for the supported files, plus 1 greyed cell per unsupported file showing only its extension code (for example `XYZ`), up to 6 greyed cells then one `+K` cell. Greyed cells are not activatable (activating one runs 0 jobs). Hovering a greyed cell puts its full explanation in the centre cell. Choosing a real action runs it on the 2 supported files only and the unsupported file is not touched. Nothing is silently dropped: greyed cells plus supported files equals the selection size.
- AC5 (request a format): every unsupported or missing-engine hive has one cell with the code `REQ` and the label "Request this format" / `اطلب هذه الصيغة`. Nothing opens until it is clicked (0 browser launches on show, 1 on click). The URL is built by a pure function and equals `https://github.com/M7MMAD-OMAR/kawaro/issues/new?title=Format+request%3A+.xyz&body=...` where the body is a fixed template plus the extension only. The extension is kept only if it matches `[a-z0-9]{1,16}`, otherwise it becomes `unknown`. The URL contains no file name, no path, no file content, no home directory, no user name, no host name and no version string. The centre text on hover of the cell says, in both languages, that nothing is sent automatically and Kawaro has no telemetry. The browser is started through the existing opener module only (`crates/daemon/tests/s049_sources.rs` forbids spelling `xdg-open` elsewhere).
- AC6 (notification path): when no hive can be shown (a shortcut with 0 files selected, or no display), the notification text names the reason and the next step: `NoSelection` gives `No file selected. Select a file, then press the shortcut again.` (and its Arabic text), unsupported gives `.xyz files are not supported yet`, missing engine gives `FFmpeg is not installed`. The old string `No actions for these files` appears nowhere in `crates/` (grep count 0).
- AC7 (closest alternative): a suggestion line appears only when the registry has an action whose input family (`media::detect`) equals the file's family and whose engine is available; it shows that action's label. When the family is `Other` or no such action exists, no suggestion line is shown (0 lines, never a generic one).
- AC8 (design and access): every colour, size and duration comes from `design/tokens.json` (`scripts/design-lint.sh` passes); greyed cells use an existing surface and text token, not opacity hacks; no shadow, gradient or one sided border. Full names never appear inside a small cell (cell text is the code only). With reduced motion on, the hive opens with 0 animated frames for the new cells. Every new cell and the centre message has a non empty accessible label in both languages, and a greyed cell is announced as unavailable with its reason.
- AC9 (budget): classification of 1000 paths takes 20 ms or less in a unit test, and the hive for an unsupported selection is presented within 150 ms of the request, same budget as today, measured on 3 trials in the private compositor.
- AC10: one `CHANGELOG.md` line in English and Arabic; no changed file contains an em dash or en dash; strings exist in Arabic and English with Western digits.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Table test over 12 rows (empty list; `a.png` with all engines; `a.xyz`; no extension; `A.XYZ` gives `xyz`; `a.mp4` with ffmpeg off gives `MissingEngine FFmpeg`; `a.docx` with office off gives LibreOffice; `a.pdf` with ghostscript off; two files needing two missing tools; `a.png`+`b.xyz` gives Mixed; two unsupported with different extensions; supported file whose action engine is available); assert the exact `Outcome` for each | unit |
| AC2 | Testing backend: call `open` with `[a.xyz]`; assert `Ok`, centre text and message strings exact in en and ar; 3 files `.xyz`, and `.a .b .c .d` selection, assert `+1` text | interface |
| AC3 | Table of 3 tools: assert message holds the tool name once, the link and the package name; spy on process spawn and socket, assert 0 | unit, interface |
| AC4 | Selection `a.png`, `b.png`, `c.xyz`: assert cell list equals real actions plus 1 greyed `XYZ`; click greyed cell, assert 0 jobs; choose an action, assert the queue receives exactly `[a.png, b.png]`; 9 unsupported files, assert 6 greyed plus `+3` | interface |
| AC5 | URL function table: `xyz`, `ABC` gives `abc`, `a b` gives `unknown`, 17 chars gives `unknown`, empty gives `unknown`; assert the full URL string, and assert it contains none of the input file name, `/home`, `$HOME`, hostname, version. Show hive with a spy opener, assert 0 calls; click `REQ`, assert 1 call with that URL | unit, interface |
| AC6 | Table of 3 reasons x 2 languages, assert exact notification strings; grep test over `crates/` for the old string, count 0 | unit |
| AC7 | Registry fixture with one image action: `.heic`-family stand-in file gets the label line; `.xyz` gets none; family present but engine off gets none | unit |
| AC8 | `scripts/design-lint.sh`; accessible label non empty for each new element in en and ar (testing backend); reduced motion flag on, assert 0 animation ticks; screenshots: 3 states (unsupported, missing engine, mixed) x ar/en x light/dark = 12 images compared to baselines, and checked by eye for RTL order, no clipped text and no text inside small cells | interface, screenshot |
| AC9 | Time `classify` over 1000 generated paths, assert 20 ms or less; private compositor, 3 trials, show delay 150 ms or less | unit, performance |
| AC10 | Grep for U+2013 and U+2014 over changed files, count 0; both language tables have equal key sets | unit |

## Execution split (one working day at most each)
| Story | Scope | Criteria |
| --- | --- | --- |
| S-096a | Core `classify`, tool table, URL builder, all strings and notification texts, no UI change except routing the old notification | AC1, AC3 (strings), AC5 (URL), AC6, AC7, AC9 (classify part), AC10 |
| S-096b | Hive: centre message, greyed cells, `REQ` cell, opener call, drop hive path, screenshots | AC2, AC4, AC5 (UI), AC8, AC9 (hive part) |

S-096b does not start before S-096a is merged and CI is green.

## Mutants plan
Changed files only, never the whole tree: `cargo mutants --in-place --in-diff <diff>` with `--jobs 2` and `CARGO_BUILD_JOBS=2`, from a script file run inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`. Expected files: the new core classification module, `crates/daemon/src/text.rs`, `crates/daemon/src/server.rs`, `crates/daemon/src/ui.rs`. No surviving mutants; each survivor gets a new test, not an exclusion.

## Out of scope
Adding any new format or engine, automatic download or install of an engine, sending the request without a click, reading file content or names, analytics of any kind, Windows live verification (the URL and classification tests run there; the click path is owner pending), changing the engine contract.

## Review log
- 2026-10-29, planner: drafted from the owner report and the code (`text.rs`, `server.rs`, `ui.rs`, `state.rs` `matching`); waiting for the manager's approval.
