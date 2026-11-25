//! Emits the items of the most recent inner observable only.
//!
//! The [`Switch`] operator, behind
//! [`ObservableExt::switch`](crate::observable::ObservableExt::switch).

use super::internal::flatten::{
    self, ContextDisposal, Identity, InnerObserverOf, Replace, SourceObserverOf,
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

/// Flattens an observable of observables by emitting the items of the most recently emitted inner
/// observable only.
/// See <https://reactivex.io/documentation/operators/switch.html>
///
/// An inner observable that arrives while another is running takes its place: the running one is
/// unsubscribed from, and whatever it still sends is dropped. So of a burst of requests — the
/// searches of someone typing — only the newest one is still wanted, and the work of the others
/// is cancelled. [`ExhaustAll`](crate::operators::combining::exhaust_all::ExhaustAll) is the
/// counterpart that keeps the running one and drops the newest.
///
/// The stream completes once the source has completed and the running inner observable, if any,
/// has completed too; an error of either ends it, while that of an inner observable that was
/// replaced is ignored.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::switch::Switch,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::<_, Infallible, _>::local();
/// let mut inner_1 = PublishSubject::local();
/// let mut inner_2 = PublishSubject::local();
///
/// let subscription = Switch::new(source.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(inner_1.clone());
/// let _ = inner_1.on_next(1);
/// let _ = source.on_next(inner_2.clone()); // `inner_1` is unsubscribed from.
/// let _ = inner_1.on_next(2);
/// let _ = inner_2.on_next(3);
///
/// drop((subscription, source, inner_1, inner_2));
/// assert_eq!(values, [1, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Switch<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Switch<OE, OE1> {
    /// Creates a [`Switch`] over `source`;
    /// [`ObservableExt::switch`](crate::observable::ObservableExt::switch) is the fluent
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

/// The observer a [`Switch`] or an operator on it gives its source, with the selector `F` that
/// turns each item into an inner observable: the `Identity` of `Switch` itself, a user callback
/// for [`SwitchMap`](crate::operators::transforming::switch_map::SwitchMap).
pub(crate) type SwitchObserverOf<OE, OE1, T, E, OR, F> =
    SourceObserverOf<OE, OE1, T, E, OR, Replace, F>;

/// The observer a [`Switch`] gives its source, whose items are the inner observables themselves.
pub(crate) type SwitchObserver<OE, OE1, T, E, OR> = SwitchObserverOf<OE, OE1, T, E, OR, Identity>;

/// The observer a [`Switch`] or an operator on it gives each inner observable.
pub(crate) type SwitchInnerObserver<OE, OE1, T, E, OR> =
    InnerObserverOf<OE, OE1, T, E, OR, Replace>;

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    ContextDisposal<M, T, E, D, Replace, D1>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for Switch<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Switch<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<SwitchObserver<OE, OE1, T, E, OR>, Item = OE1, Error = E>,
    OE1: Observable<SwitchInnerObserver<OE, OE1, T, E, OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        // One inner observable at a time.
        flatten::subscribe(
            self.source,
            |_| observer,
            NonZeroUsize::MIN,
            Replace,
            Identity,
        )
        .map_inner_into()
    }
}
