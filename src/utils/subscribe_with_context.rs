use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
    },
    observable::Subscription,
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{MaybeSend, MutGuard, Mutable, MutableHelper, Shared, WeakShared},
};
use educe::Educe;

delegate_disposal!(
    Disposal<'or, D>,
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

// Creates a subscription that is based on a shared mutable model and a observer.
pub fn subscribe_with_context<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<Disposal<'or, D>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable,
    M: MaybeSend + 'or,
    F: FnOnce(Context<T, E, OR, M>) -> Subscription<D>,
{
    let state = Shared::new(Mutable::new(State::Idle { observer, model }));
    let context = Context(state.clone());
    let disposable = ContextDisposable(state);
    let sub = builder(context);
    sub.preceded_by(disposable.into_boxed()).map_into()
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
pub enum EventGroup<T, E> {
    Next(T),
    Termination(Termination<E>),
    NextAndTermination(T, Termination<E>),
    Nexts(Vec<T>),
    NextsAndTermination(Vec<T>, Termination<E>),
}

#[derive(Educe)]
#[educe(Debug)]
pub struct ModificationResult<T, E, D, R> {
    send_events: Option<EventGroup<T, E>>,
    drop_outside: Option<D>,
    result: R,
}

impl<T, E, D, R> ModificationResult<T, E, D, R> {
    pub fn new(result: R) -> Self {
        Self {
            send_events: None,
            drop_outside: None,
            result,
        }
    }

    pub fn send_next(self, next: T) -> Self {
        self.assert_no_events_set();
        Self {
            send_events: Some(EventGroup::Next(next)),
            ..self
        }
    }

    pub fn send_termination(self, termination: Termination<E>) -> Self {
        self.assert_no_events_set();
        Self {
            send_events: Some(EventGroup::Termination(termination)),
            ..self
        }
    }

    pub fn send_next_and_termination(self, next: T, termination: Termination<E>) -> Self {
        self.assert_no_events_set();
        Self {
            send_events: Some(EventGroup::NextAndTermination(next, termination)),
            ..self
        }
    }

    pub fn send_events(self, events: EventGroup<T, E>) -> Self {
        self.assert_no_events_set();
        Self {
            send_events: Some(events),
            ..self
        }
    }

    pub fn drop_outside(self, object: D) -> Self {
        debug_assert!(
            self.drop_outside.is_none(),
            "drop_outside is already set; calling it again would silently overwrite (and drop) the previous object"
        );
        Self {
            drop_outside: Some(object),
            ..self
        }
    }

    fn assert_no_events_set(&self) {
        debug_assert!(
            self.send_events.is_none(),
            "send_events is already set; calling a send_* method again would silently overwrite the previous events"
        );
    }
}

impl<T, E> ModificationResult<T, E, (), ()> {
    pub fn new_empty() -> Self {
        Self {
            send_events: None,
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_next(next: T) -> Self {
        Self {
            send_events: Some(EventGroup::Next(next)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_termination(termination: Termination<E>) -> Self {
        Self {
            send_events: Some(EventGroup::Termination(termination)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_next_and_termination(next: T, termination: Termination<E>) -> Self {
        Self {
            send_events: Some(EventGroup::NextAndTermination(next, termination)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_events(events: EventGroup<T, E>) -> Self {
        Self {
            send_events: Some(events),
            drop_outside: None,
            result: (),
        }
    }
}

impl<T, E, D> ModificationResult<T, E, D, ()> {
    pub fn new_without_result() -> Self {
        Self {
            send_events: None,
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

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Stopped,
}

impl<T, E, OR, M> Context<T, E, OR, M>
where
    OR: Observer<T, E>,
{
    /// Modify the model with callback.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model<D, R>(
        &self,
        callback: impl FnOnce(&mut M) -> ModificationResult<T, E, D, R>,
    ) -> Result<R, Error> {
        self.0.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Idle { model, .. } => model,
                State::Processing { model, .. } => model,
                State::Stopped => {
                    drop(lock);
                    return Result::Err(Error::Stopped);
                }
            };
            let ModificationResult {
                send_events,
                drop_outside,
                result,
            } = callback(model);
            if let Some(send_events) = send_events {
                self.sending_impl(send_events, lock);
            } else {
                drop(lock);
            }
            drop(drop_outside); // Drop outside the lock to avoid potential deadlock
            Ok(result)
        })
    }

    pub fn send_next(&self, value: T) {
        self.0
            .lock_mut(|lock| self.sending_impl(EventGroup::Next(value), lock))
    }

    pub fn send_termination(&self, termination: Termination<E>) {
        self.0
            .lock_mut(|lock| self.sending_impl(EventGroup::Termination(termination), lock))
    }

    pub fn send_next_and_termination(&self, next: T, termination: Termination<E>) {
        self.0.lock_mut(|lock| {
            self.sending_impl(EventGroup::NextAndTermination(next, termination), lock)
        })
    }

    pub fn send_events(&self, events: EventGroup<T, E>) {
        self.0.lock_mut(|lock| self.sending_impl(events, lock))
    }

    fn sending_impl(&self, events: EventGroup<T, E>, mut lock: MutGuard<'_, State<T, E, OR, M>>) {
        match &mut *lock {
            State::Idle { .. } => {
                let (next_and_rest, termination) = match events {
                    EventGroup::Next(next) => (Some((next, None)), None),
                    EventGroup::Termination(termination) => (None, Some(termination)),
                    EventGroup::NextAndTermination(next, termination) => {
                        (Some((next, None)), Some(termination))
                    }
                    EventGroup::Nexts(items) => (split_first(items), None),
                    EventGroup::NextsAndTermination(items, termination) => {
                        (split_first(items), Some(termination))
                    }
                };
                if next_and_rest.is_none() && termination.is_none() {
                    // Empty `Nexts` is a no-op, consistent with the `Processing` state.
                    drop(lock);
                    return;
                }
                let idle_state = std::mem::replace(&mut *lock, State::Stopped);
                match idle_state {
                    State::Idle {
                        mut observer,
                        model,
                    } => {
                        if let Some((next, rest)) = next_and_rest {
                            let old_state = std::mem::replace(
                                &mut *lock,
                                State::Processing {
                                    next_values: rest.unwrap_or_default(),
                                    termination,
                                    model,
                                },
                            );
                            drop(lock); // Drop lock to avoid potential deadlock
                            drop(old_state);
                            // If the observer panics (in `on_next` here or inside
                            // `send_events_until_finish`), the state would otherwise stay
                            // `Processing` forever: later events get queued but never
                            // delivered, and the model leaks until dispose. The guard stops
                            // the state machine on panic; on normal return it is a no-op.
                            // NOTE: This is the only call site of `send_events_until_finish`,
                            // which relies on this guard.
                            let _stop_on_panic = StopOnPanic(&self.0);
                            observer.on_next(next);
                            self.send_events_until_finish(observer);
                        } else {
                            let termination = termination.expect("Termination must be set");
                            drop(lock); // Release lock.
                            drop(model); // Drop outside the lock to avoid potential deadlock
                            observer.on_termination(termination);
                        }
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
                    match events {
                        EventGroup::Next(value) => next_values.push(value),
                        EventGroup::Termination(tn) => {
                            *termination = Some(tn);
                        }
                        EventGroup::NextAndTermination(value, tn) => {
                            next_values.push(value);
                            *termination = Some(tn);
                        }
                        EventGroup::Nexts(items) => next_values.extend(items),
                        EventGroup::NextsAndTermination(items, tn) => {
                            next_values.extend(items);
                            *termination = Some(tn);
                        }
                    };
                }
                drop(lock);
            }
            State::Stopped => {
                drop(lock);
            }
        };
    }

    pub fn downgrade(&self) -> WeakContext<T, E, OR, M> {
        WeakContext(Shared::downgrade(&self.0))
    }

    fn send_events_until_finish(&self, mut observer: OR) {
        // Panic safety is provided by the `StopOnPanic` guard at the (only) call site
        // in `sending_impl`. If this function gains a new caller, that caller must set
        // up the same guard.
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
                                let old_state =
                                    std::mem::replace(&mut *lock, State::Idle { observer, model });
                                drop(lock);
                                drop(old_state);
                                None
                            }
                            State::Idle { .. } | State::Stopped => {
                                drop(lock);
                                unreachable!()
                            }
                        }
                    }
                    (true, Some(termination)) => {
                        let old_state = std::mem::replace(&mut *lock, State::Stopped);
                        drop(lock);
                        drop(old_state);
                        Some((observer, None, Some(termination)))
                    }
                    (false, None) => {
                        let next_values = std::mem::take(next_values);
                        drop(lock);
                        Some((observer, Some(next_values), None))
                    }
                    (false, Some(termination)) => {
                        let next_values = std::mem::take(next_values);
                        let old_state = std::mem::replace(&mut *lock, State::Stopped);
                        drop(lock);
                        drop(old_state);
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

/// Splits a `Vec` into its first element and the (non-empty) rest.
/// Returns `None` if the vec is empty.
fn split_first<T>(items: Vec<T>) -> Option<(T, Option<Vec<T>>)> {
    let mut iter = items.into_iter();
    let next = iter.next()?;
    let rest: Vec<T> = iter.collect();
    let rest = if rest.is_empty() { None } else { Some(rest) };
    Some((next, rest))
}

/// Sets the state to `Stopped` when dropped while panicking.
/// The panic must have happened outside the lock (observer calls are made outside the lock),
/// so locking here is safe on the panicking thread.
struct StopOnPanic<'a, T, E, OR, M>(&'a Shared<Mutable<State<T, E, OR, M>>>);

impl<T, E, OR, M> Drop for StopOnPanic<'_, T, E, OR, M> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
        }
    }
}

struct ContextDisposable<T, E, OR, M>(Shared<Mutable<State<T, E, OR, M>>>);

impl<T, E, OR, M> Disposable for ContextDisposable<T, E, OR, M> {
    fn dispose(self) {
        let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
    }
}

pub struct WeakContext<T, E, OR, M>(WeakShared<Mutable<State<T, E, OR, M>>>);

impl<T, E, OR, M> WeakContext<T, E, OR, M> {
    pub fn upgrade(&self) -> Option<Context<T, E, OR, M>> {
        Some(Context(self.0.upgrade()?))
    }
}
