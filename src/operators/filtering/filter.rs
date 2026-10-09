//! The [`Filter`] operator, behind
//! [`ObservableExt::filter`](crate::observable::ObservableExt::filter).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits only those items from an Observable that pass a predicate test.
/// See <https://reactivex.io/documentation/operators/filter.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::filter::Filter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Filter::new(FromIter::new(vec![1, 2, 3, 4]), |value| *value % 2 == 0);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![2, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Filter<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Filter<OE, F> {
    /// Creates a [`Filter`] over `source`;
    /// [`ObservableExt::filter`](crate::observable::ObservableExt::filter) is the fluent form.
    pub fn new<T, E>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for Filter<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&T) -> bool,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, OR> Observable<OR> for Filter<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<FilterObserver<OR, F>, Item = T, Error = E>,
    F: FnMut(&T) -> bool,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = FilterObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

pub struct FilterObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for FilterObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) -> Flow {
        if (self.callback)(&value) {
            self.observer.on_next(value)
        } else {
            Flow::Continue
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
