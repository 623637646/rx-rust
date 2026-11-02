//! A lenient source for the tests: a shared handle to the observer a `Create` was subscribed with,
//! which keeps sending through it whatever the subscription says.
//!
//! [`test_channel`](super::test_channel::test_channel) panics on a value sent after its
//! subscription was disposed; this one delivers it. It is what a test uses to check how an
//! operator copes with a source that does not honor its disposal: an inner source of `switch_map`
//! that terminates after it was switched away from, or a source that terminates after the
//! subscription to it was disposed.
//!
//! The observer is parked in a [`ParkedObserver`], so a test never notifies it while holding this
//! handle's lock: a notification that re-enters the handle — a resubscription that fills it
//! again, or a second event — would otherwise deadlock, or fail the re-entry check of debug
//! builds.
//!
//! The handle starts empty, holds the observer from [`set`](SharedSender::set) until the stream
//! terminates, and is empty again afterwards, which is how a test tells a subscription that is
//! still live apart from one that is gone.

use crate::tests_utils::parked_observer::ParkedObserver;
use educe::Educe;
use rx_rust::observer::{EventBatch, Observer, Termination};
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use std::sync::{Arc, Mutex};

/// A shared, initially empty handle sending through at most one observer.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct SharedSender<T, E, OR>(
    #[educe(Debug(ignore))] Arc<Mutex<Option<ParkedObserver<T, E, OR>>>>,
);

impl<T, E, OR> Default for SharedSender<T, E, OR> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

impl<T, E, OR> SharedSender<T, E, OR>
where
    OR: Observer<T, E>,
{
    /// Parks `observer` here, returning whether nothing was parked before.
    pub(crate) fn set(&self, observer: OR) -> bool {
        self.0
            .replace_value(Some(ParkedObserver::new(observer)))
            .is_none()
    }

    /// Whether no observer is parked here, either because none was set or because the stream was
    /// terminated and the observer dropped.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.with_ref(Option::is_none)
    }

    /// Notifies the parked observer, returning whether one was parked.
    pub(crate) fn on_next(&self, value: T) -> bool {
        match self
            .0
            .with_ref(|parked| parked.as_ref().map(ParkedObserver::sender))
        {
            Some(sender) => {
                // What the observer answers is the caller's business, not this handle's: this
                // reports only whether there was one to notify.
                let _ = sender.send(EventBatch::Next(value));
                true
            }
            None => false,
        }
    }

    /// Terminates the parked observer and empties this handle, returning whether one was parked.
    pub(crate) fn on_termination(&self, termination: Termination<E>) -> bool {
        match self.0.take_value() {
            Some(parked) => {
                let _ = parked.send(EventBatch::Termination(termination));
                true
            }
            None => false,
        }
    }
}
