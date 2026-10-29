//! Embeds every `actions/*.toml` file of the repository, in file name order,
//! so adding an action file needs no code change.
use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../actions");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    files.retain(|p| p.extension().is_some_and(|e| e == "toml"));
    files.sort();
    let mut out = String::from("&[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let name = f.file_name().unwrap().to_string_lossy();
        let path = f.canonicalize().unwrap();
        writeln!(
            out,
            "    ({name:?}, include_str!({:?})),",
            path.display().to_string()
        )
        .unwrap();
    }
    out.push(']');
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out_dir.join("builtin_actions.rs"), out).unwrap();

    // The notes of the installed version (S-099a A5); empty when the file is
    // missing, which the release gate test rejects.
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let notes = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../../docs/whats-new")
        .join(format!("{version}.json"));
    println!("cargo:rerun-if-changed={}", notes.display());
    let body = match notes.canonicalize() {
        Ok(p) => format!("include_str!({:?})", p.display().to_string()),
        Err(_) => String::from("\"\""),
    };
    std::fs::write(out_dir.join("whats_new.rs"), body).unwrap();
}
