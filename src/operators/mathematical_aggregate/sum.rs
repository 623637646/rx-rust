use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::ops::AddAssign;

/// Calculates the sum of numbers emitted by an Observable and emits this sum.
/// See <https://reactivex.io/documentation/operators/sum.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, T, E>,
    {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for Sum<OE>
where
    T: AddAssign + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
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
    fn on_next(&mut self, value: T) -> Flow {
        if let Some(sum) = &mut self.sum {
            *sum += value;
        } else {
            self.sum = Some(value);
        }
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && let Some(sum) = self.sum.take()
            && self.observer.on_next(sum).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination)
    }
}
