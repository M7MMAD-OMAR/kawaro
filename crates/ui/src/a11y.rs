//! Whether a screen reader runs (S-018 AC12): the hive is then a normal
//! window, which the reader can follow, instead of the layer-shell overlay.
//! Read from `org.a11y.Status.ScreenReaderEnabled` on the session bus
//! (`org.a11y.Bus`, `/org/a11y/bus`), and kept current from its
//! `PropertiesChanged` signal on a thread of its own, so a show never waits
//! for the bus.

use std::sync::atomic::{AtomicBool, Ordering};
use zbus::blocking::Connection;
use zbus::blocking::fdo::PropertiesProxy;
use zbus::names::InterfaceName;

static ENABLED: AtomicBool = AtomicBool::new(false);

const STATUS: &str = "org.a11y.Status";
const PROPERTY: &str = "ScreenReaderEnabled";

/// The last value seen; false until the bus answered, or without one.
pub(crate) fn screen_reader() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Starts following the property. Call once.
pub(crate) fn watch() {
    std::thread::spawn(|| {
        let _ = follow();
    });
}

fn follow() -> zbus::Result<()> {
    let conn = Connection::session()?;
    let props = PropertiesProxy::builder(&conn)
        .destination("org.a11y.Bus")?
        .path("/org/a11y/bus")?
        .build()?;
    // Subscribe first, so a change between the read and the subscription
    // is not lost.
    let changes = props.receive_properties_changed()?;
    let status = InterfaceName::from_static_str_unchecked(STATUS);
    if let Ok(v) = props.get(status.clone(), PROPERTY) {
        ENABLED.store(bool::try_from(v).unwrap_or(false), Ordering::SeqCst);
    }
    for signal in changes {
        let Ok(args) = signal.args() else { continue };
        if args.interface_name() != &status {
            continue;
        }
        if let Some(v) = args.changed_properties().get(PROPERTY) {
            ENABLED.store(bool::try_from(v).unwrap_or(false), Ordering::SeqCst);
        }
    }
    Ok(())
}
