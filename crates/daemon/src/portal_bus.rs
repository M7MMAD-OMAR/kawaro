//! The D-Bus side of the GlobalShortcuts portal client (S-014e1): on a
//! private thread and its own session bus connection, `Registry.Register`
//! first, then `CreateSession` and one `BindShortcuts`, then the portal's
//! `Activated` and `Deactivated` into the long press gate, the one that
//! `sbarhive drop --hold` and `--release` feed. Every decision is in
//! `portal.rs`; this file runs on a live bus only and is covered by
//! tests/drop/portal.sh.

use crate::portal::{self, Reason};
use crate::state::{Shared, lock};
use crate::ui;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::{ResponseError, Session};
use ashpd::zbus;
use ashpd::zbus::zvariant::{self, OwnedObjectPath, OwnedValue};
use futures_util::StreamExt;
use sbarhive_core::config::Config;
use sbarhive_core::i18n::Lang;
use std::collections::HashMap;
use std::time::Instant;

const DESKTOP: &str = "org.freedesktop.portal.Desktop";

/// Starts the client, or prints why it does not start. `enabled` is the
/// `portal_shortcut` setting.
pub fn start(enabled: bool, lang: Lang, state: Shared) {
    if !enabled {
        return sbarhive_ui::stderr_line(&portal::unavailable_line(Reason::Off));
    }
    let desktops = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if portal::listed(&desktops, portal::NO_RELEASE) {
        return sbarhive_ui::stderr_line(&portal::unavailable_line(Reason::NoRelease));
    }
    let spawned = std::thread::Builder::new()
        .name("sbarhive-portal".into())
        .spawn(move || {
            // No portal call after the fallback line, Session.Close
            // included: the portal drops the session with the connection.
            if let Some(r) = zbus::block_on(run(lang, &state)) {
                if portal::remembers(r) {
                    remember(&state);
                }
                sbarhive_ui::stderr_line(&portal::unavailable_line(r));
            }
        });
    if let Err(e) = spawned {
        eprintln!("sbarhived: portal thread: {e}");
    }
}

/// Whether the portal runs or can be activated.
async fn present(conn: &zbus::Connection) -> bool {
    let Ok(bus) = zbus::fdo::DBusProxy::new(conn).await else {
        return false;
    };
    let Ok(name) = zbus::names::BusName::try_from(DESKTOP) else {
        return false;
    };
    if bus.name_has_owner(name).await.unwrap_or(false) {
        return true;
    }
    bus.list_activatable_names()
        .await
        .is_ok_and(|names| names.iter().any(|n| n.as_str() == DESKTOP))
}

/// Registers and binds, then feeds the signals to the gate until the
/// connection ends. Returns the reason of a fallback.
async fn run(lang: Lang, state: &Shared) -> Option<Reason> {
    let Ok(conn) = zbus::Connection::session().await else {
        return Some(Reason::NoPortal);
    };
    if !present(&conn).await {
        return Some(Reason::NoPortal);
    }
    let Ok(app) = ashpd::AppID::try_from(portal::APP_ID) else {
        return Some(Reason::Error);
    };
    if ashpd::register_host_app_with_connection(conn.clone(), app)
        .await
        .is_err()
    {
        sbarhive_ui::stderr_line(portal::REGISTRY_UNAVAILABLE);
    }
    let gs = match GlobalShortcuts::with_connection(conn).await {
        Ok(gs) => gs,
        Err(ashpd::Error::PortalNotFound(_)) => return portal::interface_reason(None),
        Err(_) => return Some(Reason::Error),
    };
    if let Some(r) = portal::interface_reason(Some(gs.version())) {
        return Some(r);
    }
    let Ok(session) = gs.create_session(Default::default()).await else {
        return Some(Reason::Error);
    };
    // Before the bind, so no signal after the bound line is missed. One
    // stream keeps Activated and Deactivated in their order.
    let Ok(mut signals) = gs.receive_all_signals().await else {
        return Some(Reason::Error);
    };
    let shortcut = NewShortcut::new(portal::SHORTCUT_ID, portal::description(lang))
        .preferred_trigger(portal::PREFERRED_TRIGGER);
    let reply = match gs
        .bind_shortcuts(&session, &[shortcut], None, Default::default())
        .await
    {
        Ok(request) => request.response(),
        Err(e) => Err(e),
    };
    let (code, bound): (u32, Vec<&str>) = match &reply {
        Ok(r) => (0, r.shortcuts().iter().map(|s| s.id()).collect()),
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => (1, Vec::new()),
        Err(_) => (2, Vec::new()),
    };
    if let Some(r) = portal::bind_reason(code, &bound) {
        return Some(r);
    }
    let trigger = reply
        .iter()
        .flat_map(|r| r.shortcuts())
        .find(|s| s.id() == portal::SHORTCUT_ID)
        .map(|s| s.trigger_description().to_string())
        .unwrap_or_default();
    sbarhive_ui::stderr_line(&portal::bound_line(&trigger));

    let Some(own) = session_path(&session) else {
        return Some(Reason::Error);
    };
    let debug = std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some();
    while let Some(msg) = signals.next().await {
        let header = msg.header();
        let Some(member) = header.member() else {
            continue;
        };
        // ShortcutsChanged has another body and is skipped here.
        let Ok((path, id, _, _)) =
            msg.body()
                .deserialize::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>()
        else {
            continue;
        };
        if let Some(s) = portal::signal(&own, member.as_str(), path.as_str(), &id) {
            if debug {
                sbarhive_ui::stderr_line(portal::debug_line(s));
            }
            ui::hold_signal(s, None, Instant::now(), state.clone());
        }
    }
    None
}

/// The session's object path. ashpd keeps it private but serializes the
/// session as that path (signature `o`), so it is read back from there.
fn session_path(session: &Session<GlobalShortcuts>) -> Option<String> {
    let ctxt = zvariant::serialized::Context::new_dbus(zvariant::LE, 0);
    let data = zvariant::to_bytes(ctxt, session).ok()?;
    let (path, _) = data.deserialize::<OwnedObjectPath>().ok()?;
    Some(path.as_str().to_string())
}

/// Manager answer 5: a denial writes `portal_shortcut = false`, keeping the
/// other keys, so the dialog does not return at every login. A broken file
/// is left alone rather than replaced by the defaults.
fn remember(state: &Shared) {
    let Some(path) = Config::path() else {
        return;
    };
    let (mut config, warning) = Config::load_from(&path);
    if warning.is_some() {
        return;
    }
    config.portal_shortcut = false;
    if let Err(e) = config.save_to(&path) {
        eprintln!("sbarhived: cannot save {}: {e}", path.display());
        return;
    }
    lock(state).config.portal_shortcut = false;
}
