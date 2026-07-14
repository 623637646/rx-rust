use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::num::NonZeroUsize;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, when the bundle reaches a specified size.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::buffer_with_count::BufferWithCount,
///     },
/// };
/// use std::num::NonZeroUsize;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = BufferWithCount::new(FromIter::new(vec![1, 2, 3, 4]), NonZeroUsize::new(3).unwrap());
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![vec![1, 2, 3], vec![4]]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> BufferWithCount<OE> {
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<'or, T, E, OE> Observable<'or, Vec<T>, E> for BufferWithCount<OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = BufferWithCountObserver {
            observer,
            values: Vec::default(),
            count: self.count,
        };
        self.source.subscribe(observer)
    }
}

struct BufferWithCountObserver<T, OR> {
    observer: OR,
    values: Vec<T>,
    count: NonZeroUsize,
}

impl<T, E, OR> Observer<T, E> for BufferWithCountObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.push(value);
        if self.values.len() >= self.count.get() {
            self.observer.on_next(std::mem::take(&mut self.values));
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if !self.values.is_empty() {
                    self.observer.on_next(std::mem::take(&mut self.values));
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination);
    }
}
