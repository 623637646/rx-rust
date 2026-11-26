//! Emits given values before the items of the source.
//!
//! The [`StartWith`] operator, behind
//! [`ObservableExt::start_with`](crate::observable::ObservableExt::start_with).

use crate::{
    delegate_disposal,
    disposable::{DisposableExt, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::Observer,
};
use educe::Educe;

/// Emits a specified sequence of values, then the items of the source observable.
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
///
/// let observable = StartWith::new(FromIter::new([3, 4]), [1, 2]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct StartWith<OE, I> {
    source: OE,
    values: I,
}

impl<OE, I> StartWith<OE, I> {
    /// Creates a [`StartWith`] over `source`;
    /// [`ObservableExt::start_with`](crate::observable::ObservableExt::start_with) is the fluent
    /// form.
    pub fn new<T>(source: OE, values: I) -> Self
    where
        OE: ObservableTypes<Item = T>,
        I: IntoIterator<Item = T>,
    {
        Self { source, values }
    }
}

delegate_disposal!(Disposal<D>, Option<D>);

impl<T, E, OE, I> ObservableTypes for StartWith<OE, I>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal>;
}

impl<T, E, OE, I, OR> Observable<OR> for StartWith<OE, I>
where
    OR: Observer<T, E>,
    OE: Observable<OR, Item = T, Error = E>,
    I: IntoIterator<Item = T>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // The observer is declared last, so that a stop drops it before the values left, an unwind
        // included (decision 0011).
        let (mut values, mut observer) = (self.values.into_iter(), observer);
        let flow = values.try_for_each(|value| observer.on_next(value));
        if flow.is_break() {
            // The prepended values ended the stream, so the source is never subscribed to and
            // there is nothing to dispose of.
            return None.into_dispose_on_drop();
        }
        // No value is left, but the iterator may hold more than values: gone before the source
        // starts.
        drop(values);
        self.source
            .subscribe(observer)
            .map_inner(Some)
            .map_inner_into()
    }
}
