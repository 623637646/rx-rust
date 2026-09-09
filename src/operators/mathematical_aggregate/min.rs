use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
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
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, T, E>,
    {
        Self { source }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for Min<OE>
where
    T: PartialOrd + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = MinObserver {
            observer,
            min: None,
        };
        self.source.subscribe(observer)
    }
}

struct MinObserver<T, OR> {
    observer: OR,
    min: Option<T>,
}

impl<T, E, OR> Observer<T, E> for MinObserver<T, OR>
where
    T: PartialOrd,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(min) = &mut self.min {
            if value < *min {
                *min = value;
            }
        } else {
            self.min = Some(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(min) = self.min {
                    self.observer.on_next(min);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
