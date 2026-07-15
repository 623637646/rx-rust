use crate::delegate_disposal;
use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::increment_id::IncrementId;
use crate::utils::subscribe_with_context::{
    self, Context, ModificationResult, subscribe_with_context,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{
        subscribe_with_auto_dispose_on_termination::{
            self, subscribe_with_auto_dispose_on_termination,
        },
        types::MarkerType,
    },
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

delegate_disposal!(
    Disposal<'or, D>,
    subscribe_with_auto_dispose_on_termination::Disposal<subscribe_with_context::Disposal<'or, D>>,
    where D: Disposable
);

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for Switch<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<'or, OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let model = Model {
                sub_state: SubState::Idle,
                is_source_completed: false,
                current_sub_id: IncrementId::default(),
            };
            subscribe_with_context(observer, model, |context| {
                self.source.subscribe(SwitchObserver(context))
            })
        })
        .map_into()
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

struct SwitchObserver<T, E, OR, D: Disposable>(Context<T, E, OR, Model<D>>);

impl<'or, T, E, OR, OE1> Observer<OE1, E> for SwitchObserver<T, E, OR, OE1::D>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        let result = self.0.modify_model(|model| {
            model.current_sub_id.increment();
            match std::mem::replace(&mut model.sub_state, SubState::PendingSubscription) {
                SubState::Idle => ModificationResult::new(model.current_sub_id),
                SubState::Processing(subscription) => {
                    ModificationResult::new(model.current_sub_id).drop_outside(subscription)
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
        let _ = self.0.modify_model(|model| {
            match &mut model.sub_state {
                SubState::Idle => ModificationResult::new_without_result().drop_outside(sub), // already terminated
                SubState::PendingSubscription => {
                    model.sub_state = SubState::Processing(sub);
                    ModificationResult::new_without_result()
                }
                SubState::Processing(_) => unreachable!(),
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|model| {
                    model.is_source_completed = true;
                    match model.sub_state {
                        SubState::Idle => ModificationResult::new_send_termination(termination),
                        SubState::Processing(_) | SubState::PendingSubscription => {
                            ModificationResult::new_without_result()
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
struct SwitchInnerObserver<T, E, OR, D: Disposable>(Context<T, E, OR, Model<D>>, IncrementId);

impl<T, E, OR, D> Observer<T, E> for SwitchInnerObserver<T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.modify_model(|model| {
            if model.current_sub_id != self.1 {
                return ModificationResult::new_without_result();
            }
            ModificationResult::new_send_next(value)
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.modify_model(|model| {
            if model.current_sub_id != self.1 {
                return ModificationResult::new_without_result();
            }
            match termination {
                Termination::Completed => {
                    if model.is_source_completed {
                        ModificationResult::new_without_result().send_termination(termination)
                    } else {
                        match std::mem::replace(&mut model.sub_state, SubState::Idle) {
                            SubState::Idle => unreachable!(),
                            SubState::PendingSubscription => {
                                ModificationResult::new_without_result()
                            }
                            SubState::Processing(subscription) => {
                                ModificationResult::new_without_result().drop_outside(subscription)
                            }
                        }
                    }
                }
                Termination::Error(_) => {
                    ModificationResult::new_without_result().send_termination(termination)
                }
            }
        });
    }
}
