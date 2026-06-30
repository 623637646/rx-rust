use crate::{
    disposable::{Disposable, subscription::Subscription},
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{MutGuard, Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;

// Creates a subscription that is based on a shared mutable model.
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

enum SendEvent<T, E> {
    SendNext(T),
    SendTermination(Termination<E>),
}

#[derive(Educe)]
#[educe(Debug)]
pub struct ModificationResult<T, E, D, R> {
    send_event: Option<SendEvent<T, E>>,
    drop_outside: Option<D>,
    result: R,
}

impl<T, E, D, R> ModificationResult<T, E, D, R> {
    pub fn new(result: R) -> Self {
        Self {
            send_event: None,
            drop_outside: None,
            result,
        }
    }

    pub fn send_next(self, next: T) -> Self {
        Self {
            send_event: Some(SendEvent::SendNext(next)),
            ..self
        }
    }

    pub fn send_termination(self, termination: Termination<E>) -> Self {
        Self {
            send_event: Some(SendEvent::SendTermination(termination)),
            ..self
        }
    }

    pub fn drop_outside(self, object: D) -> Self {
        Self {
            drop_outside: Some(object),
            ..self
        }
    }
}

impl<T, E> ModificationResult<T, E, (), ()> {
    pub fn new_send_next(next: T) -> Self {
        Self {
            send_event: Some(SendEvent::SendNext(next)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_termination(termination: Termination<E>) -> Self {
        Self {
            send_event: Some(SendEvent::SendTermination(termination)),
            drop_outside: None,
            result: (),
        }
    }
}

impl<T, E, R> ModificationResult<T, E, (), R> {
    pub fn ignore_drop_outside(self) -> Self {
        self
    }
}

impl<T, E, D> ModificationResult<T, E, D, ()> {
    pub fn new_with_drop_outside(drop_outside: D) -> Self {
        Self {
            send_event: None,
            drop_outside: Some(drop_outside),
            result: (),
        }
    }
}

impl<T, E, D> Default for ModificationResult<T, E, D, ()> {
    fn default() -> Self {
        Self {
            send_event: None,
            drop_outside: None,
            result: (),
        }
    }
}

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Stopped,
}

impl<T, E, OR, M> Context<T, E, OR, M> {
    /// Modify the model with callback.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model<D, R>(
        &self,
        callback: impl FnOnce(&mut M) -> ModificationResult<T, E, D, R>,
    ) -> Result<R, Error>
    where
        OR: Observer<T, E>,
    {
        let result = self.0.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Idle { model, .. } => model,
                State::Processing { model, .. } => model,
                State::Stopped => {
                    drop(lock);
                    return Result::Err(Error::Stopped);
                }
            };
            let ModificationResult {
                send_event,
                drop_outside,
                result,
            } = callback(model);
            if let Some(send_event) = send_event {
                match send_event {
                    SendEvent::SendNext(value) => self.send_next_impl(value, lock),
                    SendEvent::SendTermination(termination) => {
                        self.send_termination_impl(termination, lock)
                    }
                }
            } else {
                drop(lock);
            }
            drop(drop_outside); // Drop outside the lock to avoid potential deadlock
            Ok(result)
        })?;
        Ok(result)
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
                match idel_state {
                    State::Idle {
                        mut observer,
                        model,
                    } => {
                        *lock = State::Processing {
                            next_values: Vec::new(),
                            termination: None,
                            model,
                        };
                        drop(lock); // Drop lock to avoid potential deadlock
                        observer.on_next(value);
                        self.send_events_until_finish(observer);
                    }
                    State::Processing { .. } | State::Stopped => {
                        drop(lock);
                        unreachable!()
                    }
                };
            }
            State::Processing {
                next_values,
                termination,
                ..
            } => {
                if termination.is_none() {
                    next_values.push(value);
                }
                drop(lock);
            }
            State::Stopped => {
                drop(lock);
            }
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
                    State::Idle { observer, model } => {
                        drop(lock); // Release lock.
                        drop(model); // Drop outside the lock to avoid potential deadlock
                        observer.on_termination(termination);
                    }
                    State::Processing { .. } | State::Stopped => {
                        drop(lock);
                        unreachable!()
                    }
                }
            }
            State::Processing {
                termination: slot, ..
            } => {
                if slot.is_none() {
                    *slot = Some(termination);
                }
                drop(lock);
            }
            State::Stopped => {
                drop(lock);
            }
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
            let result = self.0.lock_mut(|mut lock| match &mut *lock {
                State::Idle { .. } => {
                    drop(lock);
                    panic!("Can't be called in idle state");
                }
                State::Processing {
                    next_values,
                    termination,
                    ..
                } => match (next_values.is_empty(), termination.take()) {
                    (true, None) => {
                        let processing = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
                        match processing {
                            State::Processing { model, .. } => {
                                *lock = State::Idle { observer, model };
                                drop(lock);
                                None
                            }
                            State::Idle { .. } | State::Stopped => {
                                drop(lock);
                                unreachable!()
                            }
                        }
                    }
                    (true, Some(termination)) => {
                        *lock = State::Stopped;
                        drop(lock);
                        Some((observer, None, Some(termination)))
                    }
                    (false, None) => {
                        let next_values = std::mem::take(next_values);
                        drop(lock);
                        Some((observer, Some(next_values), None))
                    }
                    (false, Some(termination)) => {
                        let next_values = std::mem::take(next_values);
                        *lock = State::Stopped;
                        drop(lock);
                        Some((observer, Some(next_values), Some(termination)))
                    }
                },
                State::Stopped => {
                    drop(lock);
                    drop(observer);
                    None
                }
            });
            let Some((mut obs, next_values, termination)) = result else {
                return;
            };
            match termination {
                Some(termination) => {
                    if let Some(next_values) = next_values {
                        for value in next_values {
                            obs.on_next(value);
                        }
                    }
                    obs.on_termination(termination);
                    break;
                }
                None => {
                    for value in next_values.unwrap() {
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
