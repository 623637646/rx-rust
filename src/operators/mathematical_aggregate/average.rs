//! The [`Average`] operator, behind
//! [`ObservableExt::average`](crate::observable::ObservableExt::average).

use crate::utils::MarkerType;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

/// Calculates the average of numbers emitted by an Observable and emits this average.
/// See <https://reactivex.io/documentation/operators/average.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::average::Average,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Average::new(FromIter::new(vec![1.0_f64, 3.0, 5.0]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![3.0]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Average<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> Average<T, OE> {
    /// Creates an [`Average`] over `source`;
    /// [`ObservableExt::average`](crate::observable::ObservableExt::average) is the fluent form.
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

pub struct AverageObserver<T, OR> {
    observer: OR,
    /// Accumulated in `f64`, the type of the result. Summing in the source's own type overflows
    /// for the narrow ones long before the stream ends: three `100u8` items already exceed
    /// `u8::MAX`.
    sum: f64,
    count: usize,
    _marker: MarkerType<T>,
}

macro_rules! average_observer_impl {
    ($($t:ty)*) => ($(

        impl<E, OE> ObservableTypes for Average<$t, OE>
        where
            OE: ObservableTypes<Item = $t, Error = E>,
        {
            type Item = f64;
            type Error = E;
            type Mode = OE::Mode;
            type D = OE::D;
        }

        impl<E, OE, OR> Observable<OR> for Average<$t, OE>
        where
            OR: Observer<f64, E>,
            OE: Observable<AverageObserver<$t, OR>, Item = $t, Error = E>,
        {
            fn subscribe(self, observer: OR) -> Subscription<Self::D> {
                let observer = AverageObserver {
                    observer,
                    sum: 0f64,
                    count: 0,
                    _marker: PhantomData,
                };
                self.source.subscribe(observer)
            }
        }

        impl<E, OR> Observer<$t, E> for AverageObserver<$t, OR>
        where
            OR: Observer<f64, E>,
        {
            fn on_next(&mut self, value: $t) -> Flow {
                self.sum += value as f64;
                self.count += 1;
                Flow::Continue
            }

            fn on_termination(mut self, termination: Termination<E>) {
                // The final value ends the stream, so a downstream that stopped on it is not
                // completed on top of that: it has already ended itself.
                if matches!(termination, Termination::Completed)
                    && self.count != 0
                    && self.observer.on_next(self.sum / self.count as f64).is_stop()
                {
                    return;
                }
                self.observer.on_termination(termination)
            }
        }

    )*)
}

average_observer_impl! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 f32 f64 }
