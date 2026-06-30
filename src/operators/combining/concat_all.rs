use crate::disposable::subscription::Subscription;
use crate::observable::observable_ext::ObservableExt;
use crate::operators::others::map_infallible_to_error::MapInfallibleToError;
use crate::utils::subscribe_with_shared_model::{
    Context, Error, ModificationResult, subscribe_with_shared_model,
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
use std::{collections::VecDeque, marker::PhantomData};

/// Concatenates an Observable of Observables, emitting all values from each inner Observable in sequence.
/// See <https://reactivex.io/documentation/operators/concat.html> (referencing concat operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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

impl<E, OE1, I> ConcatAll<MapInfallibleToError<E, FromIter<I>>, OE1> {
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

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for ConcatAll<OE, OE1>
where
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                pending_observables: VecDeque::new(),
                sub_state: SubState::Idle,
                is_source_completed: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                self.source.subscribe(SourceObserver(context.clone()))
            })
        })
    }
}

enum SubState<'sub> {
    Idle,
    PendingSubscription,
    Processing(Subscription<'sub>),
}

struct Model<'sub, OE1> {
    pending_observables: VecDeque<OE1>,
    sub_state: SubState<'sub>,
    is_source_completed: bool,
}

struct SourceObserver<'sub, T, E, OR, OE1>(Context<T, E, OR, Model<'sub, OE1>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for SourceObserver<'sub, T, E, OR, OE1>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        let result = self.0.modify_model(|model| match model.sub_state {
            SubState::Idle => {
                let _ = std::mem::replace(&mut model.sub_state, SubState::PendingSubscription);
                ModificationResult::new(Some(value)).ignore_drop_outside()
            }
            SubState::PendingSubscription | SubState::Processing(_) => {
                model.pending_observables.push_back(value);
                ModificationResult::new(None)
            }
        });
        let observable = match result {
            Ok(Some(observable)) => observable,
            Ok(None) => return,
            Err(Error::Stopped) => return,
        };
        let observer = InnerObserver(self.0.clone());
        let sub = observable.subscribe(observer);
        let _ = self.0.modify_model(|model| {
            match &model.sub_state {
                SubState::Idle => ModificationResult::default().drop_outside(sub), // already terminated
                SubState::PendingSubscription => {
                    let _ = std::mem::replace(&mut model.sub_state, SubState::Processing(sub));
                    ModificationResult::default()
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
                        SubState::Idle => {
                            // The state is only possible to be PendingSubscription or Processing when the pending_observables is not empty.
                            assert!(model.pending_observables.is_empty());
                            ModificationResult::new_send_termination(termination)
                        }
                        SubState::PendingSubscription | SubState::Processing(_) => {
                            ModificationResult::default()
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

struct InnerObserver<'sub, T, E, OR, OE1>(Context<T, E, OR, Model<'sub, OE1>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<T, E> for InnerObserver<'sub, T, E, OR, OE1>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => subscribe_next_observable_until_finished(self.0.clone()),
            Termination::Error(error) => {
                self.0.send_termination(Termination::Error(error));
            }
        }
    }
}

fn subscribe_next_observable_until_finished<'or, 'sub, T, E, OR, OE1>(
    context: Context<T, E, OR, Model<'sub, OE1>>,
) where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
{
    loop {
        let result = context.modify_model(|model| {
            match &model.sub_state {
                SubState::PendingSubscription => {
                    // already terminated
                    let _ = std::mem::replace(&mut model.sub_state, SubState::Idle);
                    ModificationResult::new(None)
                }
                SubState::Idle | SubState::Processing(_) => {
                    if let Some(observable) = model.pending_observables.pop_front() {
                        match std::mem::replace(&mut model.sub_state, SubState::PendingSubscription)
                        {
                            SubState::Idle => ModificationResult::new(Some(observable)),
                            SubState::PendingSubscription => {
                                unreachable!()
                            }
                            SubState::Processing(subscription) => {
                                ModificationResult::new(Some(observable)).drop_outside(subscription)
                            }
                        }
                    } else if model.is_source_completed {
                        ModificationResult::new(None).send_termination(Termination::Completed)
                    } else {
                        match std::mem::replace(&mut model.sub_state, SubState::Idle) {
                            SubState::Idle => ModificationResult::new(None),
                            SubState::PendingSubscription => {
                                unreachable!()
                            }
                            SubState::Processing(subscription) => {
                                ModificationResult::new(None).drop_outside(subscription)
                            }
                        }
                    }
                }
            }
        });
        let observable = match result {
            Ok(Some(observable)) => observable,
            Ok(None) => break,
            Err(Error::Stopped) => {
                break;
            }
        };
        let observer = InnerObserver(context.clone());
        let sub = observable.subscribe(observer);
        let result = context.modify_model(|model| {
            match &model.sub_state {
                SubState::Idle => ModificationResult::new(false).drop_outside(sub), // already terminated
                SubState::PendingSubscription => {
                    let _ = std::mem::replace(&mut model.sub_state, SubState::Processing(sub));
                    ModificationResult::new(true)
                }
                SubState::Processing(_) => unreachable!(),
            }
        });
        match result {
            Ok(stop) => {
                if stop {
                    break;
                }
            }
            Err(Error::Stopped) => break,
        }
    }
}
