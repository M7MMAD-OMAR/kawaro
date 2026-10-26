# S-017 Glass hive with motion between cells
Status: S-017a done (2026-10-26, glass + motion merged to main d067f23; 13/13 glass_tokens tests, all gates green).

From the maintainer after first real use: "a little glassy, fluid, beautiful, smooth transitions from cell to cell". It builds on what shipped: the layer overlay (S-018, ADR 0004), the hive at the pointer with adaptive shape up to `MAX_RINGS` = 4 (S-019), the drop probe and reveal (S-021), the opening animation from the first drawn frame (S-022) and the keyboard request before the catcher's first commit (S-027). The 2026-10-20 Arabic ACs are superseded by this text (see git history). The Arabic log entries are kept as history.

## Story
As a user who opens the hive many times a day, I want it to look like a glass surface over what is behind it, with light that slides from cell to cell, so that it feels part of the desktop and not a square window.

## Facts (verified) and assumptions
- Verified by running: KWin 6.7.5 `--virtual` (`tests/fixtures/overlay/kwin-6.7.5-globals.txt`) and nested Hyprland 0.56.2 (`WAYLAND_DEBUG` trace in `tests/hyprland/out/hypr-layer-daemon.log`) both advertise `ext_background_effect_manager_v1` v1. Neither advertises `org_kde_kwin_blur_manager`. GNOME Shell 50.5 advertises neither.
- Assumed: the `capabilities` flags each compositor sends (never bound so far); that the blur lands behind a region set on the catcher layer surface; that the headless compositors draw blur at all. The tester records the flags first.

## Planner decisions (for the manager)
1. One protocol: `ext_background_effect_manager_v1`, version 1, on the layer path. `org_kde_kwin_blur` is not used: it would add `wayland-protocols-plasma` as a direct dependency, and no private compositor we run advertises it, so it cannot be tested. Plasma before the ext global gets opaque. No Hyprland `blur` layer rule: the protocol covers Hyprland, and the Lua blur fields are not verified.
2. Glass only when the compositor says it blurs (capability `blur`). `hive_glass = "auto" | "off"` in config.toml (missing means `auto`). `KAWARO_GLASS=on|off` overrides everything and is for tests. The window path (GNOME, X11, screen reader) is always opaque.
3. The blur is requested on the catcher, not the subsurface, in catcher coordinates offset by the hive position. The region is 1 logical px strips in each hexagon's pointed caps plus one rectangle for its straight band.
4. The region follows the motion: it grows ring by ring as each ring starts. It is full size during scale (open 0.92, press 0.94): a rim of 3 px at most, accepted. It stays until the catcher is destroyed on close, so glass is never over a sharp background. A drop show gets no region before its first drawn frame, and the drop move re-sends it in the commit that moves the hive.
5. Values: `glass-2` is `surface-2` at alpha `B8` (72 %), `glass-3` is `surface-3` at `D1` (82 %); the highlight stays opaque `accent`; `border` stays opaque. Motion: `ring-delay-ms` 40, `highlight-move` 140 ms `cubic-bezier(0.2, 0, 0, 1)`, `press` 70 ms to 0.94, `hive-close` 90 ms to 0.96 and opacity 0, `ease-in`.
6. The close animation keeps the surfaces for 90 ms, and the action starts at once. The tester (not the implementer) rewrites the `!hive.inner.visible()` checks right after a pick or a dismiss in `ui_tests.rs` and `drop_hive_tests.rs`.
7. Pure decisions (glass choice, region strips, ring delays) live in `overlay.rs` and `layout.rs`, which are under mutants. `layer.rs` stays outside mutants and is guarded by trace checks.
8. Real blur is measured only where it can be. The checker check in AC14 is hard on a compositor whose recorded flags contain `blur` and where a capture works. In any other case (no `blur` flag, no capture, or no blur drawn by the headless GL), it is INFO, the tester records why, and the ADR says "assumed". This is proposed for acceptance with this story's approval.
9. Each sub-story passes the CLAUDE.md quality gates plus the scripts its own ACs name. S-017e runs the full set (AC19) and adds the single S-017 CHANGELOG line.

## Split (one working day or less each, in order; S-017d needs only S-017a)
| Story | Content | ACs |
| --- | --- | --- |
| S-017a | Tokens, generators, contrast, lint, `docs/design-system.md` | AC1 to AC3 |
| S-017b | Open ring by ring, the sliding highlight, press | AC4 to AC7 |
| S-017c | The animated close | AC8, AC9 |
| S-017d | Glass on the layer path: detection, request, region | AC10 to AC14 |
| S-017e | Snapshots, performance, docs, release gates | AC15 to AC19 |

## Acceptance criteria
- AC1: `tokens.json` has `glass-2` and `glass-3` per scheme: light `#E9E1D0B8` and `#DFD3BBD1`, dark `#27231CB8` and `#322C23D1`, plus the motion tokens in decision 5. `Tokens` and `site/tokens.css` carry them. Every new duration is 0 under reduced motion in both. `build.rs` fails on an unknown easing name or bezier shape, and `hive.slint` contains no literal `ease-out`.
- AC2: `text-primary` on `glass-2` and `glass-3`, composited over pure black and pure white, is 4.5:1 or more in both schemes (8 cases).
- AC3: `scripts/design-lint.sh` is clean and fails on an 8-digit hex colour in `tokens.json` outside `glass-*`. `docs/design-system.md` has rows for glass and the new motion tokens.
- AC4: open (any path): a cell's delay is its slot ring (axial distance) × 40 ms, and the centre has 0. Each cell goes from opacity 0 and scale 0.92 to 1 in 120 ms. With mock time, 100 ms after the first frame ring 1 has a higher opacity than ring 2. At 120 + 40 × R ms (R is the outermost ring shown, 1 to 4) every cell is at 1 and 1, including a corner show with rings 3 or 4.
- AC5: one `accent` highlight element slides between cell centres for the mouse, the keyboard and the drop drag. At 70 ms after a move to a neighbour, its centre is on the segment and at least 25 % of it from each end. At 140 ms it is within 0.5 px of the new centre. Its first appearance is in place, and leaving all cells hides it in place.
- AC6: press shrinks the cell to 0.94 ± 0.005 within 70 ms; release or leaving returns it to 1. Picking stays on the sharp hexagon: every existing `geometry` and `drop_tests` test passes unchanged.
- AC7: reduced motion: at 0 ms after the first frame every cell is at 1 and 1, the highlight is at its target when it changes, and press does not scale. In `repro.sh` a normal show has 8 or more `attach` on the hive subsurface in the 240 ms after its first, and with `KAWARO_REDUCED_MOTION=1` 2 or fewer.
- AC8: close: `on_choose` or `on_dismiss` runs in the same turn as today, and the surface is gone between 80 and 90 ms + one frame later (0 with reduced motion). A click, a key, focus loss or a drag event during the close gives no second callback (1 per show). `present` during the close cancels it and shows the new hive with no extra callback.
- AC9: a probing drop show (nothing drawn) closes at once. A revealed drop show still ends within 500 ms (S-014). `repro.sh` and `tests/drop/kwin.sh level=1` keep their S-021 and S-022 probe and timing checks.
- AC10: the pure glass choice gives `on` only for the ext global with the `blur` capability, `hive_glass` not `off`, and the layer path. It prints `hive glass choice: on|off, REASON` once per change with no environment variable. `global_remove` of the manager, or capabilities without `blur`, make the next show `off` with no request.
- AC11: region (integer logical px, unit test rasterised on a 0.25 px grid): 0 pixels outside every hexagon, coverage 94 % or more of each cell's area, and at most 2 × ceil(r / 2) + 1 rectangles per cell (r is the circumradius from `hex.size.md`, about 41.6, so 43). This holds for rings 1 to 4 at fractional hive positions. `hive_glass` parsing: a missing key gives `auto`, and an unknown value gives `auto` with one load warning while the other keys stay as read.
- AC12: `tests/overlay/kwin.sh glass` (new mode, `KAWARO_GLASS=on`, `KAWARO_REDUCED_MOTION=0`, plus one show with `=1`): each drawn show has exactly one `get_background_effect` on the catcher. With motion, `set_blur_region` count = rings shown + 1, with growing add counts; a drop show that moves after its first frame gets one more. With reduced motion the count is exactly 1. Also required: no `set_blur_region` before the first hive `attach` or after the effect's destroy, the effect destroyed before the catcher's `wl_surface.destroy`, and 0 protocol errors. The capability flags are printed as INFO.
- AC13: glass fills: on the layer path with glass on, cells and stack hexagons are `glass-2` and the centre is `glass-3`. Gaps have alpha 0, and the highlighted cell has alpha 255.
- AC14: opaque: `tests/overlay/gnome.sh`, and `kwin.sh window` with `KAWARO_GLASS=on`, show 0 `get_background_effect` and `hive glass choice: off`. `repro.sh` records the Hyprland flags as INFO. Checker check, in `repro.sh` and `kwin.sh glass`: with glass on over a 4 px black and white checker, the luma standard deviation in a 16 × 16 patch inside a ring-1 cell is at most 0.1 of the checker's. Without blur it would be about 0.28. It is hard or INFO as decision 8 says.
- AC15: `crates/ui/tests/snapshots.sh` with `KAWARO_GLASS=off`: the 8 references still match, and the 4 older ones keep their sha256. The settled highlight matches today's within the `hive_snapshot` tolerance.
- AC16: 4 new references `hive-{en,ar}-{dark,light}-glass.png`, from a layer run with `KAWARO_GLASS=on`. A non-highlighted cell pixel away from text has alpha 184 ± 2, a centre pixel 209 ± 2 and a gap pixel 0. The manager approves them, under the maintainer's delegation and logged here, before they are committed.
- AC17: performance (`measure.sh` with a new `hive_bench animate` mode, KWin `--virtual`): over 20 opens and 20 highlight sweeps of every cell, the max frame render is 16.7 ms or less on the 4-ring box at scale 1. On the 2-ring box at scale 2 the median is 16.7 ms or less, with the max as INFO.
- AC18: `tests/overlay/budget.sh` passes in its plain and token runs and in a new `KAWARO_GLASS=on` run: the median warm first frame is under 150 ms, and RSS after idle is 40 MB or less.
- AC19: ADR 0004 gets an S-017 amendment with each fact labelled verified or assumed. `docs/architecture.md` gets `hive_glass` and the choice line. `CHANGELOG.md` gets one line under `[Unreleased]`, in English and Arabic. The gates pass: the CLAUDE.md quality gates (mutants `--in-place --in-diff`, `CARGO_BUILD_JOBS=1`), `tests/overlay/kwin.sh` (default, `window`, `drop`, `glass`), `tests/overlay/gnome.sh`, `tests/hyprland/repro.sh --json` and `--json --quasimode` (failed 0, skipped 0), `tests/overlay/budget.sh`, `tests/drop/kwin.sh level=1`, and `snapshots.sh`.

## Test cases
| AC | Test | Type |
| --- | --- | --- |
| AC1 to AC3 | Generator output values, zero under reduced motion, build fails on a bad easing; WCAG ratios printed for all 8 cases; lint on a bad fixture | unit |
| AC4 to AC6, AC8 | `ui_tests` with `mock_elapsed_time`: opacity and scale per ring at 0, 100 and 120 + 40R ms (2-ring and corner 4-ring plans); highlight position at 70 and 140 ms by mouse, arrows and drag; press scale; callback count and hide time with input at 45 ms into the close | unit |
| AC7, AC9 | The same with reduced motion; `attach` counts in the `repro.sh` trace; the existing drop checks | unit, integration |
| AC10, AC11 | Choice table over the KWin, GNOME and Hyprland global lists, flags, config and env; region rasterised and counted; `hive_glass` missing, `off`, unknown | unit |
| AC12 to AC14 | `kwin.sh glass` and `window` traces, `gnome.sh` trace, `repro.sh` INFO and checker capture | integration |
| AC15, AC16 | `snapshots.sh`, sha256 and the alpha probes | snapshot |
| AC17, AC18 | `measure.sh` animate mode; `budget.sh` | performance |
| AC19 | All the scripts once, one private compositor at a time; the reviewer reads the ADR, docs and CHANGELOG | integration, review |

## Out of scope
- `org_kde_kwin_blur`, Hyprland blur layer rules, rounded corners, metaball "liquid" shapes, and a settings window toggle.
- Live desktops: every test runs in private compositors only (KWin `--virtual`, headless GNOME Shell, nested Hyprland), in the 6 GB scope with one or two build jobs.
- `backlog/S-014-drop-hive.md`, `.claude/worktrees/`, the branch `windows/0.6.0`.

## Review log
- 2026-10-20، المخطِّط: الجهد نحو 3 إلى 4 أيام، فقُسّمت إلى أربع قصص فرعية. ADR 0004 لم يكن موجوداً عند الكتابة: AC9 إلى AC11 تُراجع بعد قبوله. قرارات تنتظر المشرف قبل الاعتماد، ولا تُعدَّل `CLAUDE.md` قبلها:
  1. ألوان بشفافية في الرموز: يسمح بـ `glass-2` و`glass-3`؟ هذا يوسّع قاعدة «الارتفاع بدرجات الأسطح فقط» إلى «الأسطح أو نسختها الزجاجية، والأعلى أقل شفافية». والقيم 72 % و82 %.
  2. التباين: وعد `docs/design-system.md` بـ 4.5:1 «على كل سطح» يصير على الزجاج مركّباً فوق الأسود والأبيض (AC2)، لأن ما خلفه مجهول.
  3. شكل منطقة الضبابية: `wl_region` مستطيلات فقط. المقترح أشرطة داخل السداسيات (صفر خارجها، تغطية 95 %). البديل مستطيل النافذة كاملاً، فتظهر الفجوات والزوايا صندوقاً ضبابياً، وهو مرفوض في الاقتراح.
  4. «الشكل الجميل»: زوايا سداسية مستديرة؟ المقترح لا تُضاف في هذه القصة. إن أرادها المشرف فهي رسم فقط بنصف قطر من الرموز (`radius-sm` 6)، والاختيار يبقى بالسداسي الحاد، وإلا انكسر S-014 AC2.
  5. التسارع: `build.rs` يفرض اليوم `ease-out` و`hive.slint` يكتبه حرفياً. المقترح أن تحمل الرموز أسماء التسارع ومنحنيات `cubic-bezier`، والآلية للمنفِّذ.
  6. الإغلاق المتحرك يؤخر اختفاء النافذة 90 ms، والإجراء يبدأ فوراً (AC7). موافق؟
  7. البديل في GNOME: معتم تماماً كاليوم (AC11)، لا زجاج دون ضبابية، لأن الشفافية فوق خلفية حادة تضر القراءة.
  - مفتوح مع ADR 0004: آلية الضبابية في Hyprland (قاعدة المؤلِّف لسطح layer-shell، وتجاهل الفجوات الشفافة). لا أثر في المستودع لعوامّ Hyprland، فمسار (ب) في AC9 مبني على أن Hyprland لا يعلن عامّ ضبابية للعميل.
- 2026-10-20، المدير (بتفويض المشرف في قرارات الأداء والواجهة): القصة معتمدة، والقرارات السبعة:
  1. رموز الزجاج `glass-2` و`glass-3` مسموحة (سطح بشفافية 72 % و82 %)، وتُضاف القاعدة إلى CLAUDE.md: الشفافية من رموز الزجاج فقط، وفوق ضبابية المؤلِّف فقط.
  2. التباين 4.5:1 لـ `text-primary` فوق الزجاج مركّباً على الأسود والأبيض الخالصين، في المظهرين.
  3. الضبابية داخل السداسيات فقط، لا المستطيل كله.
  4. لا زوايا مدوّرة الآن. تُعاد مع مراجعة الشكل بعد ADR 0004.
  5. منحنيات مسماة و`cubic-bezier` في الرموز مسموحة، ويُزال `ease-out` المكتوب يدوياً من `hive.slint`.
  6. إغلاق متحرك 90 ms مقبول، والإجراء يبدأ فوراً.
  7. GNOME وكل مؤلِّف بلا ضبابية: معتم بالكامل، بلا شفافية.
- 2026-10-23, planner: rewritten in English for 0.4.0 against the current code (`layer.rs`, `motion.rs`, `hive.slint`, `hex-cell.slint`, `layout.rs`, `build.rs`, the test scripts) and against S-018, S-019, S-021, S-022 and S-027. The 2026-10-20 decisions stand, except as refined in decisions 1 to 9. The AC numbers in the Arabic entries refer to the superseded ACs. The Hyprland open question there is closed by the verified ext global. The new facts: the ext global on KWin and Hyprland, and no kwin_blur anywhere. Ring delays now use the slot ring up to 4. Split into five sub-stories. No open questions. Status: draft, for the manager's approval.
- 2026-10-23, manager under the maintainer's delegation: approved with the planner's nine decisions and the split S-017a to S-017e. It runs after S-014f so that only one chain changes `crates/ui` at a time. New glass references (AC16) are approved by the manager after looking at them, logged here before commit.
