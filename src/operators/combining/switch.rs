use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::increment_id::IncrementId;
use crate::utils::serialized_delivery::UpdateOutcome;
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
use std::marker::PhantomData;

/// Converts an Observable that emits Observables into a single Observable that emits the items emitted by the most recently emitted of those Observables.
/// See <https://reactivex.io/documentation/operators/switch.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::switch::Switch,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let inner_1 = FromIter::new(vec![1, 2]);
/// let inner_2 = FromIter::new(vec![3, 4]);
/// let observable = Switch::new_from_iter([inner_1, inner_2]);
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
pub struct Switch<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Switch<OE, OE1> {
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

impl<E, OE1, I> Switch<WithErrorType<E, FromIter<I>>, OE1> {
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

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for Switch<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            slot: SubscriptionSlot::Idle,
            is_source_completed: false,
            current_sub_id: IncrementId::default(),
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            self.source.subscribe(SwitchObserver(context))
        })
    }
}

struct Model<D: Disposable> {
    slot: SubscriptionSlot<Subscription<D>>,
    is_source_completed: bool,
    current_sub_id: IncrementId,
}

struct SwitchObserver<T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<T, E, OR, Model<ID>, SD>,
);

impl<'or, T, E, OR, OE1, SD> Observer<OE1, E> for SwitchObserver<T, E, OR, OE1::D, SD>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        let result = self.0.update(|model| {
            model.current_sub_id.increment();
            UpdateOutcome::new(model.current_sub_id).with_drop_outside(model.slot.reserve())
        });
        let sub_id = match result {
            Ok(sub_id) => sub_id,
            Err(_) => return,
        };
        let observer = SwitchInnerObserver(self.0.clone(), sub_id);
        let sub = value.subscribe(observer);
        // `fill` gives the subscription back when the slot was released while it was being built,
        // which means the operator already terminated.
        let _ = self
            .0
            .update(|model| UpdateOutcome::empty().with_drop_outside(model.slot.fill(sub)));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|model| {
                    model.is_source_completed = true;
                    if model.slot.is_idle() {
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

// TODO: Improve performance, if is_source_completed is true, stop the context and no more need to lock when sending next.
// TODO: check all cases using subscribe_with_context whether it can be improved.
struct SwitchInnerObserver<T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<T, E, OR, Model<ID>, SD>,
    IncrementId,
);

impl<T, E, OR, ID, SD> Observer<T, E> for SwitchInnerObserver<T, E, OR, ID, SD>
where
    OR: Observer<T, E>,
    ID: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.update(|model| {
            if model.current_sub_id != self.1 {
                return UpdateOutcome::empty().without_events();
            }
            UpdateOutcome::empty().with_next_event(value)
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| {
            if model.current_sub_id != self.1 {
                return UpdateOutcome::empty()
                    .without_events()
                    .without_drop_outside();
            }
            match termination {
                completion @ Termination::Completed => {
                    if model.is_source_completed {
                        UpdateOutcome::empty()
                            .with_termination_event(completion)
                            .without_drop_outside()
                    } else {
                        assert!(
                            !model.slot.is_idle(),
                            "the terminating inner subscription is still held or reserved"
                        );
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside(model.slot.release())
                    }
                }
                error @ Termination::Error(_) => UpdateOutcome::empty()
                    .with_termination_event(error)
                    .without_drop_outside(),
            }
        });
    }
}
