use crate::{
    disposable::{Disposable, subscription::Subscription},
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{MutGuard, Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;

pub fn subscribe_with_shared_model<'or, 'sub, T, E, OR, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<'sub>
where
    'or: 'sub,
    T: NecessarySend + 'sub,
    E: NecessarySend + 'sub,
    OR: NecessarySend + 'or,
    M: NecessarySend + 'sub,
    F: FnOnce(Context<T, E, OR, M>) -> Subscription<'sub>,
{
    let state = Shared::new(Mutable::new(State::Idle { observer, model }));
    let context = Context(state.clone());
    let disposable = SharedModelDisposable(state);
    let sub = builder(context);
    sub + disposable
}

enum State<T, E, OR, M> {
    Idle {
        observer: OR,
        model: M,
    },
    Processing {
        next_values: Vec<T>,
        termination: Option<Termination<E>>,
        model: M,
    },
    Stopped, // Unsubscribed or disposed
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Context<T, E, OR, M>(Shared<Mutable<State<T, E, OR, M>>>);

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Action<T, E> {
    SendNext(T),
    SendTermination(Termination<E>),
    None,
}

impl<T, E, OR, M> Context<T, E, OR, M> {
    /// Modify the model with callback.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model<R>(&self, callback: impl FnOnce(Option<&mut M>) -> R) -> R
    where
        OR: Observer<T, E>,
    {
        self.modify_model_with_action_and_result(|model| (Action::None, callback(model)))
    }

    /// Modify the model with callback and return action.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model_with_action(&self, callback: impl FnOnce(Option<&mut M>) -> Action<T, E>)
    where
        OR: Observer<T, E>,
    {
        self.modify_model_with_action_and_result(|model| {
            let action = callback(model);
            (action, ())
        })
    }

    /// Modify the model with callback and return action and custom result.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model_with_action_and_result<R>(
        &self,
        callback: impl FnOnce(Option<&mut M>) -> (Action<T, E>, R),
    ) -> R
    where
        OR: Observer<T, E>,
    {
        self.0.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Idle { model, .. } => Some(model),
                State::Processing { model, .. } => Some(model),
                State::Stopped => None,
            };
            let (action, result) = callback(model);
            match action {
                Action::SendNext(value) => self.send_next_impl(value, lock),
                Action::SendTermination(termination) => {
                    self.send_termination_impl(termination, lock)
                }
                Action::None => (),
            }
            result
        })
    }

    pub fn send_next(&self, value: T)
    where
        OR: Observer<T, E>,
    {
        self.0.lock_mut(|lock| self.send_next_impl(value, lock))
    }

    pub fn send_termination(&self, termination: Termination<E>)
    where
        OR: Observer<T, E>,
    {
        self.0
            .lock_mut(|lock| self.send_termination_impl(termination, lock))
    }

    fn send_next_impl(&self, value: T, mut lock: MutGuard<'_, State<T, E, OR, M>>)
    where
        OR: Observer<T, E>,
    {
        // None means finish, Some means continue
        match &mut *lock {
            State::Idle { .. } => {
                let idel_state = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
                let (mut observer, model) = match idel_state {
                    State::Idle { observer, model } => (observer, model),
                    State::Processing { .. } | State::Stopped => unreachable!(),
                };
                *lock = State::Processing {
                    next_values: Vec::new(),
                    termination: None,
                    model,
                };
                drop(lock); // Drop lock to avoid potential deadlock
                observer.on_next(value);
                self.send_events_until_finish(observer);
            }
            State::Processing {
                next_values,
                termination,
                ..
            } => {
                if termination.is_none() {
                    next_values.push(value);
                }
            }
            State::Stopped => {}
        };
    }

    fn send_termination_impl(
        &self,
        termination: Termination<E>,
        mut lock: MutGuard<'_, State<T, E, OR, M>>,
    ) where
        OR: Observer<T, E>,
    {
        match &mut *lock {
            State::Idle { .. } => {
                let idel_state = std::mem::replace(&mut *lock, State::Stopped);
                match idel_state {
                    State::Idle { observer, .. } => {
                        drop(lock); // Drop lock to avoid potential deadlock
                        observer.on_termination(termination);
                    }
                    State::Processing { .. } | State::Stopped => unreachable!(),
                }
            }
            State::Processing {
                termination: slot, ..
            } => {
                if slot.is_none() {
                    *slot = Some(termination);
                }
            }
            State::Stopped => {}
        };
    }

    // pub fn downgrade(&self) -> WeakContext<T, E, OR, M> {
    //     WeakContext {
    //         state: Shared::downgrade(&self.state),
    //         model: Shared::downgrade(&self.model),
    //     }
    // }

    fn send_events_until_finish(&self, mut observer: OR)
    where
        OR: Observer<T, E>,
    {
        loop {
            let (returned_observer, next_values, termination) =
                self.0.lock_mut(|mut lock| match &mut *lock {
                    State::Idle { .. } => {
                        panic!("Can't be called in idle state");
                    }
                    State::Processing {
                        next_values,
                        termination,
                        ..
                    } => match (next_values.is_empty(), termination.take()) {
                        (true, None) => {
                            let processing = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
                            let model = match processing {
                                State::Processing { model, .. } => model,
                                State::Idle { .. } | State::Stopped => unreachable!(),
                            };
                            *lock = State::Idle { observer, model };
                            (None, None, None)
                        }
                        (true, Some(termination)) => {
                            *lock = State::Stopped;
                            (Some(observer), None, Some(termination))
                        }
                        (false, None) => {
                            let next_values = std::mem::take(next_values);
                            (Some(observer), Some(next_values), None)
                        }
                        (false, Some(termination)) => {
                            let next_values = std::mem::take(next_values);
                            *lock = State::Stopped;
                            (Some(observer), Some(next_values), Some(termination)) // Drop observer outside the lock to avoid potential deadlock
                        }
                    },
                    State::Stopped => (Some(observer), None, None),
                });

            match (returned_observer, next_values, termination) {
                (None, _, _) | (Some(_), None, None) => break,
                (Some(mut obs), next_values, Some(termination)) => {
                    if let Some(next_values) = next_values {
                        for value in next_values {
                            obs.on_next(value);
                        }
                    }
                    obs.on_termination(termination);
                    break;
                }
                (Some(mut obs), Some(next_values), None) => {
                    for value in next_values {
                        obs.on_next(value);
                    }
                    observer = obs;
                }
            }
        }
    }
}

struct SharedModelDisposable<T, E, OR, M>(Shared<Mutable<State<T, E, OR, M>>>);

impl<T, E, OR, M> Disposable for SharedModelDisposable<T, E, OR, M> {
    fn dispose(self) {
        let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
    }
}

// pub struct WeakContext<T, E, OR, M> {
//     state: WeakShared<Mutable<State<T, E, OR>>>,
//     model: WeakShared<Mutable<M>>,
// }

// impl<T, E, OR, M> WeakContext<T, E, OR, M> {
//     pub fn upgrade(&self) -> Option<Context<T, E, OR, M>> {
//         Some(Context {
//             state: self.state.upgrade()?,
//             model: self.model.upgrade()?,
//         })
//     }
// }
