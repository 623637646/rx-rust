use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Emits the maximum item emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/max.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         mathematical_aggregate::max::Max,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Max::new(FromIter::new(vec![3, 5, 4]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![5]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Max<OE> {
    source: OE,
}

impl<OE> Max<OE> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Max<OE>
where
    T: PartialOrd + NecessarySendSync + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        let observer = MaxObserver {
            observer,
            max: None,
        };
        self.source.subscribe(observer)
    }
}

struct MaxObserver<T, OR> {
    observer: OR,
    max: Option<T>,
}

impl<T, E, OR> Observer<T, E> for MaxObserver<T, OR>
where
    T: PartialOrd,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(max) = &mut self.max {
            if value > *max {
                *max = value;
            }
        } else {
            self.max = Some(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(max) = self.max {
                    self.observer.on_next(max);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination)
    }
}
