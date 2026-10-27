//! What the daemon actually registered for the drag trigger (S-087 AC3):
//! the registration result the welcome captions read, never the configured
//! key. On Windows the trigger is the S-084 right-button drag; on Linux it
//! is what the GlobalShortcuts portal bound (the named fallback of S-075)
//! or nothing, until S-075's measured native paths land.

use kawaro_core::welcome::Registration;

/// The bind result of the running daemon; `Unbound` until something binds.
#[cfg(unix)]
static CURRENT: std::sync::Mutex<Registration> = std::sync::Mutex::new(Registration::Unbound);

/// Records a live registration result (a successful portal bind).
#[cfg(unix)]
pub fn record(registration: Registration) {
    if let Ok(mut current) = CURRENT.lock() {
        *current = registration;
    }
}

/// The registration result the captions read.
pub fn current() -> Registration {
    // The Windows trigger is a button, named by the S-084 manager decision
    // D3 and never a key.
    #[cfg(windows)]
    {
        Registration::RightDrag
    }
    #[cfg(not(windows))]
    {
        match CURRENT.lock() {
            Ok(current) => current.clone(),
            Err(_) => Registration::Unbound,
        }
    }
}
