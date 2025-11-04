use crate::utils::types::NecessarySendSync;
use crate::{disposable::subscription::Subscription, observable::Observable, observer::Observer};
use educe::Educe;

/// Emits a specified sequence of values before beginning to emit the items from the source Observable.
/// See <https://reactivex.io/documentation/operators/startwith.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::start_with::StartWith,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = StartWith::new(FromIter::new(vec![3, 4]), vec![1, 2]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct StartWith<OE, I> {
    source: OE,
    values: I,
}

impl<OE, I> StartWith<OE, I> {
    pub fn new<'or, 'sub, T, E>(source: OE, values: I) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        I: IntoIterator<Item = T>,
    {
        Self { source, values }
    }
}

impl<'or, 'sub, T, E, OE, I> Observable<'or, 'sub, T, E> for StartWith<OE, I>
where
    OE: Observable<'or, 'sub, T, E>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        for value in self.values.into_iter() {
            observer.on_next(value);
        }
        self.source.subscribe(observer)
    }
}
