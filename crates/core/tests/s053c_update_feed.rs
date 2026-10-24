//! S-053c (backlog/S-053c-update-release-signing.md, docs/adr/0006-auto-update.md):
//! the release scripts feed the S-053a core. Written by the tester from the
//! acceptance criteria.
//!
//! Contract (the implementer follows it; the shell tests in `tests/release/`
//! hold the full script contract):
//! - `scripts/make-update-manifest.sh --dist DIR CHANNEL VERSION ASSETS_JSON OUT_JSON`
//! - `UPDATE_SIGNING_PASSWORD=... scripts/sign-update-manifest.sh KEY_FILE MANIFEST`
//!   writes `MANIFEST.minisig`.
//! - The manifest it makes parses with `Manifest::parse`, passes `evaluate`
//!   for its channel, and its signature verifies with `verify_manifest`
//!   against the throwaway public key only.
//!
//! Needs `minisign` and `jq` on PATH (CI installs minisign). A real key is
//! never involved: every key is made here with `minisign -G -W`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sbarhive_core::update::manifest::Manifest;
use sbarhive_core::update::policy::{Decision, Platform, evaluate, select_asset};
use sbarhive_core::update::verify::verify_manifest;
use sbarhive_core::update::{Channel, UpdateError};

const VERSION: &str = "0.4.0-beta.1";
const BASE: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0-beta.1/";
const RPM: &str = "sbarhive-0.4.0~beta.1-1.x86_64.rpm";
const DEB: &str = "sbarhive_0.4.0~beta.1_amd64.deb";
const MSI: &str = "SbarHive-0.4.0-beta.1-x64.msi";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(cmd: &mut Command) -> std::process::Output {
    cmd.output().expect("the program must start")
}

struct Feed {
    _dir: tempfile::TempDir,
    manifest: PathBuf,
    sig: PathBuf,
    pubkey: String,
}

fn build_feed(channel: &str, version: &str) -> Feed {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let dist = p.join("dist");
    fs::create_dir(&dist).unwrap();
    let mut assets = Vec::new();
    for (name, body) in [(RPM, "rpm"), (DEB, "deb body"), (MSI, "msi body!")] {
        fs::write(dist.join(name), body).unwrap();
        assets.push(serde_json::json!({
            "name": name,
            "url": format!("{BASE}{name}"),
            "size": body.len(),
        }));
    }
    let assets_json = p.join("assets.json");
    fs::write(
        &assets_json,
        serde_json::json!({ "assets": assets }).to_string(),
    )
    .unwrap();

    let (key, pubf) = (p.join("t.key"), p.join("t.pub"));
    let g = run(Command::new("minisign")
        .args(["-G", "-W", "-p"])
        .arg(&pubf)
        .arg("-s")
        .arg(&key));
    assert!(g.status.success(), "minisign -G -W failed: {g:?}");

    let manifest = p.join(format!("{channel}.json"));
    let m = run(Command::new(root().join("scripts/make-update-manifest.sh"))
        .arg("--dist")
        .arg(&dist)
        .args([channel, version])
        .arg(&assets_json)
        .arg(&manifest));
    assert!(m.status.success(), "make-update-manifest.sh failed: {m:?}");

    let s = run(Command::new(root().join("scripts/sign-update-manifest.sh"))
        .env("UPDATE_SIGNING_PASSWORD", "unused-by-an-unencrypted-key")
        .arg(&key)
        .arg(&manifest));
    assert!(s.status.success(), "sign-update-manifest.sh failed: {s:?}");

    let sig = PathBuf::from(format!("{}.minisig", manifest.display()));
    Feed {
        manifest,
        sig,
        pubkey: fs::read_to_string(&pubf).unwrap(),
        _dir: dir,
    }
}

#[test]
fn ac2_generated_manifest_parses_and_passes_evaluate_for_its_channel() {
    let f = build_feed("beta", VERSION);
    let bytes = fs::read(&f.manifest).unwrap();
    let m = Manifest::parse(&bytes).expect("Manifest::parse accepts the generated file");
    assert_eq!(m.schema, 1);
    assert_eq!(m.channel, Channel::Beta);
    assert_eq!(m.version, VERSION);
    assert_eq!(m.assets.len(), 3);
    assert_eq!(m.assets[0].name, RPM, "the ~ survives");
    match evaluate(&m, "0.3.3", Channel::Beta).unwrap() {
        Decision::Available(u) => {
            assert_eq!(u.version, VERSION);
            assert_eq!(u.notes_url, m.notes_url);
        }
        other => panic!("expected Available, got {other:?}"),
    }
    assert_eq!(
        evaluate(&m, "0.3.3", Channel::Stable),
        Err(UpdateError::ChannelMismatch)
    );
    assert_eq!(
        evaluate(&m, VERSION, Channel::Beta),
        Ok(Decision::UpToDate),
        "the same version is not an update"
    );
}

#[test]
fn ac2_each_platform_finds_its_package() {
    let f = build_feed("beta", VERSION);
    let m = Manifest::parse(&fs::read(&f.manifest).unwrap()).unwrap();
    let pick = |os: &str, pkg: &str| {
        select_asset(
            &m,
            Platform {
                os: os.into(),
                arch: "x86_64".into(),
                pkg: pkg.into(),
            },
        )
        .map(|a| a.name.clone())
    };
    assert_eq!(pick("linux", "rpm").as_deref(), Some(RPM));
    assert_eq!(pick("linux", "deb").as_deref(), Some(DEB));
    assert_eq!(pick("windows", "msi").as_deref(), Some(MSI));
    assert_eq!(pick("windows", "rpm"), None);
}

#[test]
fn ac3_signature_verifies_with_the_throwaway_key_only() {
    let f = build_feed("beta", VERSION);
    let bytes = fs::read(&f.manifest).unwrap();
    let sig = fs::read_to_string(&f.sig).expect("MANIFEST.minisig exists");
    assert_eq!(verify_manifest(&bytes, &sig, &[f.pubkey.as_str()]), Ok(()));
    // the committed release key must not verify a throwaway signature
    let real = include_str!("../../../packaging/keys/sbarhive-update.pub");
    assert_eq!(
        verify_manifest(&bytes, &sig, &[real]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac3_a_flipped_byte_is_a_bad_signature() {
    let f = build_feed("stable", "0.4.0");
    let mut bytes = fs::read(&f.manifest).unwrap();
    let sig = fs::read_to_string(&f.sig).unwrap();
    let last = bytes.len() / 2;
    bytes[last] ^= 0x01;
    assert_eq!(
        verify_manifest(&bytes, &sig, &[f.pubkey.as_str()]),
        Err(UpdateError::BadSignature)
    );
}

#[test]
fn ac1_sha256_and_size_in_the_manifest_match_the_files() {
    use sha2::{Digest, Sha256};
    let f = build_feed("beta", VERSION);
    let m = Manifest::parse(&fs::read(&f.manifest).unwrap()).unwrap();
    let dist = f.manifest.parent().unwrap().join("dist");
    for a in &m.assets {
        let body = fs::read(dist.join(&a.name)).unwrap();
        let want: String = Sha256::digest(&body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(a.sha256, want, "{}", a.name);
        assert_eq!(a.size, body.len() as u64, "{}", a.name);
        assert_eq!(a.url, format!("{BASE}{}", a.name));
    }
}
