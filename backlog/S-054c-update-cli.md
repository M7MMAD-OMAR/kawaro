# S-054c The `sbarhive update` command
Status: approved (manager, by delegation, 2026-10-24)

Third surface story of ADR 0006. Depends on S-053b and S-054a. The command uses `sbarhive-core` directly, like the rest of the CLI; it does not need the daemon. Under half a work day.

## Story
As a user or a script, I want `sbarhive update` to check for and install an update from a terminal, so that I can update without a graphical session.

## Acceptance criteria
- AC1 (`crates/cli/src/update.rs`): `run(args, &dyn Fetcher, &Facts, out, err) -> i32`. `sbarhive update --check` prints exactly three lines on stdout, `current <v>`, `latest <v>`, `status <s>`, with `<s>` one of `up-to-date`, `update-available`, `offline`, `unavailable`, `rejected`. A manual check runs even when `update_check = false`.
- AC2 (exit codes): `0` for `up-to-date` and for a finished install; `10` for `update-available`; `3` for `offline`; `4` for `unavailable` (no curl); `5` for `rejected` (the error text goes to stderr, one line); `2` for a usage error.
- AC3 (`sbarhive update`, no flag): same as `--check`, then if `update-available` downloads and verifies, prints `downloaded <name>` and `verified sha256 <first 12 hex>`, and exits 10 without installing. `sbarhive update --apply` does all of it and installs through `apply_now`, printing `installed <v>` and exiting 0, or the failure line and exit 1.
- AC4 (`--status`): prints `status none`, `status ready <v>` or `status failed <v>` from `update-state.json` and `update-result.json` with no network access (0 fetch calls) and exit 0.
- AC5 (`--help`): lists `--check`, `--apply`, `--status`, `--channel stable|beta` (overrides the config for this run) in English; an unknown channel exits 2 with `unknown channel`. The text of every message is available in Arabic and English by `language`.
- AC6 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed files, one CHANGELOG line, English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Fake fetcher for each of the five outcomes: capture stdout, stderr and the return code; compare the three lines byte for byte | unit |
| AC3 | Fake fetcher with fixture packages, good and tampered: the printed lines, exit codes, and that the fake installer has 0 calls without `--apply` and 1 with it; tampered gives exit 5 and 0 calls | integration |
| AC4 | State files in a temp config dir; the fetcher records 0 calls | unit |
| AC5 | `--help` snapshot text; `--channel nightly` gives exit 2; a run with `language = "ar"` prints the Arabic status words | unit |
| AC1 | The real binary: `sbarhive update --status` in an empty config dir prints `status none` and exits 0 | integration |

## Out of scope
- Installing from a local file the user chose.
- Any output that needs the daemon.

## CHANGELOG plan
- Update command (S-054c): `sbarhive update` checks, downloads, verifies and installs from the terminal, with stable exit codes. «أمر التحديث: يفحص وينزّل ويتحقق ويثبّت من الطرفية برموز خروج ثابتة».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. Binary level tests avoid the network: only `--status` runs the real binary.
