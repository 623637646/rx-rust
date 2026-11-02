//! An observer parked behind a context of the crate, which is how the test helpers deliver to an
//! observer with no lock of their own held.
//!
//! The context is the one every stateful operator is built on, with no model and no source of its
//! own: it serializes the events sent to the observer, delivers them outside its lock, and lets a
//! test send from inside a delivery, or from the drop of a value, without deadlocking.

use rx_rust::{
    disposable::Disposable,
    observer::{EventBatch, Flow, Observer},
    thread_mode::Shared,
    utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
};

/// A handle to a parked observer, cloned out of a helper's lock to send through it.
pub(crate) type ParkedSender<T, E, OR> = SubscriptionContext<Shared, T, E, OR, (), ()>;

/// An observer parked in a context, together with the disposal that stops it.
///
/// Dropping it only lets go of this handle: the observer is released, without a notification,
/// with the last handle, and a termination queued behind a running delivery is still delivered.
/// [`stop`](Self::stop) is what ends the stream early.
pub(crate) struct ParkedObserver<T, E, OR> {
    sender: ParkedSender<T, E, OR>,
    /// The bare disposal rather than the subscription that wraps it, which would stop the context
    /// when dropped.
    stop: subscribe_with_context::Disposal<Shared, T, E, (), ()>,
}

impl<T, E, OR> ParkedObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    pub(crate) fn new(observer: OR) -> Self {
        let mut sender = None;
        let mut stop = None;
        // The context owns no source: the builder only keeps a handle to it.
        let _ = subscribe_with_context(observer, (), |context| {
            sender = Some(context);
            Default::default()
        })
        .map_inner(|disposal| stop = Some(disposal));
        Self {
            sender: sender.expect("the builder runs before the context is returned"),
            stop: stop.expect("the subscription carries the disposal of the context"),
        }
    }

    /// A handle to send through, cloned so that it can be used once a helper's lock is released.
    pub(crate) fn sender(&self) -> ParkedSender<T, E, OR> {
        self.sender.clone()
    }

    /// Sends `events`, answering whether the observer still accepts more.
    pub(crate) fn send(&self, events: EventBatch<T, E>) -> Flow {
        self.sender.send(events)
    }

    /// Stops the stream: the observer is dropped without a notification, at once, unless a value
    /// is being delivered to it, in which case the delivery drops it as soon as it looks for its
    /// next event.
    pub(crate) fn stop(self) {
        self.stop.dispose();
        // The first handle dropped after the stop releases an observer left parked.
        drop(self.sender);
    }
}
