use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

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
pub struct Count<OE> {
    source: OE,
}

impl<OE> Count<OE> {
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
    {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or> for Count<OE>
where
    OE: Observable<'or, T = T, E = E>,
{
    type T = usize;
    type E = E;
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
    fn on_next(&mut self, _: T) {
        self.count += 1;
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.observer.on_next(self.count);
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
