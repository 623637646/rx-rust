use crate::disposable::subscription::Subscription;
use crate::observable::observable_ext::ObservableExt;
use crate::operators::others::map_infallible_to_error::MapInfallibleToError;
use crate::utils::increment_id::IncrementId;
use crate::utils::subscribe_with_shared_model::{
    Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{
        subscribe_unsub_after_termination::subscribe_unsub_after_termination, types::MarkerType,
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
///     observable::observable_ext::ObservableExt,
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
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> Switch<MapInfallibleToError<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, 'sub, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator).map_infallible_to_error(),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for Switch<OE, OE1>
where
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'sub,
    E: NecessarySend + 'sub,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                sub_state: SubState::Idle,
                is_source_completed: false,
                current_sub_id: IncrementId::default(),
            };
            subscribe_with_shared_model(observer, model, |context| {
                self.source.subscribe(SwitchObserver(context))
            })
        })
    }
}

enum SubState<'sub> {
    Idle,
    PendingSubscription,
    Processing(Subscription<'sub>),
}

struct Model<'sub> {
    sub_state: SubState<'sub>,
    is_source_completed: bool,
    current_sub_id: IncrementId,
}

struct SwitchObserver<'sub, T, E, OR>(Context<T, E, OR, Model<'sub>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for SwitchObserver<'sub, T, E, OR>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
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
                    let _ = std::mem::replace(&mut model.sub_state, SubState::Processing(sub));
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
// TODO: check all cases using subscribe_with_shared_model whether it can be improved.
struct SwitchInnerObserver<'sub, T, E, OR>(Context<T, E, OR, Model<'sub>>, IncrementId);

impl<T, E, OR> Observer<T, E> for SwitchInnerObserver<'_, T, E, OR>
where
    OR: Observer<T, E>,
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
