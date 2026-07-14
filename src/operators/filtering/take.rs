use crate::delegate_disposal;
use crate::disposable::option_disposal::OptionDisposal;
use crate::disposable::{Disposable, DisposableExt};
use crate::observable::Subscription;
use crate::utils::subscribe_with_auto_dispose_on_termination;
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

/// Emits only the first N items emitted by an Observable.
/// See <https://reactivex.io/documentation/operators/take.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::take::Take,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Take::new(FromIter::new(vec![1, 2, 3, 4]), 2);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Take<OE> {
    source: OE,
    count: usize,
}

impl<OE> Take<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

delegate_disposal!(
    Disposal<D>,
    OptionDisposal<Subscription<subscribe_with_auto_dispose_on_termination::Disposal<D>>>,
    where D: Disposable
);

impl<'or, T, E, OE> Observable<'or, T, E> for Take<OE>
where
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
{
    type D = Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        if self.count == 0 {
            observer.on_termination(Termination::Completed);
            OptionDisposal::none().into()
        } else {
            subscribe_with_auto_dispose_on_termination(observer, |observer| {
                self.source.subscribe(TakeObserver {
                    observer: Some(observer),
                    count: self.count,
                })
            })
            .into_option()
            .into()
        }
    }
}

struct TakeObserver<OR> {
    observer: Option<OR>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for TakeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = &mut self.observer {
            observer.on_next(value);
            self.count -= 1;
            if self.count == 0 {
                self.observer
                    .take()
                    .unwrap()
                    .on_termination(Termination::Completed);
            }
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
