use crate::{
    disposable::{Disposable, subscription::Subscription},
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{MarkerType, Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;
use std::marker::PhantomData;

pub trait SharedModel<T0, T, E, OR>: Sized {
    fn on_next(context: Context<T, E, OR, Self>, value: T0);

    fn on_termination(context: Context<T, E, OR, Self>, termination: Termination<E>);

    fn on_dispose(_context: Context<T, E, OR, Self>) {}
}

pub trait SharedModelObservable<'or, 'sub, T0, E> {
    fn subscribe_with_shared_model<T, OR>(
        self,
        observer: OR,
        model: impl SharedModel<T0, T, E, OR> + NecessarySend + 'or + 'sub,
    ) -> Subscription<'sub>
    where
        T0: 'sub,
        T: NecessarySend + 'or + 'sub,
        E: NecessarySend + 'or + 'sub,
        OR: Observer<T, E> + NecessarySend + 'or + 'sub;
}

impl<'or, 'sub, T0, E, OE> SharedModelObservable<'or, 'sub, T0, E> for OE
where
    OE: Observable<'or, 'sub, T0, E>,
{
    fn subscribe_with_shared_model<T, OR>(
        self,
        observer: OR,
        model: impl SharedModel<T0, T, E, OR> + NecessarySend + 'or + 'sub,
    ) -> Subscription<'sub>
    where
        T0: 'sub,
        T: NecessarySend + 'or + 'sub,
        E: NecessarySend + 'or + 'sub,
        OR: Observer<T, E> + NecessarySend + 'or + 'sub,
    {
        let state = Shared::new(Mutable::new(State::Idle(observer)));
        let model = Shared::new(Mutable::new(model));
        let context = Context { state, model };
        let disposable = SharedModelDisposable {
            context: context.clone(),
            _marker: PhantomData,
        };
        let observer = SharedModelObserver(context);
        self.subscribe(observer) + disposable
    }
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
    pub model: Shared<Mutable<M>>,
}

impl<T, E, OR, M> Context<T, E, OR, M>
where
    OR: Observer<T, E>,
{
    pub fn send_next(&self, value: T) {
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
        if let Some((mut observer, value)) = action {
            observer.on_next(value);
            self.send_events_until_finish(observer);
        }
    }

    pub fn send_termination(&self, termination: Termination<E>) {
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
        if let Some((observer, termination)) = action {
            observer.on_termination(termination);
        }
    }

    fn send_events_until_finish(&self, mut observer: OR) {
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

struct SharedModelObserver<T, E, OR, M>(Context<T, E, OR, M>);

impl<T0, T, E, OR, M> Observer<T0, E> for SharedModelObserver<T, E, OR, M>
where
    OR: Observer<T, E>,
    M: SharedModel<T0, T, E, OR>,
{
    fn on_next(&mut self, value: T0) {
        M::on_next(self.0.clone(), value);
    }

    fn on_termination(self, termination: crate::observer::Termination<E>) {
        M::on_termination(self.0, termination);
    }
}

struct SharedModelDisposable<T0, T, E, OR, M> {
    context: Context<T, E, OR, M>,
    _marker: MarkerType<T0>,
}

impl<T0, T, E, OR, M> Disposable for SharedModelDisposable<T0, T, E, OR, M>
where
    M: SharedModel<T0, T, E, OR>,
{
    fn dispose(self) {
        let _old_state = safe_lock!(mem_replace: self.context.state, State::Stopped);
        M::on_dispose(self.context);
    }
}
