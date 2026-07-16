use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Emits only the last N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/takelast.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::take_last::TakeLast,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = TakeLast::new(FromIter::new(vec![1, 2, 3, 4]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TakeLast<OE> {
    source: OE,
    count: usize,
}

impl<OE> TakeLast<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for TakeLast<OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.source.subscribe(TakeLastObserver {
            observer,
            buffer: VecDeque::default(),
            count: self.count,
        })
    }
}

struct TakeLastObserver<T, OR> {
    observer: OR,
    buffer: VecDeque<T>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeLastObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if self.count == 0 {
            return;
        }
        self.buffer.push_back(value);
        if self.buffer.len() > self.count {
            self.buffer.pop_front();
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                for value in self.buffer.into_iter() {
                    self.observer.on_next(value);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination);
    }
}
