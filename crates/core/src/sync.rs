//! The one poison-ignoring lock of the core crate (S-029 AC5).

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks ignoring poison: every critical section in core is a plain map or
/// value update that cannot leave the data half written.
pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}
