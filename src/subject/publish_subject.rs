use super::Subject;
use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::SafeLock;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
};
use educe::Educe;
use slotmap::{DefaultKey, DenseSlotMap};

enum ObserverActionInsertState<'or, T, E> {
    BeforeInserted(BoxedObserver<'or, T, E>),
    Inserted(DefaultKey),
    Cancelled,
}

enum ContinueAction<'or, T, E> {
    InsertObserver(Shared<Mutable<ObserverActionInsertState<'or, T, E>>>),
    RemoveObserver(DefaultKey),
    EmitNext(T),
}

enum ProcessedAction<'or, T, E> {
    Continue(Vec<ContinueAction<'or, T, E>>),
    Terminate(Termination<E>),
}

enum State<'or, T, E> {
    Idle(DenseSlotMap<DefaultKey, BoxedObserver<'or, T, E>>),
    Processing(ProcessedAction<'or, T, E>),
    Terminated(Termination<E>),
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self(Shared::new(Mutable::new(State::Idle(DenseSlotMap::new()))))
    }
}

impl<T, E> Default for PublishSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: NecessarySend + 'sub,
    E: Clone + NecessarySend + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let mut lock = self.0.lock_mut();
        match &mut *lock {
            State::Idle(observers) => {
                let key = observers.insert(BoxedObserver::new(observer));
                drop(lock);
                Subscription::new_with_disposal(PublishSubjectKeyDisposal { state: self.0, key })
            }
            State::Processing(result) => match result {
                ProcessedAction::Continue(actions) => {
                    let insert_state = Shared::new(Mutable::new(
                        ObserverActionInsertState::BeforeInserted(BoxedObserver::new(observer)),
                    ));
                    actions.push(ContinueAction::InsertObserver(insert_state.clone()));
                    drop(lock);
                    Subscription::new_with_disposal(PublishSubjectInsertStateDisposal {
                        state: self.0,
                        insert_state,
                    })
                }
                ProcessedAction::Terminate(termination) => {
                    let termination = termination.clone();
                    drop(lock);
                    observer.on_termination(termination);
                    Subscription::default()
                }
            },
            State::Terminated(termination) => {
                let termination = termination.clone();
                drop(lock);
                observer.on_termination(termination);
                Subscription::default()
            }
        }
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.0.lock_mut();
        match std::mem::replace(
            &mut *lock,
            State::Processing(ProcessedAction::Continue(Vec::new())),
        ) {
            State::Idle(mut observers) => {
                drop(lock);

                // Notify
                observers
                    .values_mut()
                    .for_each(|observer| observer.on_next(value.clone()));

                // Will reset to Idle
                let mut lock = self.0.lock_mut();
                match std::mem::replace(
                    &mut *lock,
                    State::Processing(ProcessedAction::Continue(Vec::new())), // Set to "Zero Processing" first. Correct it later.
                ) {
                    State::Idle(_) => unreachable!(),
                    State::Processing(processed_action) => {
                        drop(lock);
                        match processed_action {
                            ProcessedAction::Continue(actions) => {
                                for action in actions {
                                    match action {
                                        ContinueAction::InsertObserver(insert_state) => {
                                            let mut lock = insert_state.lock_mut();
                                            match std::mem::replace(
                                                &mut *lock,
                                                ObserverActionInsertState::Cancelled, // set to Cancelled first. Correct it later.
                                            ) {
                                                ObserverActionInsertState::BeforeInserted(
                                                    observer,
                                                ) => {
                                                    let key = observers.insert(observer);
                                                    *lock =
                                                        ObserverActionInsertState::Inserted(key);
                                                }
                                                ObserverActionInsertState::Inserted(_) => {
                                                    unreachable!()
                                                }
                                                ObserverActionInsertState::Cancelled => {
                                                    // Do nothing if it was already cancelled
                                                }
                                            }
                                        }
                                        ContinueAction::RemoveObserver(key) => {
                                            observers.remove(key);
                                        }
                                        ContinueAction::EmitNext(value) => {
                                            observers.values_mut().for_each(|observer| {
                                                observer.on_next(value.clone())
                                            });
                                        }
                                    }
                                }

                                // Reset to Idle
                                self.0.safe_lock_set(State::Idle(observers));
                            }
                            ProcessedAction::Terminate(termination) => {
                                self.0.safe_lock_set(State::Terminated(termination.clone()));
                                observers.into_iter().for_each(|(_, observer)| {
                                    observer.on_termination(termination.clone());
                                })
                            }
                        }
                    }
                    State::Terminated(_) => unreachable!(),
                };
            }
            State::Processing(processed_action) => match processed_action {
                ProcessedAction::Continue(mut actions) => {
                    actions.push(ContinueAction::EmitNext(value));
                }
                ProcessedAction::Terminate(_) => {
                    // ignore if it will be terminated
                }
            },
            State::Terminated(termination) => {
                // It's already terminated. revert
                _ = std::mem::replace(&mut *lock, State::Terminated(termination));
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut lock = self.0.lock_mut();
        match std::mem::replace(&mut *lock, State::Terminated(termination.clone())) {
            State::Idle(observers) => {
                drop(lock);
                observers.into_iter().for_each(|(_, observer)| {
                    observer.on_termination(termination.clone());
                });
            }
            State::Processing(processed_action) => {
                assert!(matches!(processed_action, ProcessedAction::Continue(_)));
                // revert
                _ = std::mem::replace(
                    &mut *lock,
                    State::Processing(ProcessedAction::Terminate(termination)),
                );
            }
            State::Terminated(termination) => {
                // revert
                _ = std::mem::replace(&mut *lock, State::Terminated(termination));
            }
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: Clone + NecessarySend + 'sub,
    E: Clone + NecessarySend + 'sub,
    'or: 'sub,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        match &*self.0.lock_ref() {
            State::Idle(_) => None,
            State::Processing(_) => None,
            State::Terminated(termination) => Some(termination.clone()),
        }
    }
}

struct PublishSubjectKeyDisposal<'or, T, E> {
    state: Shared<Mutable<State<'or, T, E>>>,
    key: DefaultKey,
}

impl<T, E> Disposable for PublishSubjectKeyDisposal<'_, T, E> {
    fn dispose(self) {
        match &mut *self.state.lock_mut() {
            State::Idle(observers) => {
                observers.remove(self.key);
            }
            State::Processing(processed_action) => match processed_action {
                ProcessedAction::Continue(actions) => {
                    actions.push(ContinueAction::RemoveObserver(self.key));
                }
                ProcessedAction::Terminate(_) => {
                    // Do nothing if it will be terminated
                }
            },
            State::Terminated(_) => {
                // Do nothing if it was already terminated
            }
        };
    }
}

struct PublishSubjectInsertStateDisposal<'or, T, E> {
    state: Shared<Mutable<State<'or, T, E>>>,
    insert_state: Shared<Mutable<ObserverActionInsertState<'or, T, E>>>,
}

impl<T, E> Disposable for PublishSubjectInsertStateDisposal<'_, T, E> {
    fn dispose(self) {
        let mut lock = self.insert_state.lock_mut();
        match std::mem::replace(&mut *lock, ObserverActionInsertState::Cancelled) {
            ObserverActionInsertState::BeforeInserted(_) => {
                // Unsubscribed before the observer was inserted
            }
            ObserverActionInsertState::Inserted(key) => {
                drop(lock);
                match &mut *self.state.lock_mut() {
                    State::Idle(observers) => {
                        observers.remove(key);
                    }
                    State::Processing(processed_action) => match processed_action {
                        ProcessedAction::Continue(actions) => {
                            actions.push(ContinueAction::RemoveObserver(key));
                        }
                        ProcessedAction::Terminate(_) => {
                            // Do nothing if it will be terminated
                        }
                    },
                    State::Terminated(_) => {
                        // Do nothing if it was already terminated
                    }
                };
            }
            ObserverActionInsertState::Cancelled => unreachable!(),
        }
    }
}
