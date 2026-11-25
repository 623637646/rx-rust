//! Lets a callback decide what to forward for each item.
//!
//! The [`HookOnNext`] operator, behind
//! [`ObservableExt::hook_on_next`](crate::observable::ObservableExt::hook_on_next).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination, emitter::Emitter},
};
use educe::Educe;

/// Hands each item of the source observable, together with the downstream observer, to a callback
/// that decides what to forward.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_next::HookOnNext,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = HookOnNext::new(FromIter::new([1, 2]), |observer, value| {
///     observer.on_next(value * 10)
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![10, 20]);
/// ```
///
/// # `new` or `new_dyn`
///
/// As with [`Create`](crate::operators::creating::create::Create), a closure's parameter is one
/// concrete type. [`new`](HookOnNext::new) hands the callback an [`Emitter`], the downstream
/// observer unboxed, so the operator subscribes that one observer type only — and the callback's
/// `on_next` is a static call, inlined into the operator.
/// [`new_dyn`](HookOnNext::new_dyn) makes a `HookOnNext<.., true>`, whose callback gets the
/// observer behind a `&mut dyn `[`Observer`], so it subscribes any observer, at the cost of a
/// virtual call per item. Both are this one type: `DYN` only picks the [`Observable`] impl.
///
/// Unlike the `new_boxed` of
/// [`HookOnTermination`](crate::operators::others::hook_on_termination::HookOnTermination) and
/// [`HookOnSubscription`](crate::operators::others::hook_on_subscription::HookOnSubscription),
/// the erased form here allocates nothing: a callback that only emits items needs the observer
/// `&mut`, and [`Observer::on_termination`], which takes `self` by value and is what a
/// [`BoxedObserver`](crate::observer::boxed_observer::BoxedObserver) is for, is out of its reach
/// either way.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnNext<OE, F, const DYN: bool = false> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnNext<OE, F> {
    /// Creates a [`HookOnNext`] over `source` whose callback gets the unboxed downstream observer;
    /// [`ObservableExt::hook_on_next`](crate::observable::ObservableExt::hook_on_next) is the
    /// fluent form. `OR` is not stored: it only gives the callback its expected signature.
    pub fn new<OR>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes,
        OR: Observer<OE::Item, OE::Error>,
        F: FnMut(&mut Emitter<OE::Mode, OR>, OE::Item) -> Flow,
    {
        Self { source, callback }
    }

    /// Like [`new`](HookOnNext::new), but the callback gets the downstream observer behind a
    /// `&mut dyn `[`Observer`], so the operator subscribes any observer;
    /// [`ObservableExt::hook_on_next_dyn`](crate::observable::ObservableExt::hook_on_next_dyn) is
    /// the fluent form.
    pub fn new_dyn(source: OE, callback: F) -> HookOnNext<OE, F, true>
    where
        OE: ObservableTypes,
        F: FnMut(&mut dyn Observer<OE::Item, OE::Error>, OE::Item) -> Flow,
    {
        HookOnNext { source, callback }
    }
}

impl<OE: ObservableTypes, F, const DYN: bool> ObservableTypes for HookOnNext<OE, F, DYN> {
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for HookOnNext<OE, F, false>
where
    OR: Observer<T, E>,
    OE: Observable<
            HookOnNextObserver<Emitter<<OE as ObservableTypes>::Mode, OR>, F>,
            Item = T,
            Error = E,
        >,
    F: FnMut(&mut Emitter<<OE as ObservableTypes>::Mode, OR>, T) -> Flow,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = HookOnNextObserver {
            observer: Emitter::new(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

impl<T, E, OE, F, OR> Observable<OR> for HookOnNext<OE, F, true>
where
    OR: Observer<T, E>,
    OE: Observable<HookOnNextObserver<OR, F, true>, Item = T, Error = E>,
    F: FnMut(&mut dyn Observer<T, E>, T) -> Flow,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = HookOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

/// The observer [`HookOnNext`] subscribes its source with: it hands each item to the callback along
/// with the downstream observer, and forwards the termination. `DYN` picks how the callback sees
/// that observer, as the two forms cannot be told apart by their callback's type alone.
#[doc(hidden)]
pub struct HookOnNextObserver<OR, F, const DYN: bool = false> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnNextObserver<OR, F, false>
where
    OR: Observer<T, E>,
    F: FnMut(&mut OR, T) -> Flow,
{
    fn on_next(&mut self, value: T) -> Flow {
        (self.callback)(&mut self.observer, value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}

impl<T, E, OR, F> Observer<T, E> for HookOnNextObserver<OR, F, true>
where
    OR: Observer<T, E>,
    F: FnMut(&mut dyn Observer<T, E>, T) -> Flow,
{
    fn on_next(&mut self, value: T) -> Flow {
        (self.callback)(&mut self.observer, value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
