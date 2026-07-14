use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

/// Emits only the Nth item emitted by the source Observable.
/// See <https://reactivex.io/documentation/operators/elementat.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::element_at::ElementAt,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = ElementAt::new(FromIter::new(vec![1, 2, 3]), 1);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ElementAt<OE> {
    source: OE,
    index: usize,
}

impl<OE> ElementAt<OE> {
    pub fn new(source: OE, index: usize) -> Self {
        Self { source, index }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for ElementAt<OE>
where
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
{
    type D = crate::utils::subscribe_with_auto_dispose_on_termination::Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            self.source.subscribe(ElementAtObserver {
                observer: Some(observer),
                index: self.index,
            })
        })
    }
}

struct ElementAtObserver<OR> {
    observer: Option<OR>,
    index: usize,
}

impl<T, E, OR> Observer<T, E> for ElementAtObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if self.observer.is_none() {
            return;
        }
        if self.index == 0 {
            if let Some(mut observer) = self.observer.take() {
                observer.on_next(value);
                observer.on_termination(Termination::Completed);
            }
        } else {
            self.index -= 1;
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            observer.on_termination(termination);
        }
    }
}
