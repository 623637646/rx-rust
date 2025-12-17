use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Suppresses the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/skip.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::skip::Skip,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Skip::new(FromIter::new(vec![1, 2, 3, 4]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Skip<OE> {
    source: OE,
    count: usize,
}

impl<OE> Skip<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Skip<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        self.source.subscribe(SkipObserver {
            observer,
            count: self.count,
        })
    }
}

struct SkipObserver<OR> {
    observer: OR,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for SkipObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if self.count > 0 {
            self.count -= 1;
        } else {
            self.observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
