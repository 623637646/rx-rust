//! Continues with a fallback observable if the source completes empty.
//!
//! The [`SwitchIfEmpty`] operator, behind
//! [`ObservableExt::switch_if_empty`](crate::observable::ObservableExt::switch_if_empty).

use crate::{
    delegate_disposal,
    disposable::{
        Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop,
        replaceable_disposal::ReplaceableDisposal,
    },
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
};
use educe::Educe;

/// Mirrors the source, or continues with a fallback observable if the source completes without
/// emitting any item.
/// See <https://reactivex.io/documentation/operators/defaultifempty.html> (`switchIfEmpty`).
///
/// [`DefaultIfEmpty`](crate::operators::conditional_boolean::default_if_empty::DefaultIfEmpty)
/// emits one value in that case; this subscribes to another observable, which emits whatever it
/// emits: the fallback of a lookup that came back empty, a cache before the network. The fallback
/// is subscribed only then, once the source completed; a source that emits or fails is mirrored
/// to its end and the fallback is never subscribed.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::switch_if_empty::SwitchIfEmpty,
///         creating::{empty::Empty, from_iter::FromIter},
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = SwitchIfEmpty::new(Empty.with_item_type(), FromIter::new([1, 2]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SwitchIfEmpty<OE, OE1> {
    source: OE,
    fallback: OE1,
}

impl<OE, OE1> SwitchIfEmpty<OE, OE1> {
    /// Creates a [`SwitchIfEmpty`] over `source`;
    /// [`ObservableExt::switch_if_empty`](crate::observable::ObservableExt::switch_if_empty) is the
    /// fluent form.
    pub fn new<T, E>(source: OE, fallback: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
    {
        Self { source, fallback }
    }
}

delegate_disposal!(
    Disposal<M, D, D1>,
    ChainDisposal<ReplaceableDisposal<M, D1>, D>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for SwitchIfEmpty<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    /// First the fallback's subscription, if it was made, then the source's.
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for SwitchIfEmpty<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<SwitchIfEmptyObserver<JoinedModeOf<OE, OE1>, OR, OE1>, Item = T, Error = E>,
    OE1: Observable<OR, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let replaceable_disposal = ReplaceableDisposal::default();
        let observer = SwitchIfEmptyObserver {
            observer,
            fallback: self.fallback,
            is_empty: true,
            replaceable_disposal: replaceable_disposal.clone(),
        };
        self.source
            .subscribe(observer)
            .preceded_by(replaceable_disposal)
            .map_inner_into()
    }
}

#[doc(hidden)]
pub struct SwitchIfEmptyObserver<M: ThreadMode, OR, OE1: ObservableTypes> {
    observer: OR,
    fallback: OE1,
    is_empty: bool,
    replaceable_disposal: ReplaceableDisposal<M, OE1::Disposal>,
}

impl<M, T, E, OR, OE1> Observer<T, E> for SwitchIfEmptyObserver<M, OR, OE1>
where
    M: ThreadMode,
    OR: Observer<T, E>,
    OE1: Observable<OR, Item = T, Error = E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.is_empty = false;
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed if self.is_empty => {
                // Skipped once the subscription is disposed: the observer is then dropped with the
                // builder, rather than subscribed after its release.
                self.replaceable_disposal
                    .replace_with(|| self.fallback.subscribe(self.observer));
            }
            termination => self.observer.on_termination(termination),
        }
    }
}
