//! The D-Bus side of the GlobalShortcuts portal client (S-014e1): on a
//! private thread and its own session bus connection, `Registry.Register`
//! first, then `CreateSession` and one `BindShortcuts`, then the portal's
//! `Activated` and `Deactivated` into the long press gate, the one that
//! `kawaro drop --hold` and `--release` feed. The `portal_shortcut`
//! switch applies live (S-014f2): a reload starts a client or stops the
//! running one, at most one at a time. Every decision is in `portal.rs`;
//! this file runs on a live bus only and is covered by tests/drop/portal.sh.

use crate::portal::{self, Reason, Reload};
use crate::state::{Shared, lock};
use crate::ui;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::{ResponseError, Session};
use ashpd::zbus;
use ashpd::zbus::zvariant::{self, OwnedObjectPath, OwnedValue};
use futures_util::StreamExt;
use futures_util::future::{AbortHandle, AbortRegistration, Abortable};
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::Instant;

const DESKTOP: &str = "org.freedesktop.portal.Desktop";

/// The latest client thread, running or ended. `stop` ends it wherever it
/// waits.
struct Client {
    stop: AbortHandle,
    thread: JoinHandle<()>,
}

static CLIENT: Mutex<Option<Client>> = Mutex::new(None);

fn client() -> MutexGuard<'static, Option<Client>> {
    lock(&CLIENT)
}

/// Follows a change of `portal_shortcut` in a reload (the 2 s poll or a
/// settings Save, both through `ui::reload`).
pub fn follow(before: bool, after: bool, lang: Lang, state: &Shared) {
    let running = client().as_ref().is_some_and(|c| !c.thread.is_finished());
    match portal::reload(before, after, running) {
        Reload::Start => start(true, lang, state.clone()),
        Reload::Stop => {
            if let Some(c) = client().as_ref() {
                c.stop.abort();
            }
        }
        Reload::Nothing => {}
    }
}

/// Starts the client, or prints why it does not start. `enabled` is the
/// `portal_shortcut` setting.
pub fn start(enabled: bool, lang: Lang, state: Shared) {
    if !enabled {
        return kawaro_ui::stderr_line(&portal::unavailable_line(Reason::Off));
    }
    let desktops = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    if portal::listed(&desktops, portal::NO_RELEASE) {
        return kawaro_ui::stderr_line(&portal::unavailable_line(Reason::NoRelease));
    }
    let mut slot = client();
    let old = slot.take();
    let (stop, own) = AbortHandle::new_pair();
    let spawned = std::thread::Builder::new()
        .name("kawaro-portal".into())
        .spawn(move || {
            // At most one client: a closing one ends first.
            if let Some(old) = old {
                let _ = old.thread.join();
            }
            // No portal call after the fallback line, Session.Close
            // included: the portal drops the session with the connection.
            if let Some(r) = zbus::block_on(run(lang, &state, own)) {
                if portal::remembers(r) {
                    remember(&state);
                }
                kawaro_ui::stderr_line(&portal::unavailable_line(r));
            }
        });
    match spawned {
        Ok(thread) => *slot = Some(Client { stop, thread }),
        Err(e) => eprintln!("kawarod: portal thread: {e}"),
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

/// Runs the client until it falls back or `stop` is requested. A stop
/// closes the session, if there is one, and returns `Off`; the caller
/// prints the line once the connection is dropped.
async fn run(lang: Lang, state: &Shared, stop: AbortRegistration) -> Option<Reason> {
    let mut session = None;
    if let Ok(r) = Abortable::new(serve(lang, state, &mut session), stop).await {
        return r;
    }
    if let Some(s) = &session {
        let _ = s.close().await;
    }
    Some(Reason::Off)
}

/// Registers and binds, then feeds the signals to the gate until the
/// connection ends. Returns the reason of a fallback. The session is kept
/// in `slot` so that a stop can close it.
async fn serve(
    lang: Lang,
    state: &Shared,
    slot: &mut Option<Session<GlobalShortcuts>>,
) -> Option<Reason> {
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
        kawaro_ui::stderr_line(portal::REGISTRY_UNAVAILABLE);
    }
    let gs = match GlobalShortcuts::with_connection(conn).await {
        Ok(gs) => gs,
        Err(ashpd::Error::PortalNotFound(_)) => return portal::interface_reason(None),
        Err(_) => return Some(Reason::Error),
    };
    if let Some(r) = portal::interface_reason(Some(gs.version())) {
        return Some(r);
    }
    let Ok(created) = gs.create_session(Default::default()).await else {
        return Some(Reason::Error);
    };
    let session = slot.insert(created);
    // Before the bind, so no signal after the bound line is missed. One
    // stream keeps Activated and Deactivated in their order.
    let Ok(mut signals) = gs.receive_all_signals().await else {
        return Some(Reason::Error);
    };
    let shortcut = NewShortcut::new(portal::SHORTCUT_ID, portal::description(lang))
        .preferred_trigger(portal::PREFERRED_TRIGGER);
    let reply = match gs
        .bind_shortcuts(session, &[shortcut], None, Default::default())
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
    kawaro_ui::stderr_line(&portal::bound_line(&trigger));
    // The registration result the welcome captions read (S-087 AC3): the
    // key the portal actually bound, as its own reply describes it. A
    // backend that describes nothing gets the requested trigger in its user
    // spelling (the xkb `LOGO` is the Super key).
    let name = if trigger.is_empty() {
        portal::PREFERRED_TRIGGER
            .replace("LOGO", "Super")
            .replace("+z", "+Z")
    } else {
        trigger.clone()
    };
    crate::registration::record(kawaro_core::welcome::Registration::Fallback(name));

    let Some(own) = session_path(session) else {
        return Some(Reason::Error);
    };
    let debug = std::env::var_os("KAWARO_DEBUG_FOCUS").is_some();
    while let Some(msg) = signals.next().await {
        let header = msg.header();
        let Some(member) = header.member() else {
            continue;
        };
        // ShortcutsChanged has another body and is skipped here.
        let Ok((path, id, _, options)) =
            msg.body()
                .deserialize::<(OwnedObjectPath, String, u64, HashMap<String, OwnedValue>)>()
        else {
            continue;
        };
        if let Some(s) = portal::signal(&own, member.as_str(), path.as_str(), &id) {
            if debug {
                kawaro_ui::stderr_line(portal::debug_line(s));
            }
            // AC15: the press's activation_token, a string option, goes
            // with it to the gate, as the IPC token of `drop --hold` does.
            let strings: Vec<(&str, &str)> = options
                .iter()
                .filter_map(|(k, v)| match &**v {
                    zvariant::Value::Str(t) => Some((k.as_str(), t.as_str())),
                    _ => None,
                })
                .collect();
            let token = portal::activation_token(&strings);
            ui::hold_signal(s, token, Instant::now(), state.clone());
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
    // Decision 3: memory and file change together, under the lock a reload
    // takes, so the reload of this write sees no switch change. A save that
    // fails leaves memory equal to the file, so a later edit of another key
    // starts no client and opens no second dialog (S-014f3).
    let mut st = lock(state);
    let (mut config, warning) = Config::load_from(&path);
    if warning.is_some() {
        return;
    }
    let file = config.portal_shortcut;
    config.portal_shortcut = false;
    let saved = match config.save_to(&path) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("kawarod: cannot save {}: {e}", path.display());
            false
        }
    };
    st.config.portal_shortcut = portal::after_denial(file, saved);
}
