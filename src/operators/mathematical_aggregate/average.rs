use crate::utils::types::{MarkerType, NecessarySend};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

/// Calculates the average of numbers emitted by an Observable and emits this average.
/// See <https://reactivex.io/documentation/operators/average.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Average<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> Average<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
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

        impl<'or, 'sub, E, OE> Observable<'or, 'sub, f64, E> for Average<$t, OE>
        where
            OE: Observable<'or, 'sub, $t, E>,
        {
            fn subscribe(self, observer: impl Observer<f64, E> + NecessarySend + 'or) -> Subscription<'sub> {
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
