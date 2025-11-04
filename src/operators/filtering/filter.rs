use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Emits only those items from an Observable that pass a predicate test.
/// See <https://reactivex.io/documentation/operators/filter.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for Filter<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> bool + NecessarySendSync + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        let observer = FilterObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct FilterObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for FilterObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) {
        if (self.callback)(&value) {
            self.observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
