//! The [`Merge`] operator, behind
//! [`ObservableExt::merge_with`](crate::observable::ObservableExt::merge_with).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    disposable::Disposable,
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Combines multiple Observables into a single Observable that emits all of their emissions.
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
/// let mut terminations = Vec::new();
///
/// let observable = Merge::new(
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
///
/// The mode is the two sources' joined ([`Joined`]): a merge of
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
    type Mode = Joined<OE1::Mode, OE2::Mode>;
    type Disposal = Disposal<Joined<OE1::Mode, OE2::Mode>, T, E, OE1::Disposal, OE2::Disposal>;
}

impl<T, E, OE1, OE2, OR> Observable<OR> for Merge<OE1, OE2>
where
    OR: Observer<T, E>,
    OE1: Observable<
            MergeObserver<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE2 as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T,
            Error = E,
        >,
    OE2: Observable<
            MergeObserver<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<
                    <OE2 as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let model = Model {
            one_is_completed: false,
        };
        subscribe_with_context(observer, model, |context| {
            let subscription_1 = self.source_1.subscribe(MergeObserver(context.clone()));
            let subscription_2 = self.source_2.subscribe(MergeObserver(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
        .map_into()
    }
}

struct Model {
    one_is_completed: bool,
}

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
