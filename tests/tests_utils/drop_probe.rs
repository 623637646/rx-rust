//! A value that reports its own drop.

use rx_rust::{
    safe_lock,
    utils::types::{Mutable, MutableHelper, Shared},
};

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// A callback run while a [`DropProbe`] is being dropped.
        pub(crate) type DropCallback = Box<dyn FnOnce()>;
    } else {
        /// A callback run while a [`DropProbe`] is being dropped.
        pub(crate) type DropCallback = Box<dyn FnOnce() + Send>;
    }
}

/// A value that runs a callback while it is being dropped.
///
/// A test sends it through the code under test on its own, or embeds it in a value of its own, to
/// check when that value is released, and to re-enter the code under test from a drop: nothing may
/// be dropped while the state of that code is locked, and a drop that broke that rule would panic
/// in single-threaded builds, and deadlock otherwise, instead of running its callback.
pub(crate) struct DropProbe(Option<DropCallback>);

impl DropProbe {
    pub(crate) fn new() -> Self {
        Self(None)
    }

    /// Runs `callback` when this probe is dropped.
    pub(crate) fn on_drop(mut self, callback: DropCallback) -> Self {
        self.0 = Some(callback);
        self
    }
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take() {
            callback();
        }
    }
}

/// How many probes counting into it were dropped.
#[derive(Clone)]
pub(crate) struct DropCount(Shared<Mutable<usize>>);

impl DropCount {
    pub(crate) fn new() -> Self {
        Self(Shared::new(Mutable::new(0)))
    }

    pub(crate) fn get(&self) -> usize {
        safe_lock!(clone: self.0)
    }

    pub(crate) fn increment(&self) {
        self.0.lock_mut(|mut lock| *lock += 1);
    }

    /// A callback counting one drop, for a probe that does something else as well.
    pub(crate) fn callback(&self) -> DropCallback {
        let count = self.clone();
        Box::new(move || count.increment())
    }

    /// A probe that only counts its drop, which is all most tests need.
    pub(crate) fn probe(&self) -> DropProbe {
        DropProbe::new().on_drop(self.callback())
    }
}
