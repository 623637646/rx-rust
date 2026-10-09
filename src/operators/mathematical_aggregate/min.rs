//! The [`Min`] operator, behind [`ObservableExt::min`](crate::observable::ObservableExt::min).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits the minimum item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/min.html>
///
/// `T` is only [`PartialOrd`], so values that do not compare — `f64::NAN` among them — are
/// never seen as smaller and are skipped. A `NaN` that arrives first is therefore kept as the
/// minimum for the rest of the stream, because nothing compares smaller than it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::min::Min,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Min::new(FromIter::new(vec![3, 1, 2]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Min<OE> {
    source: OE,
}

impl<OE> Min<OE> {
    /// Creates a [`Min`] over `source`;
    /// [`ObservableExt::min`](crate::observable::ObservableExt::min) is the fluent form.
    pub fn new<T, E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
    {
        Self { source }
    }
}

impl<T, E, OE> ObservableTypes for Min<OE>
where
    T: PartialOrd,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Min<OE>
where
    OR: Observer<T, E>,
    T: PartialOrd,
    OE: Observable<MinObserver<T, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = MinObserver {
            observer,
            min: None,
        };
        self.source.subscribe(observer)
    }
}

pub struct MinObserver<T, OR> {
    observer: OR,
    min: Option<T>,
}

impl<T, E, OR> Observer<T, E> for MinObserver<T, OR>
where
    T: PartialOrd,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if let Some(min) = &mut self.min {
            if value < *min {
                *min = value;
            }
        } else {
            self.min = Some(value);
        }
        Flow::Continue
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The final value ends the stream, so a downstream that stopped on it is not completed
        // on top of that: it has already ended itself.
        if matches!(termination, Termination::Completed)
            && let Some(min) = self.min.take()
            && self.observer.on_next(min).is_stop()
        {
            return;
        }
        self.observer.on_termination(termination)
    }
}
