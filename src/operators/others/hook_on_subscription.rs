//! The [`HookOnSubscription`] operator, behind
//! [`ObservableExt::hook_on_subscription`](crate::observable::ObservableExt::hook_on_subscription).

use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::observer::emitter::Emitter;
use crate::utils::MarkerType;
use crate::{
    disposable::Disposable,
    observable::{Observable, ObservableTypes, Subscription},
    observer::Observer,
};
use educe::Educe;
use std::marker::PhantomData;

/// Invokes a callback when the Observable is subscribed to.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_subscription::HookOnSubscription,
///     },
/// };
/// use rx_rust::observable::{Observable, ObservableTypes};
/// use std::cell::Cell;
/// use std::rc::Rc;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
/// let subscribed = Rc::new(Cell::new(false));
/// let subscribed_flag = Rc::clone(&subscribed);
///
/// let observable = HookOnSubscription::new(FromIter::new(vec![1, 2]), move |source, observer| {
///     subscribed_flag.set(true);
///     source.subscribe(observer)
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert!(subscribed.get());
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
///
/// # `new` or `new_boxed`
///
/// As with [`Create`](crate::operators::creating::create::Create), a closure's parameter is one
/// concrete type. [`new`](HookOnSubscription::new) hands the callback an
/// [`Emitter`], the downstream observer unboxed, so the operator subscribes that one observer type
/// only. [`new_boxed`](HookOnSubscription::new_boxed) makes a `HookOnSubscription<.., true>`,
/// whose callback gets the boxed observer of the source's mode, so it subscribes any observer, at
/// the cost of one allocation per subscription. Both are this one type: `BOXED` only picks the
/// [`Observable`] impl, and the lifetime the boxed observer may borrow for is not a parameter.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnSubscription<OE, F, D, const BOXED: bool = false> {
    source: OE,
    callback: F,
    /// What the callback returns.
    _marker: MarkerType<D>,
}

impl<OE, F, D> HookOnSubscription<OE, F, D> {
    /// Creates a [`HookOnSubscription`] over `source` whose callback gets the unboxed downstream
    /// observer;
    /// [`ObservableExt::hook_on_subscription`](crate::observable::ObservableExt::hook_on_subscription)
    /// is the fluent form. `OR` is not stored: it only gives the callback its expected signature.
    pub fn new<OR>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes,
        OR: Observer<OE::Item, OE::Error>,
        D: Disposable,
        F: FnOnce(OE, Emitter<OR, OE::Mode>) -> Subscription<D>,
    {
        Self::with_callback(source, callback)
    }

    /// Like [`new`](HookOnSubscription::new), but the callback gets the boxed observer of the
    /// source's mode, so the operator subscribes any observer;
    /// [`ObservableExt::hook_on_subscription_boxed`](crate::observable::ObservableExt::hook_on_subscription_boxed)
    /// is the fluent form.
    pub fn new_boxed<'a>(source: OE, callback: F) -> HookOnSubscription<OE, F, D, true>
    where
        OE: ObservableTypes,
        OE::Mode: ObserverMode,
        D: Disposable,
        F: FnOnce(
            OE,
            <OE::Mode as ObserverMode>::BoxedObserver<'a, OE::Item, OE::Error>,
        ) -> Subscription<D>,
    {
        HookOnSubscription::with_callback(source, callback)
    }
}

impl<OE, F, D, const BOXED: bool> HookOnSubscription<OE, F, D, BOXED> {
    fn with_callback(source: OE, callback: F) -> Self {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<OE, F, D, const BOXED: bool> ObservableTypes for HookOnSubscription<OE, F, D, BOXED>
where
    OE: ObservableTypes,
    D: Disposable,
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    /// Whatever the callback returns.
    type D = D;
}

impl<OE, F, D, OR> Observable<OR> for HookOnSubscription<OE, F, D, false>
where
    OE: ObservableTypes,
    OR: Observer<<OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
    D: Disposable,
    F: FnOnce(OE, Emitter<OR, <OE as ObservableTypes>::Mode>) -> Subscription<D>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        (self.callback)(self.source, Emitter::new(observer))
    }
}

impl<'a, OE, F, D, OR> Observable<OR> for HookOnSubscription<OE, F, D, true>
where
    OE: ObservableTypes,
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: IntoBoxedObserver<
            'a,
            <OE as ObservableTypes>::Item,
            <OE as ObservableTypes>::Error,
            <OE as ObservableTypes>::Mode,
        >,
    D: Disposable,
    F: FnOnce(
        OE,
        <<OE as ObservableTypes>::Mode as ObserverMode>::BoxedObserver<
            'a,
            <OE as ObservableTypes>::Item,
            <OE as ObservableTypes>::Error,
        >,
    ) -> Subscription<D>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        (self.callback)(
            self.source,
            <<OE as ObservableTypes>::Mode as ObserverMode>::boxed(observer),
        )
    }
}
