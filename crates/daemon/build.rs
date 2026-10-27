//! Embeds the Windows icon and version resource in kawarod.exe (S-085 AC1).
//!
//! Only a Windows target gets a resource; every other build returns at once.
//! winresource writes `#pragma code_page(65001)` into the rc file, so windres
//! keeps the Arabic half of the name (D5). tests/windows/test_s085.sh reads
//! the strings and the icon sizes back from the built exe.

use std::path::Path;

/// The fixed brand string, the same as `kawaro_core::identity::DISPLAY_NAME`.
const DISPLAY_NAME: &str = "Kawaro | كوارو";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let ico = Path::new(&manifest).join("../../design/brand/kawaro.ico");
    println!("cargo:rerun-if-changed={}", ico.display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets CARGO_PKG_VERSION");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("the icon path is UTF-8"))
        .set("ProductName", DISPLAY_NAME)
        .set("FileDescription", DISPLAY_NAME)
        .set("FileVersion", &version)
        .set("ProductVersion", &version)
        .set("OriginalFilename", "kawarod.exe");
    if let Err(e) = res.compile() {
        panic!("cannot embed the Windows resource: {e}");
    }
}
