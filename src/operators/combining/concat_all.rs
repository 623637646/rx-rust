use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::serialized_delivery::{DeliveryStopped, UpdateOutcome};
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, SubscriptionContext, subscribe_with_context_bound_subscription,
};
use crate::utils::subscription_slot::SubscriptionSlot;
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
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
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            pending_observables: VecDeque::new(),
            slot: SubscriptionSlot::Idle,
            is_source_completed: false,
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            self.source.subscribe(SourceObserver(context.clone()))
        })
    }
}

struct Model<'or, T, E, OE1>
where
    OE1: Observable<'or, T, E>,
{
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
    fn on_next(&mut self, value: OE1) {
        let result = self.0.update_model_and_send(|model| {
            if model.slot.reserve_if_idle() {
                UpdateOutcome::new(Some(value))
            } else {
                model.pending_observables.push_back(value);
                UpdateOutcome::new(None)
            }
        });
        let observable = match result {
            Ok(Some(observable)) => observable,
            Ok(None) => return,
            Err(DeliveryStopped) => return,
        };
        let observer = InnerObserver(self.0.clone());
        let sub = observable.subscribe(observer);
        // `fill` gives the subscription back when the slot was released while it was being built,
        // which means the operator already terminated.
        let _ = self.0.update_model_and_send(|model| {
            UpdateOutcome::empty().with_drop_outside(model.slot.fill(sub))
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update_model_and_send(|model| {
                    model.is_source_completed = true;
                    if model.slot.is_idle() {
                        // The slot is only possible to be reserved or active when the pending_observables is not empty.
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
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => subscribe_next_observable_until_finished(self.0.clone()),
            error @ Termination::Error(_) => {
                self.0.send_termination(error);
            }
        }
    }
}

fn subscribe_next_observable_until_finished<'or, T, E, OR, OE1, SD>(
    context: SubscriptionContext<T, E, OR, Model<'or, T, E, OE1>, SD>,
) where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E> + MaybeSend + 'or,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    loop {
        let result = context.update_model_and_send(|model| {
            if model.slot.is_reserved() {
                // Already terminated. A reserved slot holds nothing, so nothing is dropped here;
                // releasing it makes the pending fill give its subscription back.
                let released = model.slot.release();
                debug_assert!(released.is_none());
                return UpdateOutcome::new(None)
                    .without_events()
                    .without_drop_outside();
            }
            if let Some(observable) = model.pending_observables.pop_front() {
                UpdateOutcome::new(Some(observable))
                    .without_events()
                    .with_drop_outside(model.slot.reserve())
            } else if model.is_source_completed {
                UpdateOutcome::new(None)
                    .with_termination_event(Termination::Completed)
                    .without_drop_outside()
            } else {
                UpdateOutcome::new(None)
                    .without_events()
                    .with_drop_outside(model.slot.release())
            }
        });
        let observable = match result {
            Ok(Some(observable)) => observable,
            Ok(None) => break,
            Err(DeliveryStopped) => {
                break;
            }
        };
        let observer = InnerObserver(context.clone());
        let sub = observable.subscribe(observer);
        let result = context.update_model_and_send(|model| {
            // `fill` gives the subscription back when the slot was released while it was being
            // built, which means the inner observable already terminated: the loop then goes on
            // to the next pending observable instead of waiting for this subscription.
            let unused = model.slot.fill(sub);
            UpdateOutcome::new(unused.is_none()).with_drop_outside(unused)
        });
        match result {
            Ok(subscribed) => {
                if subscribed {
                    break;
                }
            }
            Err(DeliveryStopped) => break,
        }
    }
}
