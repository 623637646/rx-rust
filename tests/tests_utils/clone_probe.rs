//! A value that reports its own clone.

use rx_rust::thread_mode::mutable::MutableExt;
use std::sync::{Arc, Mutex};

/// A callback run while a [`CloneProbe`] is being cloned.
pub(crate) type CloneCallback = Box<dyn FnOnce() + Send>;

/// A value that runs a callback the first time it, or any copy of it, is cloned.
///
/// The counterpart of [`DropProbe`](super::drop_probe::DropProbe) for `Clone`: a subject keeps or
/// hands out copies of the values it forwards, and cloning runs user code just as dropping does, so
/// it must not happen while the state of the subject is locked. A test re-enters the code under
/// test from the callback; a clone that broke that rule would panic on the re-entry check of debug
/// builds, and deadlock otherwise, instead of running it.
///
/// The copies share the callback, so it runs once, from whichever clone comes first.
pub(crate) struct CloneProbe {
    value: i32,
    on_clone: Arc<Mutex<Option<CloneCallback>>>,
}

impl CloneProbe {
    pub(crate) fn new(value: i32) -> Self {
        Self {
            value,
            on_clone: Arc::new(Mutex::new(None)),
        }
    }

    /// Runs `callback` the first time this probe, or a copy of it, is cloned.
    pub(crate) fn on_clone(self, callback: CloneCallback) -> Self {
        let _previous = self.on_clone.replace_value(Some(callback));
        self
    }

    pub(crate) fn value(&self) -> i32 {
        self.value
    }
}

impl Clone for CloneProbe {
    fn clone(&self) -> Self {
        // Taken out of the lock before it runs: the callback clones probes too.
        if let Some(callback) = self.on_clone.take_value() {
            callback();
        }
        Self {
            value: self.value,
            on_clone: self.on_clone.clone(),
        }
    }
}
