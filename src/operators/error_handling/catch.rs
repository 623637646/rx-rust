//! The [`Catch`] operator, behind
//! [`ObservableExt::catch`](crate::observable::ObservableExt::catch).

use crate::delegate_disposal;
use crate::disposable::{
    Disposable, chain_disposal::ChainDisposal, shared_disposal::SharedDisposal,
};
use crate::observable::Subscription;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::{
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Recovers from an error by continuing with the Observable the callback returns for it.
/// See <https://reactivex.io/documentation/operators/catch.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::{just::Just, throw::Throw},
///         error_handling::catch::Catch,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Catch::new(Throw::new("boom").with_item_type(), |error| Just::new(error));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec!["boom"]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Catch<E0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<E0>,
}

impl<E0, OE, F> Catch<E0, OE, F> {
    /// Creates a [`Catch`] over `source`;
    /// [`ObservableExt::catch`](crate::observable::ObservableExt::catch) is the fluent form.
    pub fn new<T, E, OE1>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E0>,
        OE1: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(E0) -> OE1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<M, D, D1>,
    ChainDisposal<SharedDisposal<M, Subscription<D1>>, D>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E0, E, OE, OE1, F> ObservableTypes for Catch<E0, OE, F>
where
    OE: ObservableTypes<Item = T, Error = E0>,
    OE1: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(E0) -> OE1,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type Disposal = Disposal<Joined<OE::Mode, OE1::Mode>, OE::Disposal, OE1::Disposal>;
}

impl<T, E0, E, OE, OE1, F, OR> Observable<OR> for Catch<E0, OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<
            CatchObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                E,
                OR,
                F,
                <OE1 as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E0,
        >,
    OE1: Observable<OR, Item = T, Error = E>,
    F: FnOnce(E0) -> OE1,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let shared_disposal = SharedDisposal::default();
        let observer = CatchObserver {
            observer,
            callback: self.callback,
            shared_disposal: shared_disposal.clone(),
            _marker: PhantomData,
        };
        self.source
            .subscribe(observer)
            .preceded_by(shared_disposal)
            .map_into()
    }
}

pub struct CatchObserver<M: ThreadMode, E, OR, F, D: Disposable> {
    observer: OR,
    callback: F,
    shared_disposal: SharedDisposal<M, Subscription<D>>,
    _marker: MarkerType<E>,
}

impl<M, T, E0, E, OR, OE1, F> Observer<T, E0> for CatchObserver<M, E, OR, F, OE1::Disposal>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    OE1: Observable<OR, Item = T, Error = E>,
    F: FnOnce(E0) -> OE1,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E0>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => {
                self.shared_disposal.replace(|| {
                    let observable = (self.callback)(error);
                    observable.subscribe(self.observer)
                });
            }
        }
    }
}
