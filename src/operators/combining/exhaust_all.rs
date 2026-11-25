//! Emits the items of one inner observable at a time, ignoring those that arrive meanwhile.
//!
//! The [`ExhaustAll`] operator, behind
//! [`ObservableExt::exhaust_all`](crate::observable::ObservableExt::exhaust_all).

use super::internal::flatten::{
    self, ContextDisposal, Identity, Ignore, InnerObserverOf, SourceObserverOf,
};
use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::Observer,
    thread_mode::ThreadMode,
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, num::NonZeroUsize};

/// Flattens an observable of observables by emitting the items of one inner observable at a time,
/// ignoring every inner observable that arrives while one is still running.
/// See <https://reactivex.io/documentation/operators/flatmap.html> (`exhaust`).
///
/// The counterpart of [`Switch`](crate::operators::combining::switch::Switch): where that one
/// drops the running inner observable for the newest, this keeps the running one and drops the
/// newest, without ever subscribing to it. So a burst of requests — repeated clicks on a submit
/// button, a refresh asked for while one is under way — starts one piece of work, and the next
/// one starts only once it is over.
///
/// The stream completes once the source has completed and the running inner observable, if any,
/// has completed too; an error of either ends it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::exhaust_all::ExhaustAll,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::<_, Infallible, _>::local();
/// let mut inner_1 = PublishSubject::local();
/// let mut inner_2 = PublishSubject::local();
///
/// let subscription = ExhaustAll::new(source.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(inner_1.clone());
/// let _ = source.on_next(inner_2.clone()); // Ignored: `inner_1` is still running.
/// let _ = inner_1.on_next(1);
/// let _ = inner_2.on_next(2);
/// inner_1.on_termination(Termination::Completed);
/// let _ = source.on_next(inner_2.clone()); // Subscribed: nothing is running.
/// let _ = inner_2.on_next(3);
///
/// drop((subscription, source, inner_2));
/// assert_eq!(values, [1, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ExhaustAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ExhaustAll<OE, OE1> {
    /// Creates an [`ExhaustAll`] over `source`;
    /// [`ObservableExt::exhaust_all`](crate::observable::ObservableExt::exhaust_all) is the fluent
    /// form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

/// The observer an [`ExhaustAll`] or an operator on it gives its source, with the selector `F`
/// that turns each item into an inner observable: the `Identity` of `ExhaustAll` itself, a user
/// callback for [`ExhaustMap`](crate::operators::transforming::exhaust_map::ExhaustMap).
pub(crate) type ExhaustAllObserverOf<OE, OE1, T, E, OR, F> =
    SourceObserverOf<OE, OE1, T, E, OR, Ignore, F>;

/// The observer an [`ExhaustAll`] gives its source, whose items are the inner observables
/// themselves.
pub(crate) type ExhaustAllObserver<OE, OE1, T, E, OR> =
    ExhaustAllObserverOf<OE, OE1, T, E, OR, Identity>;

/// The observer an [`ExhaustAll`] or an operator on it gives each inner observable.
pub(crate) type ExhaustAllInnerObserver<OE, OE1, T, E, OR> =
    InnerObserverOf<OE, OE1, T, E, OR, Ignore>;

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    ContextDisposal<M, T, E, D, Ignore, D1>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for ExhaustAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for ExhaustAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<ExhaustAllObserver<OE, OE1, T, E, OR>, Item = OE1, Error = E>,
    OE1: Observable<ExhaustAllInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Ignore,
            Identity,
        )
        .map_inner_into()
    }
}
