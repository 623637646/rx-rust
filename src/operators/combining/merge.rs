//! The [`Merge`] operator, behind
//! [`ObservableExt::merge_with`](crate::observable::ObservableExt::merge_with).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;

/// Combines multiple observables into a single observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge::Merge,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let observable = Merge::new(
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// ```
///
/// The mode is the two sources' joined ([`JoinedModeOf`]): a merge of
/// `Local` sources keeps its state in an `Rc` and takes an observer holding one,
///
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, convert::Infallible, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .merge_with(PublishSubject::<_, Infallible, _>::local())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
///
/// while a `Shared` source makes the whole merge `Shared`, and its observer must be `Send`:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::ObservableExt, operators::creating::just::Just,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{cell::RefCell, convert::Infallible, rc::Rc};
///
/// let values = Rc::new(RefCell::new(Vec::new()));
/// let _subscription = Just::new(1)
///     .merge_with(PublishSubject::<_, Infallible, _>::shared())
///     .subscribe_with_callback(move |value| values.borrow_mut().push(value), |_| {});
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Merge<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Merge<OE1, OE2> {
    /// Creates a [`Merge`] over `source_1` and `source_2`;
    /// [`ObservableExt::merge_with`](crate::observable::ObservableExt::merge_with) is the fluent
    /// form.
    pub fn new<T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: ObservableTypes<Item = T, Error = E>,
        OE2: ObservableTypes<Item = T, Error = E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D1, D2>,
    subscribe_with_context::Disposal<M, T, E, Model, ChainDisposal<D2, D1>>,
    where M: ThreadMode, D1: Disposable, D2: Disposable
);

impl<T, E, OE1, OE2> ObservableTypes for Merge<OE1, OE2>
where
    OE1: ObservableTypes<Item = T, Error = E>,
    OE2: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE1, OE2>;
    type Disposal = Disposal<JoinedModeOf<OE1, OE2>, T, E, OE1::Disposal, OE2::Disposal>;
}

impl<T, E, OE1, OE2, OR> Observable<OR> for Merge<OE1, OE2>
where
    OR: Observer<T, E>,
    OE1: Observable<
            MergeObserver<JoinedModeOf<OE1, OE2>, T, E, OR, ChainDisposalOf<OE2, OE1>>,
            Item = T,
            Error = E,
        >,
    OE2: Observable<
            MergeObserver<JoinedModeOf<OE1, OE2>, T, E, OR, ChainDisposalOf<OE2, OE1>>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            one_is_completed: false,
        };
        subscribe_with_context(observer, model, |context| {
            let subscription_1 = self.source_1.subscribe(MergeObserver(context.clone()));
            let subscription_2 = self.source_2.subscribe(MergeObserver(context));
            subscription_1.preceded_by_wrapped(subscription_2)
        })
        .map_inner_into()
    }
}

struct Model {
    one_is_completed: bool,
}

#[doc(hidden)]
pub struct MergeObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<T, E> for MergeObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|model| {
                    if model.one_is_completed {
                        UpdateOutcome::empty().with_termination_event(completion)
                    } else {
                        model.one_is_completed = true;
                        UpdateOutcome::empty().without_events()
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.0.send_termination(error);
            }
        }
    }
}
