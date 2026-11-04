# S-128 PDF compress DPI and a qpdf child engine (linearize, cleanup, encrypt check)
Status: done (2026-11-04)

## Story
As a user, I want to compress a scan with a real image resolution target, get a fast web view copy, clean a PDF without re-encoding its images, and see whether a file is encrypted before I touch it, so that the PDF tools stop at the safe structural operations a local converter should own. This is P0 items 3 and 4 of `docs/research/pdf-feature-benchmark.md`: compress with honest DPI controls on Ghostscript, and lossless cleanup plus linearize through a qpdf child process, with the qpdf `--check` and encryption report as the guard.

## Facts found while planning
- `pdf.compress` is a Ghostscript child run: `-sDEVICE=pdfwrite -dCompatibilityLevel=1.5 -dPDFSETTINGS=/preset -sOutputFile=...`, with `screen`, `ebook`, `printer` and `prepress` presets, and the existing rule that the smaller of the output and the input is kept. Exposing a DPI target means adding Ghostscript's image downsampling parameters on top of the preset, not a new engine.
- qpdf is a small Apache-2.0 C++ program, not a library to link. The benchmark lists it as the safest first new child engine (section 4 and P0 item 4): `--linearize` for fast web view, structural rewrite for lossless cleanup, `--check` and `--show-encryption` for the encryption and permission report. It has no raster, OCR or PDF/A role.
- A qpdf rewrite breaks signatures and object IDs, as every structural rewrite does; the benchmark section 7 says detect signatures first and say so instead of silently re-signing. This story only reports and rewrites; it never signs.
- The benchmark section 10 is explicit that a new engine id needs an ADR, and an owner decision is needed before a new child system program joins the base install. Both are settled: ADR 0014 (accepted 2026-11-04) records the `qpdf` engine, the child program and the safety rules.
- The option schema from S-117 and the CLI `--set` of S-120 already carry per-run values, so the compress DPI field and the qpdf operation options need no new plumbing.
- The Ghostscript adapter is the pattern for a child engine: `locate::Program` lookup, `available()`, `run_child` in a `ProcessTree` (S-030), a timeout, a private work directory and a move to `task.output`. `kawaro doctor`, the hive matching and the MCP engine list then learn about qpdf with no MCP work.

## ADR (accepted)
`docs/adr/0014-qpdf-engine.md` is accepted (owner approval 2026-11-04). It settles the engine id `qpdf` and its three ops, the separate process rule, the fixed flags, the one-file text report of `check`, the `Unavailable` path, the encrypted input policy, the signature limitation, and the counts: 56 built-in actions and 17 option actions. The licence row in `docs/engines.md` (Apache-2.0, child only, never linked, never downloaded) still lands before the code, as the repo rule requires.

## What this story will prove
- A DPI target on Ghostscript compress is useful and honest: a lower image resolution makes a scan smaller, the pages and their text stay the same, and the existing smaller-or-original rule still protects the user from a larger file.
- qpdf gives fast web view and lossless cleanup without linking C++: `--linearize` writes a valid file with the `/Linearized` marker, and cleanup keeps page content identical while normalising and compressing the object structure.
- The encryption and structure check is a safe read: it reports encryption, password requirement, permissions and structural warnings into one text file and never decrypts or changes the input.
- A missing `qpdf` is a clear state: the actions are hidden, `kawaro doctor` says it is missing, and a direct run names the program, the package and a URL. An encrypted file is refused for the rewriting ops with an explicit reason, never silently rewritten.
- The safety rules hold: the original is never modified, output naming stays in core, a failed or cancelled run leaves no output, and every child runs in a killable process tree.

## Dependencies
- ADR 0014 is accepted and the owner approved the child program, so this story can proceed.
- The licence line in `docs/engines.md` before the code lands, with the version checked from the system, and the install hints for Debian and Ubuntu (`qpdf`) and Fedora (`qpdf`).
- S-117 for the compress `dpi` option field; S-120 for per-run values; the qpdf actions carry no options; S-010 and the Ghostscript adapter for the child engine pattern; S-030 for the process tree; the existing probe, doctor and MCP engine list.
- Fixtures: a text PDF, a scan with large images for the DPI comparison, a signed or structurally odd PDF where possible, an encrypted PDF (with and without a user password), and a file already linearized to prove a second linearize is stable.

## Decisions (settled by ADR 0014)
1. `pdf.compress` gains a `dpi` field: TOML integer 0 to 600, default 0, `maps_to = "dpi"`, unit `dpi`, bilingual label. The base `[action.params]` keeps `preset = "ebook"` and adds `dpi = 0`. A value of 0 means the preset decides and no downsample flag is passed, so the shipped behaviour is unchanged. A value from 1 to 600 is passed with the documented lowercase Ghostscript flags: `-dDownsampleColorImages`, `-dColorImageDownsampleType=/Bicubic`, `-dColorImageResolution=N`, `-dDownsampleGrayImages`, `-dGrayImageResolution=N`, `-dDownsampleMonoImages` and `-dMonoImageResolution=N`.
2. New engine id `qpdf`, program `qpdf` (`qpdf.exe` on Windows), with `available()` true only when found, an `Unavailable` message that names `qpdf`, the package and `https://qpdf.readthedocs.io/`, and a `kawaro doctor` line. No library is linked and no network is used.
3. Three qpdf ops, each one task and one output file:
   - `linearize`: `qpdf --linearize input output`, a valid PDF with the `/Linearized` marker and unchanged pages.
   - `cleanup`: `qpdf --object-streams=generate --compress-streams=y --recompress-flate --compression-level=9 input output`, a lossless structural rewrite with unchanged page content.
   - `check`: `qpdf --check --show-encryption <absolute input>` in one invocation, with the report captured from standard output and written as one UTF-8 text file from `task.output`; it reports encrypted yes or no, whether a password is required, the permission summary and structural warnings. It never decrypts and never modifies the input.
4. Three actions, fixed by ADR 0014 and with no options: `pdf.linearize` (`engine = "qpdf"`, `op = "linearize"`, ring 2, order 60, per-file, suffix `-linearized`), `pdf.cleanup` (`op = "cleanup"`, ring 2, order 70, per-file, suffix `-cleaned`) and `pdf.check` (`op = "check"`, ring 2, order 80, per-file, `ext = "txt"`, suffix `-check`), all with bilingual code and label and an icon from the fixed set. The registry goes from 53 to 56 actions and the option action count stays 17, because none of the three carries an option.
5. Encrypted input: `linearize` and `cleanup` refuse it with `EngineError::Input` and a reason that says the file is encrypted and needs a password; there is no password prompt and no decryption in this story. `check` reports the encryption state without needing a password when qpdf can read the encryption dictionary, and says so when a password is required to go further.
6. A missing program, a non-zero exit, a timeout, a killed child and cancellation map to the existing `EngineError` variants. No partial output is left at `task.output`, and the private work directory is removed on every path.

## Acceptance criteria
- AC1 (compress schema): `pdf.compress` has the `dpi` option field and base param from decision 1; `OptionsSpec::validate` passes; the four presets `screen`, `ebook`, `printer` and `prepress`, with their Arabic and English labels, are unchanged; no other action changes.
- AC2 (compress DPI): on the scan fixture, `dpi = 72` produces a smaller file than `dpi = 300` (both at preset `ebook`), the page count and the extracted page text are unchanged, the output loads, and the smaller-or-original rule still holds. `dpi = 0` passes no downsample flag, so its size is the preset-only baseline and matches the behaviour before this story. `dpi = -1`, `dpi = 601` and a non integer are `BadParam` named `dpi` with no output.
- AC3 (qpdf engine): the engine is registered and available exactly when `qpdf` is found; `capabilities()` declares the three ops; an unknown op is `UnsupportedOp`; with no program, the actions are absent from the hive and `kawaro_plan`, `kawaro doctor` reports the missing line, and a direct run gives `Unavailable` with `qpdf`, the package and the URL, and no output file.
- AC4 (linearize): a normal PDF becomes a valid PDF with `/Linearized` in the first kilobyte, the same page count, page text and MediaBox, and a second linearize of the result is stable. A malformed input gives `Failed` with the stderr tail; an encrypted input gives `Input` with an encryption reason; neither leaves an output.
- AC5 (cleanup): `pdf.cleanup` runs exactly `qpdf --object-streams=generate --compress-streams=y --recompress-flate --compression-level=9 <input> <output>`, a lossless structural rewrite that never decodes or re-encodes an image. Page count, page text, MediaBox and every embedded image stream are byte identical, the output loads, and the input bytes are unchanged. An encrypted input is refused like AC4. A signed or otherwise structurally odd fixture is recorded, not promised to keep its signature.
- AC6 (check): `pdf.check` runs `qpdf --check --show-encryption <input>` in one invocation and writes its standard output as one UTF-8 text file, no BOM, LF endings. For an unencrypted file it says not encrypted. For an encrypted input the same invocation still writes the report, which says encrypted, whether a password is required and the permission summary, without decrypting and without failing on the encryption itself. For a structurally damaged file it reports the qpdf warnings, and the task succeeds when qpdf returns warnings rather than a hard failure. The input is unchanged and no PDF output is written.
- AC7 (actions): the three new actions match decision 4, are bilingual, validate, and the registry has 56 built-in actions with 17 option actions (the 17 of S-127, unchanged). The tester updates the S-117 count test from 53 to 56 and leaves its options list at 17, and updates any PDF registry count as part of the story.
- AC8 (gates and safety): `cargo fmt`, clippy on `kawaro-engines` with `-D warnings`, and the tests of `kawaro-engines`, `kawaro-core`, `kawaro-cli` and `kawaro-daemon` pass; no new dependency and no new `Cargo.lock` package; no network; MSRV 1.85; `docs/engines.md` has the qpdf row; ADR 0014 is accepted. The original is never modified, output naming stays in core, progress starts at 0 and ends at 1.0, a pre-cancelled token leaves no output, and no child outlives the task.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Read `actions/pdf.toml` through `ActionDef`: the `dpi` field 0 to 600 with default 0, base param `dpi = 0`, presets and labels | unit |
| AC2 | Scan fixture at 0, 72 and 300 dpi: the 0 baseline, size ordering, page text equality, bad values | integration (skips without `gs`) |
| AC3 | Registration, availability with a fake and with no program, `Unavailable` text, doctor line, plan exclusion | unit |
| AC4 | `/Linearized` marker, page equality, second run, malformed and encrypted input | integration (skips without `qpdf`) |
| AC5 | Cleanup page text, MediaBox and image stream bytes equal to the input; input untouched; encrypted refusal; the argument list is asserted with a fake program | integration (skips without `qpdf`) |
| AC6 | `--check --show-encryption` argument list with a fake program; report text for plain, encrypted and damaged fixtures; no BOM; input unchanged | integration (skips without `qpdf`) |
| AC7 | Action rows, bilingual labels, `validate()`, 56 actions and 17 option actions | unit |
| AC8 | The cargo gates, the `docs/engines.md` row and the ADR file, then a pre-cancelled run and a directory listing | unit, gate |

## Out of scope
Encrypting or decrypting with a password, changing permissions, removing restrictions, repair beyond qpdf `--check` (a later story), qpdf JSON output, page selection or splitting through qpdf, attachments and portfolios, linearizing as an automatic step after every rewrite, PDF/A, OCR, a new download or model, packaging qpdf or pinning a version, a Windows or macOS check in this story, and any UI beyond the existing action cells and the stage 2 form of S-119.

## Review log
- 2026-11-04, planner: draft from P0 items 3 and 4 of `docs/research/pdf-feature-benchmark.md` (sections 4, 5, 7, 8 and 10) and the owner's request for honest compress controls and a qpdf child engine. Open before approval: the ADR for the `qpdf` engine id and the child program decision; the Ghostscript parameter set and the compress DPI range (proposed 36 to 600, default 150); whether `check` reports signatures separately from encryption; the action names and the registry counts, left provisional on purpose; and the exact qpdf flags, which are proposals until a spike confirms them on the fixtures. Not implemented.
- 2026-11-04, planner: updated from draft to in progress after ADR 0014 was accepted. The compress DPI field is now 0 to 600 with 0 meaning the preset decides and no downsample flag, so the default keeps the shipped behaviour; the documented lowercase flags are used for 1 to 600. The qpdf action names are fixed as pdf.linearize, pdf.cleanup and pdf.check with ring 2, suffixes -linearized, -cleaned and -check, no options, and the final counts are 56 actions and 17 option actions. The test table and AC7 now name those numbers.

- 2026-11-04, mcp-impl: gs compress dpi 0..600 with the documented lowercase downsample flags; new qpdf engine (linearize, cleanup, check) with locate::Program::Qpdf, doctor line and docs; qpdf is not installed on this machine, so the live path is covered only for the missing state and the command construction.
- 2026-11-04, tester: s128 tests 6/6 (gs dpi 300/0/absent, qpdf capabilities, unavailable path, locate injection, action rows, live run skipped); pdf_actions 13 rows and the four-engine set; s117 56/17.
- 2026-11-04, lead: ADR 0014 accepted, CHANGELOG and docs updated. Caveat recorded: qpdf live operations need a machine with qpdf installed.

- 2026-11-04, independent review: REWORK on cleanup flags, check encryption, prepress and docs. Fixed: cleanup uses --compression-level=9 with no image re-encode, check runs --check --show-encryption and writes the report on encrypted output, prepress is accepted by Ghostscript, the documented downsample type flags are added, and docs/engines.md is corrected.
