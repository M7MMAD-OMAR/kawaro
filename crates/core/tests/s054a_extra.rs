//! S-054a: small helpers the implementer added beside the tester's contract
//! (the `--kind` text, the effective user id, the system program paths).

use kawaro_core::update::apply::{PkgKind, Programs, effective_uid};
use std::path::Path;

#[test]
fn kind_text_round_trips_and_rejects_the_rest() {
    for k in [PkgKind::Rpm, PkgKind::Deb] {
        assert_eq!(PkgKind::parse(k.as_str()), Some(k));
    }
    assert_eq!(PkgKind::parse("RPM"), None);
    assert_eq!(PkgKind::parse(""), None);
    assert_eq!(PkgKind::parse("msi"), None);
}

#[test]
fn effective_uid_is_the_second_number_of_the_uid_line() {
    let status = "Name:\tx\nUid:\t1000\t0\t1000\t1000\nGid:\t5\t6\t7\t8\n";
    assert_eq!(effective_uid(status), Some(0));
    assert_eq!(effective_uid("Uid:\t0\t1000\t0\t0\n"), Some(1000));
    assert_eq!(effective_uid("Gid:\t0\t0\t0\t0\n"), None);
    assert_eq!(effective_uid("Uid:\t0\n"), None);
    assert_eq!(effective_uid("Uid:\t0\tx\n"), None);
    assert_eq!(effective_uid(""), None);
}

#[test]
fn system_programs_are_absolute() {
    let p = Programs::system();
    assert_eq!(p.dnf, Path::new("/usr/bin/dnf"));
    assert_eq!(p.apt_get, Path::new("/usr/bin/apt-get"));
}
