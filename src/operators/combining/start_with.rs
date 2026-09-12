use crate::disposable::{DisposableExt, option_disposal::OptionDisposal};
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::Observer,
};
use educe::Educe;

/// Emits a specified sequence of values before beginning to emit the items from the source Observable.
/// See <https://reactivex.io/documentation/operators/startwith.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE, values: I) -> Self
    where
        OE: Observable<'or, T, E>,
        I: IntoIterator<Item = T>,
    {
        Self { source, values }
    }
}

impl<'or, T, E, OE, I> Observable<'or, T, E> for StartWith<OE, I>
where
    OE: Observable<'or, T, E>,
    I: IntoIterator<Item = T>,
{
    type D = OptionDisposal<Subscription<OE::D>>;

    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        for value in self.values.into_iter() {
            if observer.on_next(value).is_stop() {
                // The prepended values ended the stream, so the source is never subscribed to and
                // there is nothing to dispose of.
                return OptionDisposal::none().into_subscription();
            }
        }
        self.source
            .subscribe(observer)
            .into_option()
            .into_subscription()
    }
}
