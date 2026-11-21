//! S-151: the socket directory is checked before the socket is used.
//!
//! The directory must be a real directory, owned by this user, and writable
//! by neither the group nor the others, so no other user can take the
//! socket or read what is written to it.

#![cfg(unix)]

use kawaro_core::transport::{Bound, Endpoint, Listener, Stream};
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;

/// A temporary directory with the given mode.
fn dir_with_mode(mode: u32) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(mode)).unwrap();
    dir
}

#[test]
fn s151_a_socket_directory_that_others_can_write_is_refused() {
    let dir = dir_with_mode(0o777);
    let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
    let error = match Listener::bind(&endpoint) {
        Ok(_) => panic!("a world writable directory must be refused"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), ErrorKind::PermissionDenied, "{error}");
    assert!(error.to_string().contains("writable"), "{error}");
}

#[test]
fn s151_a_private_socket_directory_is_accepted() {
    let dir = dir_with_mode(0o700);
    let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
    let bound = match Listener::bind(&endpoint) {
        Ok(bound) => bound,
        Err(error) => panic!("a private directory must be accepted: {error}"),
    };
    assert!(matches!(bound, Bound::Listening(_)));
}

#[test]
fn s151_a_client_refuses_a_socket_in_a_writable_directory() {
    let dir = dir_with_mode(0o700);
    let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
    let bound = match Listener::bind(&endpoint) {
        Ok(bound) => bound,
        Err(error) => panic!("a private directory must be accepted: {error}"),
    };
    assert!(matches!(bound, Bound::Listening(_)));
    // The directory becomes writable by the others: the client refuses it
    // too, before it connects.
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
    let error = match Stream::connect(&endpoint) {
        Ok(_) => panic!("a world writable directory must be refused"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), ErrorKind::PermissionDenied, "{error}");
}

#[test]
fn s151_a_socket_directory_that_is_a_symlink_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let endpoint = Endpoint::from_path(link.join("kawaro.sock"));
    let error = match Listener::bind(&endpoint) {
        Ok(_) => panic!("a symlinked directory must be refused"),
        Err(error) => error,
    };
    assert_eq!(error.kind(), ErrorKind::InvalidInput, "{error}");
}
