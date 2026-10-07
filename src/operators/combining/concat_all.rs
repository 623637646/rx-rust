//! The [`ConcatAll`] operator, behind
//! [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all).

use crate::delegate_disposal;
use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::resubscribe::Resubscribe;
use crate::utils::serialized_delivery::{DeliveryStopped, DropDecided, UpdateOutcome};
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::utils::subscription_slot::SubscriptionSlot;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::MarkerType,
};
use educe::Educe;
use std::{collections::VecDeque, marker::PhantomData};

/// Flattens an Observable of Observables by emitting all of the values of each inner Observable,
/// one inner Observable after the other.
/// See <https://reactivex.io/documentation/operators/concat.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::concat_all::ConcatAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = ConcatAll::new_from_iter([
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![3, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatAll<OE, OE1> {
    /// Creates a [`ConcatAll`] over `source`;
    /// [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all) is the fluent
    /// form.
    pub fn new<T, E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> ConcatAll<WithErrorType<E, FromIter<I>>, OE1> {
    /// Creates a [`ConcatAll`] over the observables of `into_iterator`.
    pub fn new_from_iter<T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, OE1>,
    subscribe_with_context::Disposal<M, T, E, Model<OE1>, D>,
    where M: ThreadMode, D: Disposable, OE1: ObservableTypes
);

impl<T, E, OE, OE1> ObservableTypes for ConcatAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type Disposal = Disposal<Joined<OE::Mode, OE1::Mode>, T, E, OE::Disposal, OE1>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for ConcatAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SourceObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                OE1,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = OE1,
            Error = E,
        >,
    OE1: Observable<
            InnerObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                OE1,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let model = Model {
            pending_observables: VecDeque::new(),
            slot: SubscriptionSlot::Idle,
            is_source_completed: false,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(SourceObserver {
                context: context.clone(),
                subscribe_inner: Resubscribe::new(),
            })
        })
        .map_into()
    }
}

struct Model<OE1>
where
    OE1: ObservableTypes,
{
    /// Values wait here while an inner is active or its subscription is still being built.
    /// An idle slot always has an empty queue.
    pending_observables: VecDeque<OE1>,
    slot: SubscriptionSlot<Subscription<OE1::Disposal>>,
    is_source_completed: bool,
}

pub struct SourceObserver<M: ThreadMode, T, E, OR, OE1, SD>
where
    OE1: ObservableTypes<Item = T, Error = E>,
    SD: Disposable,
{
    context: SubscriptionContext<M, T, E, OR, Model<OE1>, SD>,
    /// Subscribes an inner observable with an [`InnerObserver`].
    subscribe_inner: Resubscribe<OE1, InnerObserver<M, T, E, OR, OE1, SD>>,
}

impl<M: ThreadMode, T, E, OR, OE1, SD> Observer<OE1, E> for SourceObserver<M, T, E, OR, OE1, SD>
where
    OR: Observer<T, E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    SD: Disposable,
{
    fn on_next(&mut self, value: OE1) -> Flow {
        let result = self.context.update(|model| {
            if model.slot.reserve_if_idle() {
                UpdateOutcome::new(Some(value))
            } else {
                model.pending_observables.push_back(value);
                UpdateOutcome::new(None)
            }
        });
        match result {
            Ok(Some(observable)) => {
                match subscribe_observables(&self.context, observable, self.subscribe_inner) {
                    Ok(()) => Flow::Continue,
                    Err(DeliveryStopped) => Flow::Stop,
                }
            }
            Ok(None) => Flow::Continue,
            Err(DeliveryStopped) => Flow::Stop,
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update(|model| {
                    model.is_source_completed = true;
                    if model.slot.is_idle() {
                        // An idle slot has no inner subscription or queued work left.
                        debug_assert!(model.pending_observables.is_empty());
                        UpdateOutcome::empty().with_termination_event(completion)
                    } else {
                        UpdateOutcome::empty().without_events()
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}

pub struct InnerObserver<M: ThreadMode, T, E, OR, OE1, SD>
where
    OE1: ObservableTypes<Item = T, Error = E>,
    SD: Disposable,
{
    context: SubscriptionContext<M, T, E, OR, Model<OE1>, SD>,
    /// Subscribes the next inner observable with another inner observer.
    subscribe_inner: Resubscribe<OE1, Self>,
}

impl<M: ThreadMode, T, E, OR, OE1, SD> Observer<T, E> for InnerObserver<M, T, E, OR, OE1, SD>
where
    OR: Observer<T, E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.context.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let next = self.context.update(|model| {
                    let finished = model.slot.release();
                    next_step(model, finished)
                });
                if let Ok(Some(observable)) = next {
                    let _ = subscribe_observables(&self.context, observable, self.subscribe_inner);
                }
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}

type NextStep<T, E, OE, D> =
    UpdateOutcome<T, E, Option<OE>, DropDecided<Option<Subscription<D>>>, true>;

/// `fill` and `release` hand back the finished subscription to whichever runs second. Only
/// that caller advances the queue; the first leaves the slot occupied for the other to finish.
/// This runs under the same lock as the handoff, so taking the next observable and reserving its
/// slot cannot race a source emission. The returned subscription is dropped outside the lock.
fn next_step<T, E, OE1>(
    model: &mut Model<OE1>,
    finished: Option<Subscription<OE1::Disposal>>,
) -> NextStep<T, E, OE1, OE1::Disposal>
where
    OE1: ObservableTypes<Item = T, Error = E>,
{
    let outcome = if finished.is_none() {
        UpdateOutcome::new(None).without_events()
    } else if let Some(observable) = model.pending_observables.pop_front() {
        // A successful handoff made the slot idle under this same lock.
        let _ = model.slot.reserve_if_idle();
        UpdateOutcome::new(Some(observable)).without_events()
    } else if model.is_source_completed {
        UpdateOutcome::new(None).with_termination_event(Termination::Completed)
    } else {
        UpdateOutcome::new(None).without_events()
    };
    outcome.with_drop_outside(finished)
}

/// Subscribes to the already reserved observable, then loops over any successors that complete
/// before their subscription is filled. An inner that stays active hands the continuation to its
/// completion callback instead, so immediately completing chains never recurse.
fn subscribe_observables<M: ThreadMode, T, E, OR, OE1, SD>(
    context: &SubscriptionContext<M, T, E, OR, Model<OE1>, SD>,
    mut observable: OE1,
    subscribe_inner: Resubscribe<OE1, InnerObserver<M, T, E, OR, OE1, SD>>,
) -> Result<(), DeliveryStopped>
where
    OR: Observer<T, E>,
    OE1: ObservableTypes<Item = T, Error = E>,
    SD: Disposable,
{
    loop {
        let subscription = subscribe_inner.subscribe(
            observable,
            InnerObserver {
                context: context.clone(),
                subscribe_inner,
            },
        );
        let next = context.update(|model| {
            let finished = model.slot.fill(subscription);
            next_step(model, finished)
        })?;
        match next {
            Some(next) => observable = next,
            None => return Ok(()),
        }
    }
}
