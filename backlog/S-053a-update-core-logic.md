# S-053a Update core logic: versions, manifest, signature, schedule, settings
Status: review

First of three core stories of ADR 0006 (S-053a to S-053c). Pure code only, no network, no files outside tests. Needs P1 (public key committed at `packaging/keys/sbarhive-update.pub`) and P2 (`minisign` to make fixtures, committed once; tests never run it and no secret key is committed). About one work day.

## Story
As a user, I want the app to judge an update by exact rules, so that only a newer, signed release of my channel can ever be offered.

## Acceptance criteria
- AC1 (`crates/core/src/update/version.rs`): `pub fn is_newer(installed: &str, candidate: &str) -> Result<bool, UpdateError>` on semver. True for `0.3.3`<`0.4.0`, `0.4.0-beta.1`<`0.4.0`, `0.4.0-beta.1`<`0.4.0-beta.2`; false for equal and lower; `Err(BadVersion)` for `""`, `v0.4.0`, `0.4`.
- AC2 (`update/manifest.rs`): `Manifest::parse(&[u8])` reads schema 1 (fields in ADR 0006 section 2), ignores unknown keys, and returns `Err` for: `schema` other than 1 (`UnsupportedSchema`), a `sha256` that is not 64 lowercase hex, `size` 0, a `kind` outside `msi|rpm|deb`, invalid JSON.
- AC3 (`update/policy.rs`): `evaluate(&Manifest, installed: &str, channel: Channel) -> Result<Decision, UpdateError>`: channel mismatch is `Err(ChannelMismatch)`; version not newer is `Ok(UpToDate)`; any asset `url` not starting with `https://github.com/M7MMAD-OMAR/sbarhive/releases/download/` is `Err(BadUrl)` (including `http://` and a lookalike host); else `Ok(Available(Update))`.
- AC4 (`policy.rs`): `select_asset(&Manifest, Platform { os, arch, pkg })` returns the one asset for (`windows`,`x86_64`,`msi`), (`linux`,`x86_64`,`rpm`) or (`linux`,`x86_64`,`deb`); `None` for `aarch64` or a missing kind.
- AC5 (`update/verify.rs`): `verify_manifest(bytes: &[u8], minisig: &str, keys: &[&str]) -> Result<(), UpdateError>`. `Ok` for `tests/fixtures/update/manifest-stable-0.4.0.json` with its `.minisig` and `key-a.pub`; `Err(BadSignature)` after flipping one byte of the manifest, with the signature of `key-b` (`manifest-stable-0.4.0.json.b.minisig`) and only `key-a`, and with an empty key list; `Ok` when `keys = [key-b, key-a]`.
- AC6 (`verify.rs`): `UPDATE_KEYS` is non empty and every entry parses as a minisign public key (a placeholder fails); `minisign-verify` licence and dependency list verified with `cargo info` and recorded in `docs/engines.md`; if it is not MIT, Apache-2.0 or BSD, or has dependencies not in `Cargo.lock`, the ADR fallback is used instead.
- AC7 (`update/schedule.rs`): `due(&State, now_secs, interval_secs) -> bool` with the 24 h interval: never checked true; 23 h 59 min after `last_check` false; 24 h true; `now` earlier than `last_check` true; with `failures = n` the wait is `min(2^(n-1) h, 24 h)` from `last_failure` (1 h, 2 h, 4 h, 24 h at n=6). `State` round trips through JSON, and a corrupt text gives `State::default()`.
- AC8 (`config.rs`): keys `update_check` (default `true`), `update_channel` (`stable`|`beta`, default `stable`), `update_mode` (`ask`|`auto`, default `ask`), underscore names with explicit `rename`. `update_channel = "nightly"` makes `Config::load_from` return defaults and a warning naming the file; the other keys keep working in a file without these.
- AC9 (gates): fmt, clippy, `cargo test --all`, `scripts/design-lint.sh` clean; `cargo mutants --in-place --in-diff` gives 0 missed; one CHANGELOG line, English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | The table above, asserting the exact bool or the error variant | unit |
| AC2 | Eight inputs from `tests/fixtures/update/`: the valid manifest gives 3 assets with the exact `sha256` strings; each bad file gives its named error | unit |
| AC3 | Manifests edited in memory: other channel, same version, lower version, `http://` url, `https://github.com.evil.example/...` url, valid | unit |
| AC4 | The three platforms return the asset with the expected `name`; aarch64 gives `None` | unit |
| AC5 | The five signature cases with committed fixtures | unit |
| AC6 | Parse every `UPDATE_KEYS` entry; `grep -c "minisign-verify" docs/engines.md` is at least 1 | unit, review |
| AC7 | The time table; 6 consecutive failures; garbage and empty state text | unit |
| AC8 | Defaults, a partial file, the `nightly` value | unit |

## Out of scope
- Network, downloads, applying (S-053b, S-054a).
- Settings window rows (ui work, later).

## CHANGELOG plan
- Update rules (S-053a): versions, the signed manifest, channels and the check schedule are decided in code; nothing is downloaded yet. «قواعد التحديث: الإصدارات والبيان الموقّع والقنوات وجدول الفحص، دون تنزيل بعد».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. The `.b.minisig` and `key-b.pub` fixtures are made once with `minisign`; no secret key is committed.
- 2026-10-24, tester: tests in `crates/core/tests/s053a_update.rs` (52 tests, one or more per AC1 to AC8; AC6 licence part is review, AC9 is the gates), fixtures in `tests/fixtures/update/` made with throwaway minisign keys (secret halves deleted). Run: `cargo test -p sbarhive-core --test s053a_update`. Result before implementation: the test target does not compile, for the right reason (31 errors): `E0432 unresolved import sbarhive_core::update`, `E0432 unresolved import sbarhive_core::config::UpdateMode`, `E0433 cannot find update in sbarhive_core` (x5), `E0609 no field update_check / update_channel / update_mode on type Config` (x21), `E0560 struct Config has no field named update_check / update_channel / update_mode` (x3). The contract is the header comment of the test file. Assumptions: error variants other than BadVersion, UnsupportedSchema, ChannelMismatch, BadUrl, BadSignature are named by the tester (InvalidJson, BadSha256, BadSize, BadKind, BadKey); the default channel rule is `update::default_channel`, `Config::default().update_channel` stays `Stable` as AC8 says; manual bypass is `schedule::allowed`.
- 2026-10-24, implementer: `crates/core/src/update/{mod,version,manifest,policy,verify,schedule}.rs` and the three `Config` keys. `minisign-verify` 0.3.0 is MIT with no dependencies (`cargo info`), so it is used and the fallback is not; `semver` 1.0.28 became a direct dependency of core. Cargo.lock gains 1 package (minisign-verify). All 52 tests of `s053a_update` pass. Deviations: none; the backoff cap is the fixed 24 h of ADR 0006, not the interval argument. Open point for S-054b: the beta-default channel (`update::default_channel`) needs to know whether `update_channel` was set explicitly, but `Config` holds a plain `Channel` defaulting to `Stable`, so the daemon cannot tell "unset" from "set to stable"; S-054b must make the field an `Option<Channel>` or read the key presence from the file.
