//! The subscription an observer carries to subscribe an observable with an observer of its own
//! type.
//!
//! An observer that subscribes again with itself — or with another instance of its own type, as
//! the inner observer of [`ConcatAll`](crate::operators::combining::concat_all::ConcatAll) does
//! for the next inner observable — needs `OE: Observable<OR>` to make that call. It cannot ask for
//! it on its own `Observer` implementation: proving that implementation would then depend on
//! itself, since the observable's `Observable<OR>` implementation in turn requires `OR: Observer`,
//! and rustc rejects the cycle as an overflow (`E0275`).
//!
//! The bound does hold where the operator itself is subscribed, because the operator's
//! `Observable<OR>` implementation states it. [`Resubscribe::new`] is called there: the
//! subscription is monomorphized at that point and carried along as a function pointer, whose type
//! names no bound. The observer's `Observer` implementation then only needs
//! `OE: ObservableTypes`.
//!
//! An observer that subscribes with a *different* observer type, as
//! [`Switch`](crate::operators::combining::switch::Switch) or
//! [`Catch`](crate::operators::error_handling::catch::Catch) do, needs none of this.
//!
//! # Usage
//!
//! [`Retry`](crate::operators::error_handling::retry::Retry) is the reference: its observer holds
//! a `Resubscribe<OE1, Self>`, created in `Retry`'s `Observable<OR>::subscribe`, and calls it with
//! itself when the callback asks for a retry.

use crate::observable::{Observable, ObservableTypes, Subscription};
use crate::observer::Observer;
use educe::Educe;

/// Subscribes an `OE` with an `OR`, where `OR` is an observer that does so itself.
///
/// Create it with [`new`](Self::new) where `OE: Observable<OR>` holds — in the operator's
/// `Observable<OR>::subscribe` — and store it in the observer. See the
/// [module documentation](self) for why the bound cannot be stated on the observer instead.
#[derive(Educe)]
#[educe(Debug, Clone, Copy)]
pub struct Resubscribe<OE: ObservableTypes, OR>(
    #[educe(Debug(ignore))] fn(OE, OR) -> Subscription<OE::Disposal>,
);

impl<OE: ObservableTypes, OR> Resubscribe<OE, OR> {
    /// Captures the subscription of `OE` with `OR`, while the bound that allows it is in scope.
    pub fn new() -> Self
    where
        OE: Observable<OR>,
        OR: Observer<OE::Item, OE::Error>,
    {
        Self(|observable, observer| observable.subscribe(observer))
    }

    /// Subscribes `observable` with `observer`.
    pub fn subscribe(self, observable: OE, observer: OR) -> Subscription<OE::Disposal> {
        (self.0)(observable, observer)
    }
}

impl<OE, OR> Default for Resubscribe<OE, OR>
where
    OE: Observable<OR>,
    OR: Observer<OE::Item, OE::Error>,
{
    fn default() -> Self {
        Self::new()
    }
}
