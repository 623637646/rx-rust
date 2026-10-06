//! The subscription of a future or stream adapter, made on the first poll and released when the
//! adapter is over.
//!
//! An adapter that turns an observable into something pulled — a `Future`, a `Stream` — must not
//! subscribe when it is built, since nothing is listening until it is first polled. It holds a
//! [`LazySubscription`] instead, calls [`subscribe_once`](LazySubscription::subscribe_once) from
//! every poll, and [`release`](LazySubscription::release)s the source as soon as it has its answer.
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observer::callback_observer::CallbackObserver,
//!     operators::creating::range::Range,
//!     utils::lazy_subscription::LazySubscription,
//! };
//!
//! let mut seen = Vec::new();
//! let mut subscription = LazySubscription::new(Range::new(1..4));
//! assert!(matches!(subscription, LazySubscription::Source(_))); // Nothing subscribed yet.
//!
//! subscription.subscribe_once(|| CallbackObserver::new(|value| seen.push(value), |_| {}));
//! assert!(matches!(subscription, LazySubscription::Subscribed(_)));
//!
//! assert_eq!(seen, [1, 2, 3]);
//!
//! // Every later call is a no-op: the source is not subscribed to again.
//! subscription.subscribe_once(|| CallbackObserver::new(|value| seen.push(value), |_| {}));
//! assert_eq!(seen, [1, 2, 3]);
//!
//! subscription.release(); // Disposes the subscription.
//! assert!(matches!(subscription, LazySubscription::Released));
//! ```

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};

/// Where a future or stream adapter stands with its source: the source it has not subscribed to
/// yet, the subscription it holds, or nothing once it has released it.
///
/// The subscription is the parameter `S`, always `Subscription<OE::Disposal>`, rather than spelled in
/// the variant: there, the drop check asks the disposal type to outlive the adapter before
/// normalizing it, so the projection `OE::Disposal` drags along every borrow of the source, and an
/// adapter over a source that borrows a local would hold that borrow until the adapter is
/// dropped (`test_mut_ref` of the stream tests).
pub enum LazySubscription<OE, S> {
    /// Not polled yet.
    Source(OE),
    /// Subscribed to the source; dropping it disposes the subscription.
    Subscribed(S),
    /// The adapter is over; it never subscribes again.
    Released,
}

impl<OE> LazySubscription<OE, Subscription<OE::Disposal>>
where
    OE: ObservableTypes,
{
    /// Holds `source` until the first [`subscribe_once`](Self::subscribe_once).
    pub fn new(source: OE) -> Self {
        Self::Source(source)
    }

    /// Subscribes `observer()` to the source on the first call, and does nothing afterwards.
    ///
    /// A synchronous source delivers everything from inside `subscribe`, while this is
    /// [`Released`](Self::Released); the subscription is stored once `subscribe` returns.
    pub fn subscribe_once<OR>(&mut self, observer: impl FnOnce() -> OR)
    where
        OR: Observer<OE::Item, OE::Error>,
        OE: Observable<OR>,
    {
        match std::mem::replace(self, Self::Released) {
            Self::Source(source) => *self = Self::Subscribed(source.subscribe(observer())),
            other => *self = other,
        }
    }

    /// Releases the subscription now instead of whenever the adapter itself is dropped.
    pub fn release(&mut self) {
        *self = Self::Released;
    }
}
