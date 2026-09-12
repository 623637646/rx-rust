use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

/// Counts the number of items emitted by the source Observable and emits this count.
/// See <https://reactivex.io/documentation/operators/count.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::count::Count,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Count::new(FromIter::new(vec![1, 2, 3, 4]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Count<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> Count<T, OE> {
    pub fn new<'or, E>(source: OE) -> Self
    where
        OE: Observable<'or, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE> Observable<'or, usize, E> for Count<T, OE>
where
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(
        self,
        observer: impl Observer<usize, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = CountObserver { observer, count: 0 };
        self.source.subscribe(observer)
    }
}

struct CountObserver<OR> {
    observer: OR,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for CountObserver<OR>
where
    OR: Observer<usize, E>,
{
    fn on_next(&mut self, _: T) -> Flow {
        self.count += 1;
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && self.observer.on_next(self.count).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination)
    }
}
