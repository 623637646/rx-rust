use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

/// Emits only the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/take.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::take::Take,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Take::new(FromIter::new(vec![1, 2, 3, 4]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Take<OE> {
    source: OE,
    count: usize,
}

impl<OE> Take<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Take<OE>
where
    OE: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        if self.count == 0 {
            observer.on_termination(Termination::Completed);
            Subscription::default()
        } else {
            subscribe_unsub_after_termination(observer, |observer| {
                self.source.subscribe(TakeObserver {
                    observer: Some(observer),
                    count: self.count,
                })
            })
        }
    }
}

struct TakeObserver<OR> {
    observer: Option<OR>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = &mut self.observer {
            observer.on_next(value);
            self.count -= 1;
            if self.count == 0 {
                self.observer
                    .take()
                    .unwrap()
                    .on_termination(Termination::Completed);
            }
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
