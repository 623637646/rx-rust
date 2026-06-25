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
    F: FnOnce(Context<T, E, OR, M>) -> Subscription<'sub>,
{
    let state = Shared::new(Mutable::new(State::Idle(observer)));
    let model = Shared::new(Mutable::new(model));
    let context = Context {
        state: state.clone(),
        model,
    };
    let disposable = SharedModelDisposable(state);
    let sub = builder(context);
    sub + disposable
}

enum State<T, E, OR> {
    Idle(OR),
    Processing {
        next_values: Vec<T>,
        termination: Option<Termination<E>>,
    },
    Stopped, // Unsubscribed or disposed
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Context<T, E, OR, M> {
    state: Shared<Mutable<State<T, E, OR>>>,
    model: Shared<Mutable<M>>,
}

pub enum Action<T, E> {
    Next(T),
    Termination(Termination<E>),
    None,
}

impl<T, E, OR, M> Context<T, E, OR, M> {
    pub fn lock_model(&self, callback: impl FnOnce(&mut M) -> Action<T, E>)
    where
        OR: Observer<T, E>,
    {
        self.model.lock_mut(|mut lock| {
            let action = callback(&mut *lock);
            match action {
                Action::Next(value) => self.send_next(value, lock),
                Action::Termination(termination) => self.send_termination(termination, lock),
                Action::None => (),
            }
        });
    }

    fn send_next(&self, value: T, lock: MutGuard<'_, M>)
    where
        OR: Observer<T, E>,
    {
        // None means finish, Some means continue
        let action = self.state.lock_mut(|mut lock| match &mut *lock {
            State::Idle(_) => {
                let idel_state = std::mem::replace(
                    &mut *lock,
                    State::Processing {
                        next_values: Vec::new(),
                        termination: None,
                    },
                );
                match idel_state {
                    State::Idle(observer) => Some((observer, value)),
                    State::Processing { .. } | State::Stopped => unreachable!(),
                }
            }
            State::Processing {
                next_values,
                termination,
            } => {
                if termination.is_none() {
                    next_values.push(value);
                }
                None
            }
            State::Stopped => None,
        });
        drop(lock); // Drop lock to avoid potential deadlock
        if let Some((mut observer, value)) = action {
            observer.on_next(value);
            self.send_events_until_finish(observer);
        }
    }

    fn send_termination(&self, termination: Termination<E>, lock: MutGuard<'_, M>)
    where
        OR: Observer<T, E>,
    {
        let action = self.state.lock_mut(|mut lock| match &mut *lock {
            State::Idle(_) => {
                let idel_state = std::mem::replace(&mut *lock, State::Stopped);
                match idel_state {
                    State::Idle(observer) => Some((observer, termination)),
                    State::Processing { .. } | State::Stopped => unreachable!(),
                }
            }
            State::Processing {
                termination: slot, ..
            } => {
                if slot.is_none() {
                    *slot = Some(termination);
                }
                None
            }
            State::Stopped => None,
        });
        drop(lock); // Drop lock to avoid potential deadlock
        if let Some((observer, termination)) = action {
            observer.on_termination(termination);
        }
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
                self.state.lock_mut(|mut lock| match &mut *lock {
                    State::Idle(_) => {
                        panic!("Can't be called in idle state");
                    }
                    State::Processing {
                        next_values,
                        termination,
                    } => match (next_values.is_empty(), termination.take()) {
                        (true, None) => {
                            *lock = State::Idle(observer);
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

struct SharedModelDisposable<T, E, OR>(Shared<Mutable<State<T, E, OR>>>);

impl<T, E, OR> Disposable for SharedModelDisposable<T, E, OR> {
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
