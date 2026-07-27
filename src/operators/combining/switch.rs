use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::increment_id::IncrementId;
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, ModelUpdate, SubscriptionContext,
    subscribe_with_context_bound_subscription,
};
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
            sub_state: SubState::Idle,
            is_source_completed: false,
            current_sub_id: IncrementId::default(),
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            self.source.subscribe(SwitchObserver(context))
        })
    }
}

enum SubState<D: Disposable> {
    Idle,
    PendingSubscription,
    Processing(Subscription<D>),
}

struct Model<D: Disposable> {
    sub_state: SubState<D>,
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
        let result = self.0.try_update_model(|model| {
            model.current_sub_id.increment();
            match std::mem::replace(&mut model.sub_state, SubState::PendingSubscription) {
                SubState::Idle => ModelUpdate::new(model.current_sub_id).without_drop_outside(),
                SubState::Processing(subscription) => {
                    ModelUpdate::new(model.current_sub_id).with_drop_outside(subscription)
                }
                SubState::PendingSubscription => unreachable!(),
            }
        });
        let sub_id = match result {
            Ok(sub_id) => sub_id,
            Err(_) => return,
        };
        let observer = SwitchInnerObserver(self.0.clone(), sub_id);
        let sub = value.subscribe(observer);
        let _ = self.0.try_update_model(|model| {
            match &mut model.sub_state {
                SubState::Idle => ModelUpdate::empty().with_drop_outside(sub), // already terminated
                SubState::PendingSubscription => {
                    model.sub_state = SubState::Processing(sub);
                    ModelUpdate::empty().without_drop_outside()
                }
                SubState::Processing(_) => unreachable!(),
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.try_update_model(|model| {
                    model.is_source_completed = true;
                    match model.sub_state {
                        SubState::Idle => ModelUpdate::empty().with_termination_event(termination),
                        SubState::Processing(_) | SubState::PendingSubscription => {
                            ModelUpdate::empty().without_events()
                        }
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
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
        let _ = self.0.try_update_model(|model| {
            if model.current_sub_id != self.1 {
                return ModelUpdate::empty().without_events();
            }
            ModelUpdate::empty().with_next_event(value)
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.try_update_model(|model| {
            if model.current_sub_id != self.1 {
                return ModelUpdate::empty().without_events().without_drop_outside();
            }
            match termination {
                Termination::Completed => {
                    if model.is_source_completed {
                        ModelUpdate::empty()
                            .with_termination_event(termination)
                            .without_drop_outside()
                    } else {
                        match std::mem::replace(&mut model.sub_state, SubState::Idle) {
                            SubState::Idle => unreachable!(),
                            SubState::PendingSubscription => {
                                ModelUpdate::empty().without_events().without_drop_outside()
                            }
                            SubState::Processing(subscription) => ModelUpdate::empty()
                                .without_events()
                                .with_drop_outside(subscription),
                        }
                    }
                }
                Termination::Error(_) => ModelUpdate::empty()
                    .with_termination_event(termination)
                    .without_drop_outside(),
            }
        });
    }
}
