//! A shared handle to the sending end of a stream that the test cannot own directly, because the
//! observer it sends through only exists once the code under test subscribes.
//!
//! The observer is parked in a [`SerializedDelivery`], so a test never notifies it while holding
//! this handle's lock: a notification that re-enters the handle — a resubscription that fills it
//! again, or a second event — would otherwise deadlock, or panic in single-threaded builds.
//!
//! The handle starts empty, holds the observer from [`set`](SharedSender::set) until the stream
//! terminates, and is empty again afterwards, which is how a test tells a subscription that is
//! still live apart from one that is gone.

use educe::Educe;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::utils::mutable::MutableHelper;
use rx_rust::{
    observer::{Observer, Termination},
    utils::{
        mutable::Mutable, pending_events::EventBatch, serialized_delivery::SerializedDelivery,
        types::Shared,
    },
};

type Delivery<T, E, OR> = SerializedDelivery<T, E, OR, ()>;

/// A shared, initially empty handle sending through at most one observer.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct SharedSender<T, E, OR>(Shared<Mutable<Option<Delivery<T, E, OR>>>>);

impl<T, E, OR> Default for SharedSender<T, E, OR> {
    fn default() -> Self {
        Self(Shared::new(Mutable::new(None)))
    }
}

impl<T, E, OR> SharedSender<T, E, OR>
where
    OR: Observer<T, E>,
{
    /// Starts with `observer` already parked here, for a stream whose observer the test owns
    /// before the code under test runs.
    pub(crate) fn new(observer: OR) -> Self {
        Self(Shared::new(Mutable::new(Some(SerializedDelivery::idle(
            observer,
            (),
        )))))
    }

    /// Parks `observer` here, returning whether nothing was parked before.
    pub(crate) fn set(&self, observer: OR) -> bool {
        self.0
            .replace_value(Some(SerializedDelivery::idle(observer, ())))
            .is_none()
    }

    /// Whether no observer is parked here, either because none was set or because the stream was
    /// terminated and the observer dropped.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.with_ref(Option::is_none)
    }

    /// Notifies the parked observer, returning whether one was parked.
    pub(crate) fn on_next(&self, value: T) -> bool {
        match self.0.clone_value() {
            Some(delivery) => delivery.send(EventBatch::Next(value)),
            None => false,
        }
    }

    /// Drops the parked observer without notifying it, and empties this handle, returning whether
    /// one was parked.
    ///
    /// A delivery that is running stops as soon as it looks for its next event, so this also ends
    /// a stream from inside the delivery it is re-entering.
    pub(crate) fn stop(&self) -> bool {
        match self.0.take_value() {
            Some(delivery) => {
                delivery.stop();
                true
            }
            None => false,
        }
    }

    /// Terminates the parked observer and empties this handle, returning whether one was parked.
    pub(crate) fn on_termination(&self, termination: Termination<E>) -> bool {
        match self.0.take_value() {
            Some(delivery) => delivery.send(EventBatch::Termination(termination)),
            None => false,
        }
    }
}
