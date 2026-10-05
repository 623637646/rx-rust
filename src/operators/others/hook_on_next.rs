//! The [`HookOnNext`] operator, behind
//! [`ObservableExt::hook_on_next`](crate::observable::ObservableExt::hook_on_next).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Invokes a callback for each item emitted by the source Observable.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::hook_on_next::HookOnNext,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = HookOnNext::new(FromIter::new(vec![1, 2]), |observer, value| {
///     observer.on_next(value * 10)
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![10, 20]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct HookOnNext<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> HookOnNext<OE, F> {
    /// Creates a [`HookOnNext`] over `source`;
    /// [`ObservableExt::hook_on_next`](crate::observable::ObservableExt::hook_on_next) is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnMut(&mut dyn Observer<T, E>, T) -> Flow,
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for HookOnNext<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&mut dyn Observer<T, E>, T) -> Flow,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type D = OE::D;
}

impl<T, E, OE, F, OR> Observable<OR> for HookOnNext<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<HookOnNextObserver<OR, F>, Item = T, Error = E>,
    F: FnMut(&mut dyn Observer<T, E>, T) -> Flow,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observer = HookOnNextObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct HookOnNextObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for HookOnNextObserver<OR, F>
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
