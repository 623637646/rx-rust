//! The [`All`] operator, behind [`ObservableExt::all`](crate::observable::ObservableExt::all).

use crate::observable::Subscription;
use crate::utils::MarkerType;
use crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver;
use crate::utils::subscribe_with_auto_dispose_on_termination::{
    self, subscribe_with_auto_dispose_on_termination,
};
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

/// Emits a single boolean value that indicates whether all items emitted by a source Observable satisfy a specified condition.
/// See <https://reactivex.io/documentation/operators/all.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::all::All,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = All::new(FromIter::new(vec![1, 2, 3]), |value| value < 5);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![true]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct All<T, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T>,
}

impl<T, OE, F> All<T, OE, F> {
    /// Creates an [`All`] over `source`;
    /// [`ObservableExt::all`](crate::observable::ObservableExt::all) is the fluent form.
    pub fn new<E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnMut(T) -> bool,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE, F> ObservableTypes for All<T, OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(T) -> bool,
{
    type Item = bool;
    type Error = E;
    type Mode = OE::Mode;
    type D = subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode, OE::D>;
}

impl<T, E, OE, F, OR> Observable<OR> for All<T, OE, F>
where
    OR: Observer<bool, E>,
    OE: Observable<
            AllObserver<
                AutoDisposeOnTerminationObserver<
                    <OE as ObservableTypes>::Mode,
                    OR,
                    <OE as ObservableTypes>::D,
                >,
                F,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut(T) -> bool,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let observer = AllObserver {
                observer: Some(observer),
                callback: self.callback,
            };
            self.source.subscribe(observer)
        })
    }
}

pub struct AllObserver<OR, F> {
    observer: Option<OR>,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for AllObserver<OR, F>
where
    OR: Observer<bool, E>,
    F: FnMut(T) -> bool,
{
    fn on_next(&mut self, value: T) -> Flow {
        // A source that does not honor the flow or the disposal keeps emitting; the callback is
        // the caller's and may have side effects, so it must not run once the result was decided.
        if self.observer.is_none() {
            return Flow::Stop;
        }
        if (self.callback)(value) {
            return Flow::Continue;
        }
        // One value that fails decides the result, so the rest of the source is of no use.
        let Some(mut observer) = self.observer.take() else {
            return Flow::Stop;
        };
        if observer.on_next(false).is_continue() {
            observer.on_termination(Termination::Completed);
        }
        Flow::Stop
    }

    fn on_termination(mut self, termination: Termination<E>) {
        let Some(mut observer) = self.observer.take() else {
            return;
        };
        // The result is the last value of the stream, so a downstream that stopped on it is not
        // completed on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed) && observer.on_next(true).is_stop() {
            return;
        }
        observer.on_termination(termination);
    }
}
