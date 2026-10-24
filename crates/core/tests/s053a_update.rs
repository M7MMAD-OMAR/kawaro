//! S-053a (backlog/S-053a-update-core-logic.md, docs/adr/0006-auto-update.md):
//! the pure update rules. Written by the tester from the acceptance criteria.
//!
//! Contract (do not edit these tests; the implementer follows it):
//! - `sbarhive_core::update` (crate::update, `crates/core/src/update/mod.rs`) holds
//!   `UpdateError`, `Channel` and the sub modules `version`, `manifest`, `policy`,
//!   `verify`, `schedule`, all `pub`.
//! - `UpdateError`: `Debug + Clone + PartialEq + Eq`, unit variants `BadVersion`,
//!   `UnsupportedSchema`, `InvalidJson`, `BadSha256`, `BadSize`, `BadKind`,
//!   `ChannelMismatch`, `BadUrl`, `BadSignature`, `BadKey`.
//! - `Channel { Stable, Beta }`: `Debug + Clone + Copy + PartialEq + Eq + Default`
//!   (default `Stable`), serde lowercase (`"stable"`, `"beta"`).
//!   `update::default_channel(installed: &str) -> Result<Channel, UpdateError>`:
//!   `Beta` when the installed version has a semver prerelease part, else `Stable`
//!   (manager decision of 2026-10-24); `Err(BadVersion)` for a bad version.
//! - `version::is_newer(installed: &str, candidate: &str) -> Result<bool, UpdateError>`.
//! - `manifest::Manifest` (`Debug + Clone + PartialEq`), pub fields `schema: u32`,
//!   `channel: Channel`, `version: String`, `published: String`, `notes_url: String`,
//!   `assets: Vec<Asset>`. `manifest::Asset` (`Debug + Clone + PartialEq`), pub fields
//!   `kind: String`, `os: String`, `arch: String`, `name: String`, `url: String`,
//!   `size: u64`, `sha256: String`. `Manifest::parse(&[u8]) -> Result<Manifest, UpdateError>`.
//!   The schema is checked first: `{"schema":2}` is `UnsupportedSchema`. Any other
//!   structural problem (not JSON, not UTF-8, missing or mistyped field, unknown
//!   channel) is `InvalidJson`.
//! - `policy::evaluate(&Manifest, installed: &str, Channel) -> Result<Decision, UpdateError>`,
//!   order of checks as in ADR 0006 section 3: channel, then version, then urls (every
//!   asset). `Decision` (`Debug + PartialEq`): `UpToDate`, `Available(Update)`;
//!   `Update` has pub `version: String` and `notes_url: String`.
//!   `policy::Platform { os: String, arch: String, pkg: String }` and
//!   `policy::select_asset(&Manifest, Platform) -> Option<&Asset>`.
//! - `verify::verify_manifest(bytes: &[u8], minisig: &str, keys: &[&str]) -> Result<(), UpdateError>`.
//!   A key entry is the text of a minisign `.pub` file (two lines) or its bare base64
//!   line. A signature text that does not parse is `BadSignature`.
//!   `verify::UPDATE_KEYS: &[&str]`, `verify::parse_key(&str) -> Result<(), UpdateError>`
//!   (`Err(BadKey)` when it is not a minisign public key).
//! - `schedule::State { last_check: Option<u64>, last_failure: Option<u64>, failures: u32 }`
//!   (`Debug + Clone + PartialEq + Default`), `State::to_json(&self) -> String`,
//!   `State::from_json(&str) -> State` (missing fields default; anything that is not
//!   a valid object gives `State::default()`). `schedule::due(&State, now_secs: u64,
//!   interval_secs: u64) -> bool`, `schedule::allowed(update_check: bool, manual: bool) -> bool`.
//! - `config::UpdateMode { Ask, Auto }` (serde lowercase, default `Ask`);
//!   `Config` gains `update_check: bool`, `update_channel: update::Channel`,
//!   `update_mode: config::UpdateMode`, keys spelled with underscores.

use std::path::{Path, PathBuf};

use sbarhive_core::config::{Config, UpdateMode};
use sbarhive_core::update::manifest::Manifest;
use sbarhive_core::update::policy::{Decision, Platform, evaluate, select_asset};
use sbarhive_core::update::schedule::{State, allowed, due};
use sbarhive_core::update::verify::{UPDATE_KEYS, parse_key, verify_manifest};
use sbarhive_core::update::version::is_newer;
use sbarhive_core::update::{Channel, UpdateError, default_channel};

const HOUR: u64 = 3600;
const DAY: u64 = 86_400;
const URL_PREFIX: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fx(name: &str) -> PathBuf {
    root().join("tests/fixtures/update").join(name)
}

fn fx_bytes(name: &str) -> Vec<u8> {
    std::fs::read(fx(name)).unwrap()
}

fn fx_text(name: &str) -> String {
    std::fs::read_to_string(fx(name)).unwrap()
}

fn valid() -> Manifest {
    Manifest::parse(&fx_bytes("manifest-stable-0.4.0.json")).unwrap()
}

/// The valid fixture edited in memory, then parsed.
fn edited(f: impl FnOnce(&mut serde_json::Value)) -> Manifest {
    let mut v: serde_json::Value =
        serde_json::from_slice(&fx_bytes("manifest-stable-0.4.0.json")).unwrap();
    f(&mut v);
    Manifest::parse(&serde_json::to_vec(&v).unwrap()).unwrap()
}

fn edited_err(f: impl FnOnce(&mut serde_json::Value)) -> UpdateError {
    let mut v: serde_json::Value =
        serde_json::from_slice(&fx_bytes("manifest-stable-0.4.0.json")).unwrap();
    f(&mut v);
    Manifest::parse(&serde_json::to_vec(&v).unwrap()).unwrap_err()
}

// ---------------------------------------------------------------- AC1

#[test]
fn ac1_is_newer_true_cases() {
    assert_eq!(is_newer("0.3.3", "0.4.0"), Ok(true));
    assert_eq!(is_newer("0.4.0-beta.1", "0.4.0"), Ok(true));
    assert_eq!(is_newer("0.4.0-beta.1", "0.4.0-beta.2"), Ok(true));
    // Numeric, not textual, comparison.
    assert_eq!(is_newer("0.9.0", "0.10.0"), Ok(true));
    assert_eq!(is_newer("0.4.0-beta.2", "0.4.0-beta.10"), Ok(true));
    assert_eq!(is_newer("0.3.9", "0.4.0-beta.1"), Ok(true));
    assert_eq!(is_newer("0.4.0", "0.4.1"), Ok(true));
    assert_eq!(is_newer("0.4.9", "1.0.0"), Ok(true));
}

#[test]
fn ac1_is_newer_false_for_equal_and_lower() {
    assert_eq!(is_newer("0.4.0", "0.4.0"), Ok(false));
    assert_eq!(is_newer("0.4.0-beta.1", "0.4.0-beta.1"), Ok(false));
    assert_eq!(is_newer("0.4.0", "0.3.3"), Ok(false));
    assert_eq!(is_newer("0.4.0", "0.4.0-beta.1"), Ok(false));
    assert_eq!(is_newer("0.4.0-beta.2", "0.4.0-beta.1"), Ok(false));
    assert_eq!(is_newer("0.10.0", "0.9.0"), Ok(false));
    assert_eq!(is_newer("1.0.0", "0.99.99"), Ok(false));
    assert_eq!(is_newer("0.4.1", "0.4.0"), Ok(false));
}

#[test]
fn ac1_is_newer_bad_versions_are_bad_version_on_either_side() {
    for bad in ["", "v0.4.0", "0.4", "0.4.0 ", " 0.4.0", "0.4.0.1", "x.y.z"] {
        assert_eq!(
            is_newer(bad, "0.4.0"),
            Err(UpdateError::BadVersion),
            "{bad:?}"
        );
        assert_eq!(
            is_newer("0.4.0", bad),
            Err(UpdateError::BadVersion),
            "{bad:?}"
        );
        assert_eq!(is_newer(bad, bad), Err(UpdateError::BadVersion), "{bad:?}");
    }
}

#[test]
fn ac1_default_channel_follows_the_prerelease_part() {
    assert_eq!(default_channel("0.4.0-beta.1"), Ok(Channel::Beta));
    assert_eq!(default_channel("1.0.0-rc.1"), Ok(Channel::Beta));
    assert_eq!(default_channel("0.4.0"), Ok(Channel::Stable));
    assert_eq!(default_channel("0.3.3"), Ok(Channel::Stable));
    // Build metadata is not a prerelease part.
    assert_eq!(default_channel("0.4.0+build.5"), Ok(Channel::Stable));
    assert_eq!(default_channel(""), Err(UpdateError::BadVersion));
    assert_eq!(
        default_channel("v0.4.0-beta.1"),
        Err(UpdateError::BadVersion)
    );
}

// ---------------------------------------------------------------- AC2

#[test]
fn ac2_valid_manifest_gives_three_assets_with_exact_values() {
    let m = valid();
    assert_eq!(m.schema, 1);
    assert_eq!(m.channel, Channel::Stable);
    assert_eq!(m.version, "0.4.0");
    assert_eq!(m.published, "2026-10-24T12:00:00Z");
    assert_eq!(
        m.notes_url,
        "https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v0.4.0"
    );
    assert_eq!(m.assets.len(), 3);
    let a = &m.assets;
    assert_eq!(
        (a[0].kind.as_str(), a[0].os.as_str(), a[0].arch.as_str()),
        ("msi", "windows", "x86_64")
    );
    assert_eq!(a[0].name, "SbarHive-0.4.0-x86_64.msi");
    assert_eq!(
        a[0].url,
        format!("{URL_PREFIX}v0.4.0/SbarHive-0.4.0-x86_64.msi")
    );
    assert_eq!(a[0].size, 4_194_304);
    assert_eq!(a[0].sha256, "1".repeat(64));
    assert_eq!(
        (a[1].kind.as_str(), a[1].os.as_str(), a[1].arch.as_str()),
        ("rpm", "linux", "x86_64")
    );
    assert_eq!(a[1].name, "sbarhive-0.4.0-1.x86_64.rpm");
    assert_eq!(a[1].size, 3_145_728);
    assert_eq!(
        a[1].sha256,
        "a3f1c2d4e5b60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90"
    );
    assert_eq!(
        (a[2].kind.as_str(), a[2].os.as_str(), a[2].arch.as_str()),
        ("deb", "linux", "x86_64")
    );
    assert_eq!(a[2].name, "sbarhive_0.4.0_amd64.deb");
    assert_eq!(a[2].size, 2_097_152);
    assert_eq!(
        a[2].sha256,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
}

#[test]
fn ac2_unknown_keys_are_ignored() {
    let m = Manifest::parse(&fx_bytes("manifest-unknown-keys.json")).unwrap();
    assert_eq!(m, valid());
}

#[test]
fn ac2_each_bad_fixture_gives_its_named_error() {
    let cases = [
        ("manifest-bad-schema.json", UpdateError::UnsupportedSchema),
        ("manifest-bad-sha-short.json", UpdateError::BadSha256),
        ("manifest-bad-sha-upper.json", UpdateError::BadSha256),
        ("manifest-bad-size-zero.json", UpdateError::BadSize),
        ("manifest-bad-kind.json", UpdateError::BadKind),
        ("manifest-bad-json.json", UpdateError::InvalidJson),
    ];
    for (file, err) in cases {
        assert_eq!(Manifest::parse(&fx_bytes(file)), Err(err), "{file}");
    }
}

#[test]
fn ac2_sha256_must_be_exactly_64_lowercase_hex() {
    let set =
        |s: String| edited_err(move |v| v["assets"][1]["sha256"] = serde_json::Value::String(s));
    assert_eq!(set("a".repeat(63)), UpdateError::BadSha256);
    assert_eq!(set("a".repeat(65)), UpdateError::BadSha256);
    assert_eq!(set(String::new()), UpdateError::BadSha256);
    assert_eq!(set(format!("{}g", "a".repeat(63))), UpdateError::BadSha256);
    assert_eq!(set(format!("{}A", "a".repeat(63))), UpdateError::BadSha256);
    assert_eq!(set(format!("{} ", "a".repeat(63))), UpdateError::BadSha256);
    // The check covers every asset, not only the first.
    assert_eq!(
        edited_err(|v| v["assets"][2]["sha256"] = "z".repeat(64).into()),
        UpdateError::BadSha256
    );
    // Digits and the full lowercase range are fine.
    let ok = edited(|v| v["assets"][1]["sha256"] = "0123456789abcdef".repeat(4).into());
    assert_eq!(ok.assets[1].sha256, "0123456789abcdef".repeat(4));
}

#[test]
fn ac2_size_zero_is_bad_and_one_is_fine() {
    assert_eq!(
        edited_err(|v| v["assets"][0]["size"] = 0.into()),
        UpdateError::BadSize
    );
    assert_eq!(
        edited_err(|v| v["assets"][2]["size"] = 0.into()),
        UpdateError::BadSize
    );
    let m = edited(|v| v["assets"][1]["size"] = 1.into());
    assert_eq!(m.assets[1].size, 1);
}

#[test]
fn ac2_kind_is_msi_rpm_or_deb_only() {
    for bad in ["exe", "MSI", "", "rpm ", "apk"] {
        assert_eq!(
            edited_err(|v| v["assets"][1]["kind"] = bad.into()),
            UpdateError::BadKind,
            "{bad:?}"
        );
    }
}

#[test]
fn ac2_schema_is_checked_first_and_must_be_one() {
    assert_eq!(
        Manifest::parse(br#"{"schema":2}"#),
        Err(UpdateError::UnsupportedSchema)
    );
    assert_eq!(
        Manifest::parse(br#"{"schema":0}"#),
        Err(UpdateError::UnsupportedSchema)
    );
    assert_eq!(
        edited_err(|v| v["schema"] = 3.into()),
        UpdateError::UnsupportedSchema
    );
    // A schema 2 manifest with an unknown channel is still an unsupported schema.
    assert_eq!(
        edited_err(|v| {
            v["schema"] = 2.into();
            v["channel"] = "nightly".into();
        }),
        UpdateError::UnsupportedSchema
    );
}

#[test]
fn ac2_structural_problems_are_invalid_json() {
    for bytes in [
        &b""[..],
        b"not json",
        b"[]",
        b"{}",
        br#"{"schema":1}"#,
        b"\xff\xfe\x00",
    ] {
        assert_eq!(
            Manifest::parse(bytes),
            Err(UpdateError::InvalidJson),
            "{bytes:?}"
        );
    }
    assert_eq!(
        edited_err(|v| v["channel"] = "nightly".into()),
        UpdateError::InvalidJson
    );
    assert_eq!(
        edited_err(|v| {
            v.as_object_mut().unwrap().remove("version");
        }),
        UpdateError::InvalidJson
    );
    assert_eq!(
        edited_err(|v| v["assets"][0]["size"] = "big".into()),
        UpdateError::InvalidJson
    );
}

// ---------------------------------------------------------------- AC3

fn eval(m: &Manifest, installed: &str, ch: Channel) -> Result<Decision, UpdateError> {
    evaluate(m, installed, ch)
}

#[test]
fn ac3_valid_manifest_is_available_with_its_version_and_notes() {
    let m = valid();
    match eval(&m, "0.3.3", Channel::Stable) {
        Ok(Decision::Available(u)) => {
            assert_eq!(u.version, "0.4.0");
            assert_eq!(u.notes_url, m.notes_url);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn ac3_other_channel_is_channel_mismatch_even_when_not_newer() {
    let m = valid();
    assert_eq!(
        eval(&m, "0.3.3", Channel::Beta),
        Err(UpdateError::ChannelMismatch)
    );
    // The channel is checked before the version.
    assert_eq!(
        eval(&m, "0.4.0", Channel::Beta),
        Err(UpdateError::ChannelMismatch)
    );
    assert_eq!(
        eval(&m, "0.9.0", Channel::Beta),
        Err(UpdateError::ChannelMismatch)
    );
    let beta = edited(|v| {
        v["channel"] = "beta".into();
        v["version"] = "0.4.0-beta.2".into();
    });
    assert_eq!(
        eval(&beta, "0.4.0-beta.1", Channel::Stable),
        Err(UpdateError::ChannelMismatch)
    );
}

#[test]
fn ac3_same_or_lower_version_is_up_to_date() {
    let m = valid();
    assert_eq!(eval(&m, "0.4.0", Channel::Stable), Ok(Decision::UpToDate));
    assert_eq!(eval(&m, "0.4.1", Channel::Stable), Ok(Decision::UpToDate));
    assert_eq!(eval(&m, "1.0.0", Channel::Stable), Ok(Decision::UpToDate));
    // Version is checked before the urls: a stale manifest is just up to date.
    let stale = edited(|v| v["assets"][0]["url"] = "http://example.com/x.msi".into());
    assert_eq!(
        eval(&stale, "0.4.0", Channel::Stable),
        Ok(Decision::UpToDate)
    );
}

#[test]
fn ac3_beta_channel_offers_the_next_beta_and_the_final_release() {
    let beta2 = edited(|v| {
        v["channel"] = "beta".into();
        v["version"] = "0.4.0-beta.2".into();
    });
    assert!(matches!(
        eval(&beta2, "0.4.0-beta.1", Channel::Beta),
        Ok(Decision::Available(u)) if u.version == "0.4.0-beta.2"
    ));
    assert_eq!(
        eval(&beta2, "0.4.0-beta.2", Channel::Beta),
        Ok(Decision::UpToDate)
    );
    assert_eq!(eval(&beta2, "0.4.0", Channel::Beta), Ok(Decision::UpToDate));
}

#[test]
fn ac3_bad_installed_version_is_bad_version() {
    assert_eq!(
        eval(&valid(), "", Channel::Stable),
        Err(UpdateError::BadVersion)
    );
    assert_eq!(
        eval(&valid(), "v0.3.3", Channel::Stable),
        Err(UpdateError::BadVersion)
    );
}

#[test]
fn ac3_bad_manifest_version_is_bad_version() {
    let m = edited(|v| v["version"] = "v0.4.0".into());
    assert_eq!(
        eval(&m, "0.3.3", Channel::Stable),
        Err(UpdateError::BadVersion)
    );
}

#[test]
fn ac3_every_asset_url_must_start_with_the_release_prefix() {
    let bad_urls = [
        "http://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/x.msi",
        "https://github.com.evil.example/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/x.msi",
        "https://github.com/M7MMAD-OMAR/sbarhive/releases/download",
        "https://github.com/M7MMAD-OMAR/sbarhive-evil/releases/download/v0.4.0/x.msi",
        "https://GitHub.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/x.msi",
        "https://github.com/m7mmad-omar/sbarhive/releases/download/v0.4.0/x.msi",
        " https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/x.msi",
        "https://evil.example/https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/x.msi",
        "",
    ];
    for bad in bad_urls {
        for idx in 0..3 {
            let m = edited(|v| v["assets"][idx]["url"] = bad.into());
            assert_eq!(
                eval(&m, "0.3.3", Channel::Stable),
                Err(UpdateError::BadUrl),
                "asset {idx}: {bad:?}"
            );
        }
    }
    // Exactly the prefix plus a name is accepted.
    let ok = edited(|v| v["assets"][1]["url"] = format!("{URL_PREFIX}x").into());
    assert!(matches!(
        eval(&ok, "0.3.3", Channel::Stable),
        Ok(Decision::Available(_))
    ));
}

// ---------------------------------------------------------------- AC4

fn plat(os: &str, arch: &str, pkg: &str) -> Platform {
    Platform {
        os: os.into(),
        arch: arch.into(),
        pkg: pkg.into(),
    }
}

#[test]
fn ac4_three_platforms_return_the_expected_asset() {
    let m = valid();
    assert_eq!(
        select_asset(&m, plat("windows", "x86_64", "msi"))
            .unwrap()
            .name,
        "SbarHive-0.4.0-x86_64.msi"
    );
    assert_eq!(
        select_asset(&m, plat("linux", "x86_64", "rpm"))
            .unwrap()
            .name,
        "sbarhive-0.4.0-1.x86_64.rpm"
    );
    assert_eq!(
        select_asset(&m, plat("linux", "x86_64", "deb"))
            .unwrap()
            .name,
        "sbarhive_0.4.0_amd64.deb"
    );
}

#[test]
fn ac4_aarch64_and_mismatched_triples_give_none() {
    let m = valid();
    assert!(select_asset(&m, plat("linux", "aarch64", "rpm")).is_none());
    assert!(select_asset(&m, plat("linux", "aarch64", "deb")).is_none());
    assert!(select_asset(&m, plat("windows", "aarch64", "msi")).is_none());
    assert!(select_asset(&m, plat("windows", "x86_64", "rpm")).is_none());
    assert!(select_asset(&m, plat("linux", "x86_64", "msi")).is_none());
    assert!(select_asset(&m, plat("macos", "x86_64", "deb")).is_none());
    assert!(select_asset(&m, plat("linux", "x86_64", "apk")).is_none());
}

#[test]
fn ac4_missing_kind_gives_none_and_the_right_arch_wins() {
    let no_deb = edited(|v| {
        v["assets"].as_array_mut().unwrap().pop();
    });
    assert!(select_asset(&no_deb, plat("linux", "x86_64", "deb")).is_none());
    assert!(select_asset(&no_deb, plat("linux", "x86_64", "rpm")).is_some());

    // An aarch64 rpm listed first must not be chosen for x86_64, and the reverse.
    let two = edited(|v| {
        let arr = v["assets"].as_array_mut().unwrap();
        let mut arm = arr[1].clone();
        arm["arch"] = "aarch64".into();
        arm["name"] = "sbarhive-0.4.0-1.aarch64.rpm".into();
        arr.insert(0, arm);
    });
    assert_eq!(
        select_asset(&two, plat("linux", "x86_64", "rpm"))
            .unwrap()
            .name,
        "sbarhive-0.4.0-1.x86_64.rpm"
    );
    assert_eq!(
        select_asset(&two, plat("linux", "aarch64", "rpm"))
            .unwrap()
            .name,
        "sbarhive-0.4.0-1.aarch64.rpm"
    );
    // The os is part of the key too: a windows rpm entry is not a linux one.
    let win_first = edited(|v| {
        let arr = v["assets"].as_array_mut().unwrap();
        let mut other = arr[1].clone();
        other["os"] = "windows".into();
        other["name"] = "wrong.rpm".into();
        arr.insert(0, other);
    });
    assert_eq!(
        select_asset(&win_first, plat("linux", "x86_64", "rpm"))
            .unwrap()
            .name,
        "sbarhive-0.4.0-1.x86_64.rpm"
    );
}

// ---------------------------------------------------------------- AC5

fn flip(bytes: &[u8], at: usize) -> Vec<u8> {
    let mut b = bytes.to_vec();
    b[at] ^= 0x01;
    b
}

#[test]
fn ac5_good_signature_with_key_a_is_ok() {
    let key_a = fx_text("key-a.pub");
    let sig = fx_text("manifest-stable-0.4.0.json.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    assert_eq!(verify_manifest(&bytes, &sig, &[&key_a]), Ok(()));
}

#[test]
fn ac5_bare_base64_key_line_is_accepted_too() {
    let key_a = fx_text("key-a.pub");
    let bare = key_a.lines().nth(1).unwrap();
    assert!(bare.starts_with("RW"));
    let sig = fx_text("manifest-stable-0.4.0.json.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    assert_eq!(verify_manifest(&bytes, &sig, &[bare]), Ok(()));
}

#[test]
fn ac5_one_flipped_byte_is_a_bad_signature() {
    let key_a = fx_text("key-a.pub");
    let sig = fx_text("manifest-stable-0.4.0.json.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    for at in [0, 10, bytes.len() / 2, bytes.len() - 1] {
        assert_eq!(
            verify_manifest(&flip(&bytes, at), &sig, &[&key_a]),
            Err(UpdateError::BadSignature),
            "byte {at}"
        );
    }
    let mut longer = bytes.clone();
    longer.push(b'\n');
    assert_eq!(
        verify_manifest(&longer, &sig, &[&key_a]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac5_signature_of_key_b_is_rejected_when_only_key_a_is_trusted() {
    let key_a = fx_text("key-a.pub");
    let sig_b = fx_text("manifest-stable-0.4.0.json.b.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    assert_eq!(
        verify_manifest(&bytes, &sig_b, &[&key_a]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac5_empty_key_list_is_a_bad_signature() {
    let sig = fx_text("manifest-stable-0.4.0.json.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    assert_eq!(
        verify_manifest(&bytes, &sig, &[]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac5_either_key_of_the_list_is_enough_in_any_order() {
    let key_a = fx_text("key-a.pub");
    let key_b = fx_text("key-b.pub");
    let sig_a = fx_text("manifest-stable-0.4.0.json.minisig");
    let sig_b = fx_text("manifest-stable-0.4.0.json.b.minisig");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    assert_eq!(verify_manifest(&bytes, &sig_a, &[&key_b, &key_a]), Ok(()));
    assert_eq!(verify_manifest(&bytes, &sig_a, &[&key_a, &key_b]), Ok(()));
    assert_eq!(verify_manifest(&bytes, &sig_b, &[&key_a, &key_b]), Ok(()));
    assert_eq!(verify_manifest(&bytes, &sig_b, &[&key_b]), Ok(()));
    assert_eq!(
        verify_manifest(&bytes, &sig_a, &[&key_b]),
        Err(UpdateError::BadSignature)
    );
    // A tampered file fails with both keys trusted.
    assert_eq!(
        verify_manifest(&flip(&bytes, 3), &sig_a, &[&key_a, &key_b]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac5_unparseable_signature_text_is_a_bad_signature() {
    let key_a = fx_text("key-a.pub");
    let bytes = fx_bytes("manifest-stable-0.4.0.json");
    for sig in ["", "garbage", "untrusted comment: x\nnot base64\n"] {
        assert_eq!(
            verify_manifest(&bytes, sig, &[&key_a]),
            Err(UpdateError::BadSignature),
            "{sig:?}"
        );
    }
}

// ---------------------------------------------------------------- AC6

#[test]
fn ac6_update_keys_is_not_empty_and_every_entry_parses() {
    assert!(!UPDATE_KEYS.is_empty());
    for (i, k) in UPDATE_KEYS.iter().enumerate() {
        assert_eq!(parse_key(k), Ok(()), "UPDATE_KEYS[{i}]");
    }
}

#[test]
fn ac6_the_first_update_key_is_the_committed_public_key() {
    let committed =
        std::fs::read_to_string(root().join("packaging/keys/sbarhive-update.pub")).unwrap();
    assert_eq!(UPDATE_KEYS[0].trim(), committed.trim());
    assert_eq!(parse_key(&committed), Ok(()));
}

#[test]
fn ac6_placeholders_and_junk_do_not_parse_as_keys() {
    for bad in [
        "",
        "REPLACE_ME",
        "untrusted comment: placeholder\nREPLACE_WITH_REAL_KEY",
        "RWRSFM4u",
        "not a key at all",
    ] {
        assert_eq!(parse_key(bad), Err(UpdateError::BadKey), "{bad:?}");
    }
    // The fixture keys are real keys.
    assert_eq!(parse_key(&fx_text("key-a.pub")), Ok(()));
    assert_eq!(parse_key(&fx_text("key-b.pub")), Ok(()));
}

#[test]
fn ac6_engines_doc_records_minisign_verify() {
    let doc = std::fs::read_to_string(root().join("docs/engines.md")).unwrap();
    assert!(
        doc.lines()
            .filter(|l| l.contains("minisign-verify"))
            .count()
            >= 1,
        "docs/engines.md must record minisign-verify (licence and dependencies, via cargo info)"
    );
}

// ---------------------------------------------------------------- AC7

fn failed(n: u32, last_failure: u64) -> State {
    State {
        last_check: Some(1_000),
        last_failure: Some(last_failure),
        failures: n,
    }
}

#[test]
fn ac7_never_checked_is_due() {
    assert!(due(&State::default(), 0, DAY));
    assert!(due(&State::default(), 5_000_000, DAY));
}

#[test]
fn ac7_interval_boundaries_after_a_success() {
    let s = State {
        last_check: Some(1_000_000),
        ..State::default()
    };
    assert!(!due(&s, 1_000_000, DAY));
    assert!(!due(&s, 1_000_000 + DAY - 60, DAY)); // 23 h 59 min
    assert!(!due(&s, 1_000_000 + DAY - 1, DAY));
    assert!(due(&s, 1_000_000 + DAY, DAY));
    assert!(due(&s, 1_000_000 + DAY + 1, DAY));
    assert!(due(&s, 1_000_000 + 10 * DAY, DAY));
}

#[test]
fn ac7_the_interval_argument_is_used() {
    let s = State {
        last_check: Some(1_000_000),
        ..State::default()
    };
    assert!(!due(&s, 1_000_000 + HOUR - 1, HOUR));
    assert!(due(&s, 1_000_000 + HOUR, HOUR));
    assert!(!due(&s, 1_000_000 + 2 * DAY - 1, 2 * DAY));
    assert!(due(&s, 1_000_000 + 2 * DAY, 2 * DAY));
}

#[test]
fn ac7_clock_going_backwards_is_due() {
    let s = State {
        last_check: Some(1_000_000),
        ..State::default()
    };
    assert!(due(&s, 999_999, DAY));
    assert!(due(&s, 0, DAY));
    let f = failed(2, 1_000_000);
    assert!(due(&f, 999_999, DAY));
}

#[test]
fn ac7_backoff_waits_one_two_four_eight_sixteen_hours_then_24() {
    let t = 5_000_000;
    let waits = [(1, 1), (2, 2), (3, 4), (4, 8), (5, 16), (6, 24)];
    for (n, hours) in waits {
        let s = failed(n, t);
        assert!(!due(&s, t, DAY), "n={n} at once");
        assert!(
            !due(&s, t + hours * HOUR - 1, DAY),
            "n={n} one second early"
        );
        assert!(due(&s, t + hours * HOUR, DAY), "n={n} on time");
        assert!(due(&s, t + hours * HOUR + 1, DAY), "n={n} late");
    }
}

#[test]
fn ac7_backoff_is_capped_at_24_hours_and_never_overflows() {
    let t = 5_000_000;
    for n in [6, 7, 8, 31, 32, 33, 63, 64, 65, 100, u32::MAX] {
        let s = failed(n, t);
        assert!(!due(&s, t + DAY - 1, DAY), "n={n}");
        assert!(due(&s, t + DAY, DAY), "n={n}");
    }
}

#[test]
fn ac7_backoff_counts_from_last_failure_not_last_check() {
    // The last success was long ago; the failure was just now: wait for the backoff.
    let s = State {
        last_check: Some(1_000),
        last_failure: Some(9_000_000),
        failures: 1,
    };
    assert!(!due(&s, 9_000_000 + HOUR - 1, DAY));
    assert!(due(&s, 9_000_000 + HOUR, DAY));
    // Never succeeded, one failure: also a wait from the failure.
    let s = State {
        last_check: None,
        last_failure: Some(9_000_000),
        failures: 1,
    };
    assert!(!due(&s, 9_000_000 + HOUR - 1, DAY));
    assert!(due(&s, 9_000_000 + HOUR, DAY));
}

#[test]
fn ac7_failures_without_a_failure_time_are_due() {
    let s = State {
        last_check: Some(1_000_000),
        last_failure: None,
        failures: 3,
    };
    assert!(due(&s, 1_000_001, DAY));
}

#[test]
fn ac7_zero_failures_ignore_a_stale_failure_time() {
    let s = State {
        last_check: Some(1_000_000),
        last_failure: Some(1_000_000 + 100),
        failures: 0,
    };
    assert!(!due(&s, 1_000_000 + DAY - 1, DAY));
    assert!(due(&s, 1_000_000 + DAY, DAY));
}

#[test]
fn ac7_state_round_trips_through_json() {
    for s in [
        State::default(),
        State {
            last_check: Some(1_792_829_690),
            last_failure: Some(1_792_829_999),
            failures: 4,
        },
        State {
            last_check: None,
            last_failure: Some(7),
            failures: u32::MAX,
        },
        State {
            last_check: Some(u64::MAX),
            last_failure: None,
            failures: 0,
        },
    ] {
        assert_eq!(State::from_json(&s.to_json()), s);
    }
}

#[test]
fn ac7_state_json_uses_the_documented_key_names() {
    let s = State::from_json(r#"{"last_check":5,"last_failure":7,"failures":2}"#);
    assert_eq!(
        s,
        State {
            last_check: Some(5),
            last_failure: Some(7),
            failures: 2
        }
    );
    // Missing fields take their defaults; unknown keys are ignored.
    let s = State::from_json(r#"{"last_check":5,"extra":true}"#);
    assert_eq!(
        s,
        State {
            last_check: Some(5),
            last_failure: None,
            failures: 0
        }
    );
    let json: serde_json::Value = serde_json::from_str(
        &State {
            last_check: Some(5),
            last_failure: Some(7),
            failures: 2,
        }
        .to_json(),
    )
    .unwrap();
    assert_eq!(json["last_check"], 5);
    assert_eq!(json["last_failure"], 7);
    assert_eq!(json["failures"], 2);
}

#[test]
fn ac7_corrupt_or_empty_state_text_gives_the_default() {
    for text in [
        "",
        "   ",
        "garbage",
        "{",
        "[]",
        "null",
        "42",
        r#"{"last_check":"yesterday"}"#,
        r#"{"failures":-1}"#,
        "\u{0}\u{1}",
    ] {
        assert_eq!(State::from_json(text), State::default(), "{text:?}");
    }
}

#[test]
fn ac7_manual_check_bypasses_update_check_false() {
    assert!(allowed(true, false));
    assert!(allowed(true, true));
    assert!(allowed(false, true));
    assert!(!allowed(false, false));
}

// ---------------------------------------------------------------- AC8

fn load(text: &str) -> (Config, Option<String>) {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, text).unwrap();
    Config::load_from(&p)
}

#[test]
fn ac8_defaults() {
    let c = Config::default();
    assert!(c.update_check);
    assert_eq!(c.update_channel, Channel::Stable);
    assert_eq!(c.update_mode, UpdateMode::Ask);
    let (c, w) = load("hold_ms = 500\n");
    assert_eq!(w, None);
    assert!(c.update_check);
    assert_eq!(c.update_channel, Channel::Stable);
    assert_eq!(c.update_mode, UpdateMode::Ask);
    assert_eq!(u64::from(c.hold_ms), 500);
}

#[test]
fn ac8_all_three_keys_are_read_with_the_other_keys() {
    let (c, w) = load(
        "hold_ms = 500\nupdate_check = false\nupdate_channel = \"beta\"\nupdate_mode = \"auto\"\n",
    );
    assert_eq!(w, None);
    assert!(!c.update_check);
    assert_eq!(c.update_channel, Channel::Beta);
    assert_eq!(c.update_mode, UpdateMode::Auto);
    assert_eq!(u64::from(c.hold_ms), 500);
    let (c, w) = load("update_channel = \"stable\"\nupdate_mode = \"ask\"\nupdate_check = true\n");
    assert_eq!(w, None);
    assert!(c.update_check);
    assert_eq!(c.update_channel, Channel::Stable);
    assert_eq!(c.update_mode, UpdateMode::Ask);
}

#[test]
fn ac8_each_key_alone_leaves_the_others_at_default() {
    let (c, _) = load("update_check = false\n");
    assert!(!c.update_check);
    assert_eq!(c.update_channel, Channel::Stable);
    assert_eq!(c.update_mode, UpdateMode::Ask);
    let (c, _) = load("update_channel = \"beta\"\n");
    assert!(c.update_check);
    assert_eq!(c.update_channel, Channel::Beta);
    assert_eq!(c.update_mode, UpdateMode::Ask);
    let (c, _) = load("update_mode = \"auto\"\n");
    assert!(c.update_check);
    assert_eq!(c.update_channel, Channel::Stable);
    assert_eq!(c.update_mode, UpdateMode::Auto);
}

#[test]
fn ac8_nightly_channel_gives_the_defaults_and_a_warning_naming_the_file() {
    let (c, w) = load("hold_ms = 500\nupdate_channel = \"nightly\"\n");
    assert_eq!(c, Config::default());
    assert!(w.expect("a warning").contains("config.toml"));
}

#[test]
fn ac8_other_invalid_values_also_fall_back_with_a_warning() {
    for text in [
        "update_mode = \"manual\"\n",
        "update_check = \"yes\"\n",
        "update_channel = 3\n",
    ] {
        let (c, w) = load(text);
        assert_eq!(c, Config::default(), "{text:?}");
        assert!(w.is_some(), "{text:?}");
    }
}

#[test]
fn ac8_keys_are_saved_with_underscores_and_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let c = Config {
        update_check: false,
        update_channel: Channel::Beta,
        update_mode: UpdateMode::Auto,
        ..Config::default()
    };
    c.save_to(&p).unwrap();
    let text = std::fs::read_to_string(&p).unwrap();
    let has = |line: &str| text.lines().any(|l| l.trim() == line);
    assert!(has("update_check = false"), "{text}");
    assert!(has("update_channel = \"beta\""), "{text}");
    assert!(has("update_mode = \"auto\""), "{text}");
    assert!(!text.contains("update-"), "{text}");
    assert_eq!(Config::load_from(&p), (c, None));
}
