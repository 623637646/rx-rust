//! The [`HookOnTermination`] operator, behind
//! [`ObservableExt::hook_on_termination`](crate::observable::ObservableExt::hook_on_termination).

use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::observer::emitter::Emitter;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Hands the termination of the source Observable, together with the downstream observer, to a
/// callback that decides what to deliver.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_termination::HookOnTermination,
///     },
/// };
/// use rx_rust::observer::Observer;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable =
///     HookOnTermination::new(FromIter::new(vec![1]), move |observer, termination| {
///         // The callback decides what reaches the observer: here, the termination as it is.
///         observer.on_termination(termination);
///     });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
///
/// # `new` or `new_boxed`
///
/// As with [`Create`](crate::operators::creating::create::Create), a closure's parameter is one
/// concrete type. [`new`](HookOnTermination::new) hands the callback an [`Emitter`], the
/// downstream observer unboxed, so the operator subscribes that one observer type only.
/// [`new_boxed`](HookOnTermination::new_boxed) makes a `HookOnTermination<.., true>`, whose
/// callback gets the boxed observer of the source's mode, so it subscribes any observer, at the
/// cost of one allocation per subscription. Both are this one type: `BOXED` only picks the
/// [`Observable`] impl, and the lifetime the boxed observer may borrow for is not a parameter.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnTermination<OE, F, const BOXED: bool = false> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnTermination<OE, F> {
    /// Creates a [`HookOnTermination`] over `source` whose callback gets the unboxed downstream
    /// observer;
    /// [`ObservableExt::hook_on_termination`](crate::observable::ObservableExt::hook_on_termination)
    /// is the fluent form. `OR` is not stored: it only gives the callback its expected signature.
    pub fn new<OR>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes,
        OR: Observer<OE::Item, OE::Error>,
        F: FnOnce(Emitter<OR, OE::Mode>, Termination<OE::Error>),
    {
        Self { source, callback }
    }

    /// Like [`new`](HookOnTermination::new), but the callback gets the boxed observer of the
    /// source's mode, so the operator subscribes any observer;
    /// [`ObservableExt::hook_on_termination_boxed`](crate::observable::ObservableExt::hook_on_termination_boxed)
    /// is the fluent form.
    pub fn new_boxed<'a>(source: OE, callback: F) -> HookOnTermination<OE, F, true>
    where
        OE: ObservableTypes,
        OE::Mode: ObserverMode,
        F: FnOnce(
            <OE::Mode as ObserverMode>::BoxedObserver<'a, OE::Item, OE::Error>,
            Termination<OE::Error>,
        ),
    {
        HookOnTermination { source, callback }
    }
}

impl<OE: ObservableTypes, F, const BOXED: bool> ObservableTypes
    for HookOnTermination<OE, F, BOXED>
{
    type Item = OE::Item;
    type Error = OE::Error;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for HookOnTermination<OE, F, false>
where
    OR: Observer<T, E>,
    OE: Observable<
            HookOnTerminationObserver<Emitter<OR, <OE as ObservableTypes>::Mode>, F>,
            Item = T,
            Error = E,
        >,
    F: FnOnce(Emitter<OR, <OE as ObservableTypes>::Mode>, Termination<E>),
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = HookOnTerminationObserver {
            observer: Emitter::new(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

impl<'a, T, E, OE, F, OR> Observable<OR> for HookOnTermination<OE, F, true>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: IntoBoxedObserver<'a, T, E, <OE as ObservableTypes>::Mode>,
    OE: Observable<
            HookOnTerminationObserver<
                <<OE as ObservableTypes>::Mode as ObserverMode>::BoxedObserver<'a, T, E>,
                F,
            >,
            Item = T,
            Error = E,
        >,
    F: FnOnce(
        <<OE as ObservableTypes>::Mode as ObserverMode>::BoxedObserver<'a, T, E>,
        Termination<E>,
    ),
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = HookOnTerminationObserver {
            observer: <<OE as ObservableTypes>::Mode as ObserverMode>::boxed(observer),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

/// The observer [`HookOnTermination`] subscribes its source with: it forwards the values, and hands
/// the termination to the callback along with the downstream observer.
pub struct HookOnTerminationObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnTerminationObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(OR, Termination<E>),
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(self.observer, termination);
    }
}
