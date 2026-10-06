//! The [`Count`] operator, behind
//! [`ObservableExt::count`](crate::observable::ObservableExt::count).

use crate::utils::MarkerType;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
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
    /// Creates a [`Count`] over `source`;
    /// [`ObservableExt::count`](crate::observable::ObservableExt::count) is the fluent form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE> ObservableTypes for Count<T, OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = usize;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Count<T, OE>
where
    OR: Observer<usize, E>,
    OE: Observable<CountObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = CountObserver { observer, count: 0 };
        self.source.subscribe(observer)
    }
}

pub struct CountObserver<OR> {
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
