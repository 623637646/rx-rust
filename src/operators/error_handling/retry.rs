use crate::delegate_disposal;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal,
};
use crate::observable::Subscription;
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub enum RetryAction<E, OE1> {
    Retry(OE1),
    Stop(E),
}

/// Retries an Observable in case of an error, based on a retry policy.
/// See <https://reactivex.io/documentation/operators/retry.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{just::Just, throw::Throw},
///         error_handling::retry::{Retry, RetryAction},
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Retry::new(Throw::new("boom").with_item_type(), |_| RetryAction::Retry(Just::new(42).with_error_type()));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Retry<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> Retry<OE, F> {
    pub fn new<'or, T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, T, E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Self { source, callback }
    }
}

delegate_disposal!(
    Disposal<D, D1>,
    ChainDisposal<SharedDisposal<Subscription<D1>>, D>,
    where D: Disposable, D1: Disposable
);

impl<'or, T, E, OE, OE1, F> Observable<'or, T, E> for Retry<OE, F>
where
    OE: Observable<'or, T, E>,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    F: FnMut(E) -> RetryAction<E, OE1> + MaybeSend + 'or,
{
    type D = Disposal<OE::D, OE1::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let shared_disposal = SharedDisposal::default();
        let observer = RetryObserver {
            observer,
            callback: self.callback,
            shared_disposal: shared_disposal.clone(),
        };
        self.source
            .subscribe(observer)
            .preceded_by(shared_disposal)
            .map_into()
    }
}

struct RetryObserver<OR, F, D: Disposable> {
    observer: OR,
    callback: F,
    shared_disposal: SharedDisposal<Subscription<D>>,
}

impl<'or, T, E, OR, OE1, F> Observer<T, E> for RetryObserver<OR, F, OE1::D>
where
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    F: FnMut(E) -> RetryAction<E, OE1> + MaybeSend + 'or,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => self.observer.on_termination(completion),
            Termination::Error(error) => {
                let action = (self.callback)(error);
                match action {
                    RetryAction::Retry(observable) => {
                        self.shared_disposal
                            .clone()
                            .replace(|| observable.subscribe(self));
                    }
                    RetryAction::Stop(error) => {
                        self.observer.on_termination(Termination::Error(error))
                    }
                }
            }
        }
    }
}
