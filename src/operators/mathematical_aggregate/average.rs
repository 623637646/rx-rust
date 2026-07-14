use crate::utils::types::{MarkerType, MaybeSend};
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
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
    pub fn new<'or, E>(source: OE) -> Self
    where
        OE: Observable<'or, T = T, E = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

struct AverageObserver<T, OR> {
    observer: OR,
    sum: T,
    count: usize,
}

macro_rules! average_observer_impl {
    ($($t:ty)*) => ($(

        impl<'or, E, OE> Observable<'or> for Average<$t, OE>
        where
            OE: Observable<'or, T = $t, E = E>,
        {
    type T = f64;
    type E = E;
            type D = OE::D;

            fn subscribe(self, observer: impl Observer<f64, E> + MaybeSend + 'or) -> Subscription<Self::D> {
                let observer = AverageObserver {
                    observer,
                    sum: 0 as $t,
                    count: 0,
                };
                self.source.subscribe(observer)
            }
        }

        impl<E, OR> Observer<$t, E> for AverageObserver<$t, OR>
        where
            OR: Observer<f64, E>,
        {
            fn on_next(&mut self, value: $t) {
                self.sum += value;
                self.count += 1;
            }

            fn on_termination(mut self, termination: Termination<E>) {
                match termination {
                    Termination::Completed => {
                        if self.count != 0 {
                            self.observer.on_next(self.sum as f64 / self.count as f64);
                        }
                    }
                    Termination::Error(_) => {}
                }
                self.observer.on_termination(termination)
            }
        }

    )*)
}

average_observer_impl! { usize u8 u16 u32 u64 u128 isize i8 i16 i32 i64 i128 f32 f64 }
