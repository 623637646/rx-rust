use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::ops::AddAssign;

/// Calculates the sum of numbers emitted by an Observable and emits this sum.
/// See <https://reactivex.io/documentation/operators/sum.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::sum::Sum,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Sum::new(FromIter::new(vec![1, 2, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![6]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Sum<OE> {
    source: OE,
}

impl<OE> Sum<OE> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Sum<OE>
where
    T: AddAssign + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = SumObserver {
            observer,
            sum: None,
        };
        self.source.subscribe(observer)
    }
}

struct SumObserver<T, OR> {
    observer: OR,
    sum: Option<T>,
}

impl<T, E, OR> Observer<T, E> for SumObserver<T, OR>
where
    T: AddAssign,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(sum) = &mut self.sum {
            *sum += value;
        } else {
            self.sum = Some(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(sum) = self.sum {
                    self.observer.on_next(sum);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
