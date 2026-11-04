//! S-134 AC1: Config::from_table reads [mcp] audit_path.
//! Contract pinned here (tester): audit_path is a string key; an absolute,
//! ~ or relative string is kept as written in Config.audit_path; absent,
//! empty and non-string values leave it None.

use kawaro_mcp::Config;

fn config(text: &str) -> Config {
    let table: toml::Table = toml::from_str(text).unwrap();
    Config::from_table(&table)
}

#[test]
fn ac1_an_absolute_audit_path_is_read() {
    let c = config("[mcp]\naudit_path = \"/var/log/kawaro/audit.jsonl\"\n");
    assert_eq!(
        c.audit_path,
        Some(std::path::PathBuf::from("/var/log/kawaro/audit.jsonl"))
    );
}

#[test]
fn ac1_a_tilde_and_a_relative_audit_path_are_kept_as_written() {
    let c = config("[mcp]\naudit_path = \"~/logs/audit.jsonl\"\n");
    assert_eq!(
        c.audit_path,
        Some(std::path::PathBuf::from("~/logs/audit.jsonl")),
        "from_table keeps the string; resolved_audit_path expands it"
    );
    let c = config("[mcp]\naudit_path = \"logs/audit.jsonl\"\n");
    assert_eq!(
        c.audit_path,
        Some(std::path::PathBuf::from("logs/audit.jsonl"))
    );
}

#[test]
fn ac1_absent_empty_and_wrong_typed_paths_are_none() {
    assert_eq!(config("[mcp]\nenabled = true\n").audit_path, None);
    assert_eq!(config("[mcp]\naudit_path = \"\"\n").audit_path, None);
    assert_eq!(config("[mcp]\naudit_path = 7\n").audit_path, None);
    assert_eq!(
        config("[mcp]\naudit_path = [\"logs/audit.jsonl\"]\n").audit_path,
        None
    );
}
