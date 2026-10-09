//! The [`Retry`] operator, behind
//! [`ObservableExt::retry`](crate::observable::ObservableExt::retry).

use crate::delegate_disposal;
use crate::disposable::dispose_on_drop::DisposeOnDrop;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, replaceable_disposal::ReplaceableDisposal,
};
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::resubscribe::Resubscribe;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
/// What the callback of [`Retry`] decides about an error.
pub enum RetryAction<E, OE1> {
    /// Subscribe to this observable and keep going.
    Retry(OE1),
    /// Give up and terminate downstream with this error.
    Stop(E),
}

/// Re-subscribes after an error, to the Observable the callback returns, until the callback
/// gives up with an error of its own.
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
    /// Creates a [`Retry`] over `source`;
    /// [`ObservableExt::retry`](crate::observable::ObservableExt::retry) is the fluent form.
    pub fn new<T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
        F: FnMut(E) -> RetryAction<E, OE1>,
    {
        Self { source, callback }
    }
}

delegate_disposal!(
    Disposal<M, D, D1>,
    ChainDisposal<ReplaceableDisposal<M, DisposeOnDrop<D1>>, D>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1, F> ObservableTypes for Retry<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type Disposal = Disposal<Joined<OE::Mode, OE1::Mode>, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, F, OR> Observable<OR> for Retry<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            RetryObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                OR,
                F,
                OE1,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            RetryObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                OR,
                F,
                OE1,
            >,
            Item = T,
            Error = E,
        >,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let replaceable_disposal = ReplaceableDisposal::default();
        let observer = RetryObserver {
            observer,
            callback: self.callback,
            replaceable_disposal: replaceable_disposal.clone(),
            resubscribe: Resubscribe::new(),
        };
        self.source
            .subscribe(observer)
            .preceded_by(replaceable_disposal)
            .map_inner_into()
    }
}

pub struct RetryObserver<M: ThreadMode, OR, F, OE1: ObservableTypes> {
    observer: OR,
    callback: F,
    replaceable_disposal: ReplaceableDisposal<M, DisposeOnDrop<OE1::Disposal>>,
    /// Subscribes the observable the callback returned, with this observer.
    resubscribe: Resubscribe<OE1, Self>,
}

impl<M, T, E, OR, OE1, F> Observer<T, E> for RetryObserver<M, OR, F, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnMut(E) -> RetryAction<E, OE1>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => self.observer.on_termination(completion),
            Termination::Error(error) => {
                let action = (self.callback)(error);
                match action {
                    RetryAction::Retry(observable) => {
                        let resubscribe = self.resubscribe;
                        self.replaceable_disposal
                            .clone()
                            .replace_with(|| resubscribe.subscribe(observable, self));
                    }
                    RetryAction::Stop(error) => {
                        self.observer.on_termination(Termination::Error(error))
                    }
                }
            }
        }
    }
}
