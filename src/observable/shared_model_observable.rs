use crate::{
    disposable::{Disposable, subscription::Subscription},
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{ActionAfterLock, Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;
use std::collections::VecDeque;

pub trait SharedModel<T0, T, E, OR, EX>: Sized {
    fn on_next(value: T0, context: Context<T, E, OR, Self>, extra: &mut EX);

    fn on_termination(termination: Termination<E>, context: Context<T, E, OR, Self>, extra: EX);
}

pub trait SharedModelObservable<'or, 'sub, T0, E> {
    fn subscribe_with_shared_model<T, OR, EX>(
        self,
        observer: OR,
        model: impl SharedModel<T0, T, E, OR, EX> + NecessarySend + 'or + 'sub,
        extra: EX,
    ) -> Subscription<'sub>
    where
        T: NecessarySend + 'or + 'sub,
        E: NecessarySend + 'or + 'sub,
        OR: Observer<T, E> + NecessarySend + 'or + 'sub,
        EX: NecessarySend + 'or;
}

impl<'or, 'sub, T0, E, OE> SharedModelObservable<'or, 'sub, T0, E> for OE
where
    OE: Observable<'or, 'sub, T0, E>,
{
    fn subscribe_with_shared_model<T, OR, EX>(
        self,
        observer: OR,
        model: impl SharedModel<T0, T, E, OR, EX> + NecessarySend + 'or + 'sub,
        extra: EX,
    ) -> Subscription<'sub>
    where
        T: NecessarySend + 'or + 'sub,
        E: NecessarySend + 'or + 'sub,
        OR: Observer<T, E> + NecessarySend + 'or + 'sub,
        EX: NecessarySend + 'or,
    {
        let state = Shared::new(Mutable::new(State::Idle(observer)));
        let model = Shared::new(Mutable::new(model));
        let disposable = SharedModelDisposable(state.clone());
        let context = Context { state, model };
        let observer = SharedModelObserver { context, extra };
        self.subscribe(observer) + disposable
    }
}

enum State<T, E, OR> {
    Idle(OR),
    Processing {
        next_values: VecDeque<T>,
        termination: Option<Termination<E>>,
    },
    Stopped, // Unsubscribed or disposed
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Context<T, E, OR, M> {
    state: Shared<Mutable<State<T, E, OR>>>,
    pub model: Shared<Mutable<M>>,
}

impl<T, E, OR, M> Context<T, E, OR, M>
where
    OR: Observer<T, E>,
{
    pub fn send_next(&self, value: T) {
        // None means finish, Some means continue
        let action = self
            .state
            .safe_lock_mut_with_args(value, |state, value| match state {
                State::Idle(_) => {
                    let idel_state = std::mem::replace(
                        &mut *state,
                        State::Processing {
                            next_values: VecDeque::new(),
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
                        next_values.push_back(value);
                    }
                    None
                }
                State::Stopped => None,
            });
        if let Some((mut observer, value)) = action {
            observer.on_next(value);
            self.send_events_until_finish(observer);
        }
    }

    pub fn send_termination(&self, termination: Termination<E>) {
        let action = self
            .state
            .safe_lock_mut_with_args(termination, |state, termination| match state {
                State::Idle(_) => {
                    let idel_state = std::mem::replace(state, State::Stopped);
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
        if let Some((observer, termination)) = action {
            observer.on_termination(termination);
        }
    }

    fn send_events_until_finish(&self, mut observer: OR) {
        loop {
            let (action, returned_observer) =
                self.state
                    .safe_lock_mut_with_args(observer, |state, observer| match state {
                        State::Idle(_) => {
                            panic!("Can't be called in idle state");
                        }
                        State::Processing {
                            next_values,
                            termination,
                        } => {
                            if let Some(next) = next_values.pop_front() {
                                (ActionAfterLock::Next(next), Some(observer))
                            } else if let Some(termination) = termination.take() {
                                *state = State::Stopped;
                                (ActionAfterLock::Termination(termination), Some(observer))
                            } else {
                                *state = State::Idle(observer);
                                (ActionAfterLock::None, None)
                            }
                        }
                        State::Stopped => (ActionAfterLock::None, Some(observer)),
                    });

            if let Some(mut obs) = returned_observer {
                match action {
                    ActionAfterLock::Next(value) => {
                        obs.on_next(value);
                        observer = obs; // continue the loop
                    }
                    ActionAfterLock::Termination(termination) => {
                        obs.on_termination(termination);
                        break;
                    }
                    ActionAfterLock::None => {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }
}

struct SharedModelObserver<T, E, OR, M, EX> {
    context: Context<T, E, OR, M>,
    extra: EX,
}

impl<T0, T, E, OR, M, EX> Observer<T0, E> for SharedModelObserver<T, E, OR, M, EX>
where
    OR: Observer<T, E>,
    M: SharedModel<T0, T, E, OR, EX>,
{
    fn on_next(&mut self, value: T0) {
        M::on_next(value, self.context.clone(), &mut self.extra);
    }

    fn on_termination(self, termination: crate::observer::Termination<E>) {
        M::on_termination(termination, self.context.clone(), self.extra);
    }
}

struct SharedModelDisposable<T, E, OR>(Shared<Mutable<State<T, E, OR>>>);

impl<T, E, OR> Disposable for SharedModelDisposable<T, E, OR> {
    fn dispose(self) {
        let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
    }
}
