use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Applies a function to each item emitted by an Observable, sequentially, and emits the final accumulated value.
/// See <https://reactivex.io/documentation/operators/reduce.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::reduce::Reduce,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Reduce::new(FromIter::new(vec![1, 2, 3]), 0, |acc, value| acc + value);
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
pub struct Reduce<T, OE, F> {
    source: OE,
    initial_value: T,
    callback: F,
}

impl<T, OE, F> Reduce<T, OE, F> {
    pub fn new<'or, T1, E>(source: OE, initial_value: T, callback: F) -> Self
    where
        OE: Observable<'or, T = T1, E = E>,
        F: FnMut(T, T1) -> T,
    {
        Self {
            source,
            initial_value,
            callback,
        }
    }
}

impl<'or, T, T1, E, OE, F> Observable<'or> for Reduce<T, OE, F>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T = T1, E = E>,
    F: FnMut(T, T1) -> T + MaybeSend + 'or,
{
    type T = T;
    type E = E;
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = ReduceObserver {
            observer,
            value: Some(self.initial_value),
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

struct ReduceObserver<T, OR, F> {
    observer: OR,
    value: Option<T>,
    callback: F,
}

impl<T, T1, E, OR, F> Observer<T1, E> for ReduceObserver<T, OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(T, T1) -> T,
{
    fn on_next(&mut self, value: T1) {
        self.value = Some((self.callback)(self.value.take().unwrap(), value));
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.observer.on_next(self.value.unwrap());
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
