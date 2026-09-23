//! The [`ConcatAll`] operator, behind
//! [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all).

use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::serialized_delivery::{DeliveryStopped, DropDecided, UpdateOutcome};
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, subscribe_with_context_owning_source,
};
use crate::utils::subscription_slot::SubscriptionSlot;
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::types::MarkerType,
};
use educe::Educe;
use std::{collections::VecDeque, marker::PhantomData};

/// Concatenates an Observable of Observables, emitting all values from each inner Observable in sequence.
/// See <https://reactivex.io/documentation/operators/concat.html> (referencing concat operator for general concept)
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
    /// [`ObservableExt::concat_all`](crate::observable::ObservableExt::concat_all) is the fluent form.
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, OE1, E>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> ConcatAll<WithErrorType<E, FromIter<I>>, OE1> {
    /// Creates a [`ConcatAll`] over the observables of `into_iterator`.
    pub fn new_from_iter<'or, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for ConcatAll<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E> + MaybeSend + 'or,
    OE1::D: MaybeSend + 'or,
{
    type D = subscribe_with_context::OwningDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            pending_observables: VecDeque::new(),
            slot: SubscriptionSlot::Idle,
            is_source_completed: false,
        };
        subscribe_with_context_owning_source(observer, model, |context| {
            self.source.subscribe(SourceObserver(context.clone()))
        })
    }
}

struct Model<'or, T, E, OE1>
where
    OE1: Observable<'or, T, E>,
{
    /// Values wait here while an inner is active or its subscription is still being built.
    /// An idle slot always has an empty queue.
    pending_observables: VecDeque<OE1>,
    slot: SubscriptionSlot<Subscription<OE1::D>>,
    is_source_completed: bool,
}

struct SourceObserver<'or, T, E, OR, OE1, SD>(
    SubscriptionContext<T, E, OR, Model<'or, T, E, OE1>, SD>,
)
where
    OE1: Observable<'or, T, E>,
    SD: Disposable;

impl<'or, T, E, OR, OE1, SD> Observer<OE1, E> for SourceObserver<'or, T, E, OR, OE1, SD>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E> + MaybeSend + 'or,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    fn on_next(&mut self, value: OE1) -> Flow {
        let result = self.0.update(|model| {
            if model.slot.reserve_if_idle() {
                UpdateOutcome::new(Some(value))
            } else {
                model.pending_observables.push_back(value);
                UpdateOutcome::new(None)
            }
        });
        match result {
            Ok(Some(observable)) => match subscribe_observables(&self.0, observable) {
                Ok(()) => Flow::Continue,
                Err(DeliveryStopped) => Flow::Stop,
            },
            Ok(None) => Flow::Continue,
            Err(DeliveryStopped) => Flow::Stop,
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|model| {
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
                self.0.send_termination(error);
            }
        }
    }
}

struct InnerObserver<'or, T, E, OR, OE1, SD>(
    SubscriptionContext<T, E, OR, Model<'or, T, E, OE1>, SD>,
)
where
    OE1: Observable<'or, T, E>,
    SD: Disposable;

impl<'or, T, E, OR, OE1, SD> Observer<T, E> for InnerObserver<'or, T, E, OR, OE1, SD>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E> + MaybeSend + 'or,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let next = self.0.update(|model| {
                    let finished = model.slot.release();
                    next_step(model, finished)
                });
                if let Ok(Some(observable)) = next {
                    let _ = subscribe_observables(&self.0, observable);
                }
            }
            error @ Termination::Error(_) => {
                self.0.send_termination(error);
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
fn next_step<'or, T, E, OE1>(
    model: &mut Model<'or, T, E, OE1>,
    finished: Option<Subscription<OE1::D>>,
) -> NextStep<T, E, OE1, OE1::D>
where
    OE1: Observable<'or, T, E>,
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
fn subscribe_observables<'or, T, E, OR, OE1, SD>(
    context: &SubscriptionContext<T, E, OR, Model<'or, T, E, OE1>, SD>,
    mut observable: OE1,
) -> Result<(), DeliveryStopped>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E> + MaybeSend + 'or,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    loop {
        let subscription = observable.subscribe(InnerObserver(context.clone()));
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
