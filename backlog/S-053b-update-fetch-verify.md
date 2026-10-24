# S-053b Update check and verified download
Status: approved (manager, by delegation, 2026-10-24)

Second core story of ADR 0006. Depends on S-053a. The network goes through a `Fetcher` trait so every test uses a fake or a fake `curl` script; no test touches the internet. About one work day.

## Story
As a user, I want a downloaded update checked against the signed hash before anything runs it, so that a corrupted or tampered file is never applied.

## Acceptance criteria
- AC1 (`crates/core/src/update/fetch.rs`): `trait Fetcher { fn get(&self, url: &str, dest: &Path, max_bytes: u64) -> Result<(), FetchError> }` with `FetchError::{Offline, Http(u16), TooLarge, Unavailable}`. `pub fn curl_args(url, dest, max_bytes) -> Vec<String>` returns exactly `--silent --show-error --fail --location --proto =https --proto-redir =https --max-redirs 5 --max-time 30 --max-filesize <max_bytes> --user-agent sbarhive-update --output <dest> <url>` (30 becomes 1800 for a package). No `--cookie`, `--header` or `--referer`, and the text of the arguments contains no version number.
- AC2 (`fetch.rs`): `CurlFetcher::with_program(path)` runs that program; `CurlFetcher::system()` uses `/usr/bin/curl` then `/bin/curl`, or `%SystemRoot%\System32\curl.exe`, never `PATH`; none present gives `Unavailable`. With a fake script exiting 6 (cannot resolve) the result is `Offline`; exiting 22 is `Http`; exiting 63 is `TooLarge`; exiting 0 after writing a file is `Ok`.
- AC3 (`update/flow.rs`): `check(&Config, &mut State, &dyn Fetcher, now, manual: bool) -> CheckOutcome` with `Disabled | NotDue | UpToDate | Available(Update) | Offline | Unavailable | Rejected(UpdateError)`. Automatic with `update_check = false`: `Disabled` and 0 fetch calls. Automatic and not `due`: `NotDue`, 0 calls. A manual check ignores both. A real check makes exactly 2 calls: `<base>/<channel>.json` then `<base>/<channel>.json.minisig`, with `<base>` the `updates` release URL of ADR 0006.
- AC4 (`flow.rs`): state after a check. Fetched and verified: `last_check = now`, `failures = 0`, `last_seen = version`. `Offline`: `last_check` unchanged, `failures` plus 1, `last_failure = now`. `Rejected` (bad signature, channel, URL): `failures` plus 1, and the outcome carries the error; a downgraded manifest is `UpToDate`, not an error.
- AC5 (`flow.rs`): `download_verified(&Update, dir, &dyn Fetcher) -> Result<VerifiedPackage, UpdateError>` downloads to `<dir>/<name>.part` with `max_bytes = size`, checks the length and the SHA-256 (streamed, 64 KiB reads), then renames to `<dir>/<name>`. Matching bytes: `Ok`, final file present, no `.part`. One flipped byte: `Err(HashMismatch)`, final file absent, `.part` absent. A fetcher that returns more bytes than `size`: `Err(SizeMismatch)`, both absent. `VerifiedPackage` has no public constructor outside this function.
- AC6 (`flow.rs`): after a successful download, every other version folder under `<cache>/updates/` is deleted. Offline during download: `Err(Offline)`, nothing is left in the cache folder.
- AC7 (`paths.rs`): `cache_dir_with(os, env, known)` is pure: Linux `XDG_CACHE_HOME/sbarhive`, else `HOME/.cache/sbarhive`; Windows `%LOCALAPPDATA%\SbarHive`; the update folder is `<cache>/updates/<version>`. `update-state.json` sits next to `config.toml`.
- AC9 (`fetch.rs`, added by the manager after the S-053a security review, 2026-10-24): `CurlFetcher::get` and `check` refuse, before running curl, any url that contains `..`, a percent encoded dot (`%2e` or `%2E`), a backslash, whitespace or a control character, with `UpdateError::BadUrl`; the signed url prefix check of S-053a stays as it is. A duplicate asset entry in a manifest never changes which asset `select_asset` returns for a platform (the first match), and `download_verified` verifies the size and the SHA-256 of that one.
- AC8 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed non Windows files, `scripts/windows/check.sh` exits 0; one CHANGELOG line, English and Arabic.

## Test cases
The fake `Fetcher` serves the S-053a fixtures from memory and counts calls. The fake `curl` is a shell script in a temp dir that writes its own argv to a file.
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `curl_args` equals the expected vector (manifest and package variants); no argument matches a version regex | unit |
| AC2 | Fake script with exit 6, 22, 63, 0: the mapped result; the recorded argv equals `curl_args`; empty `with_program` path gives `Unavailable` | integration |
| AC3 | `update_check=false` automatic: calls 0; manual: calls 2 and the URLs asserted in order; not due: calls 0 | unit |
| AC4 | Four runs (good, offline, bad signature, lower version) with the state compared field by field | unit |
| AC5 | Three byte sets (exact, one byte flipped, longer than size): outcome plus a directory listing of `dir` | unit |
| AC6 | Two stale version folders plus a good download: only the new one remains; offline run leaves an empty folder | unit |
| AC7 | Table of environments for both platforms | unit |

## Out of scope
- Applying the package (S-054a), scheduling (S-054b).
- A native HTTP client (ADR 0006 section 4).

## CHANGELOG plan
- Update download (S-053b): the signed manifest is fetched, and a package is kept only when its size and SHA-256 match. «تنزيل التحديث: يُجلب البيان الموقّع ولا تُحفظ الحزمة إلا إذا طابق حجمها وبصمتها».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. `.part` removal on failure is asserted by listing the folder, not by the absence of an error.
