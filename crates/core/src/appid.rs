//! The one application id (S-049, ADR 0005 section 5): the Windows pipe name
//! starts with it, and the Windows toast carries it as its
//! `System.AppUserModel.ID`, so the notification shows as Kawaro once the
//! id is registered (S-050, S-051).

/// The reverse DNS id of Kawaro.
pub const APP_ID: &str = "io.github.m7mmad_omar.Kawaro";
