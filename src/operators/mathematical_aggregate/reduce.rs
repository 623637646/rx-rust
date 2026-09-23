//! The [`Reduce`] operator, behind
//! [`ObservableExt::reduce`](crate::observable::ObservableExt::reduce).

use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

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
pub struct Reduce<T, T1, OE, F> {
    source: OE,
    initial_value: T,
    callback: F,
    _marker: MarkerType<T1>,
}

impl<T, T1, OE, F> Reduce<T, T1, OE, F> {
    /// Creates a [`Reduce`] over `source`;
    /// [`ObservableExt::reduce`](crate::observable::ObservableExt::reduce) is the fluent form.
    pub fn new<'or, E>(source: OE, initial_value: T, callback: F) -> Self
    where
        OE: Observable<'or, T1, E>,
        F: FnMut(T, T1) -> T,
    {
        Self {
            source,
            initial_value,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, T1, E, OE, F> Observable<'or, T, E> for Reduce<T, T1, OE, F>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T1, E>,
    F: FnMut(T, T1) -> T + MaybeSend + 'or,
{
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
    /// `None` only once the callback has panicked, which takes the accumulator with it.
    value: Option<T>,
    callback: F,
}

impl<T, T1, E, OR, F> Observer<T1, E> for ReduceObserver<T, OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(T, T1) -> T,
{
    fn on_next(&mut self, value: T1) -> Flow {
        let Some(accumulated) = self.value.take() else {
            // The callback panicked and the accumulator went with it, so there is nothing left to
            // accumulate into: this observer accepts nothing more.
            return Flow::Stop;
        };
        self.value = Some((self.callback)(accumulated, value));
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        let Some(accumulated) = self.value.take() else {
            // No accumulator left to emit, see `on_next`: the termination still goes downstream.
            return self.observer.on_termination(termination);
        };
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && self.observer.on_next(accumulated).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination)
    }
}
