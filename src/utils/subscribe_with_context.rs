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
use std::collections::VecDeque;

delegate_disposal!(
    Disposal<'or, D>, // TODO: Rename?
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

// TODO: Rename?
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
    assert_observer::<OR>();
    let state = Shared::new(Mutable::new(State::Idle {
        observer,
        model,
        sub: Subscription::default(),
    }));
    let context = Context(state.clone());
    let disposable = ContextDisposable(state);
    let sub = builder(context);
    sub.preceded_by(disposable.into_boxed()).map_into()
}

pub type BoundDisposal<'or> = BoxedDisposal<'or>;

pub fn subscribe_with_context_bound_disposal<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<BoundDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
    F: FnOnce(Context<T, E, OR, M, D>) -> Subscription<D>,
{
    assert_observer::<OR>();
    let state = Shared::new(Mutable::new(State::Subscribing { observer, model }));
    let context = Context(state.clone());
    let disposable = ContextDisposable(state.clone());
    let sub = builder(context);
    let sub_to_drop = state.lock_mut(|mut lock| {
        let state = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
        match state {
            State::Subscribing { observer, model } => {
                *lock = State::Idle {
                    observer,
                    model,
                    sub,
                };
                None
            }
            State::Idle { .. } => unreachable!(),
            State::Processing {
                next_values,
                termination,
                model,
                sub: None,
            } => {
                *lock = State::Processing {
                    next_values,
                    termination,
                    model,
                    sub: Some(sub),
                };
                None
            }
            State::Processing { sub: Some(_), .. } => unreachable!(),
            State::Stopped => Some(sub),
        }
    });
    drop(sub_to_drop); // Drop outside the lock to avoid potential deadlock
    disposable.into_boxed().into_subscription()
}

#[derive(Educe)]
#[educe(Debug)]
enum State<T, E, OR, M, D: Disposable> {
    Subscribing {
        observer: OR,
        model: M,
    },
    Idle {
        observer: OR,
        model: M,
        sub: Subscription<D>,
    },
    Processing {
        next_values: VecDeque<T>,
        termination: Option<Termination<E>>,
        model: M,
        sub: Option<Subscription<D>>, // Back to Subscribing if None, otherwise to Idle.
    },
    Stopped, // Unsubscribed or disposed
}

type SharedState<T, E, OR, M, D> = Shared<Mutable<State<T, E, OR, M, D>>>;
type WeakSharedState<T, E, OR, M, D> = WeakShared<Mutable<State<T, E, OR, M, D>>>;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Context<T, E, OR, M, D: Disposable = ()>(SharedState<T, E, OR, M, D>);

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum EventGroup<T, E> {
    Next(T),
    Termination(Termination<E>),
    NextAndTermination(T, Termination<E>),
    Nexts(Vec<T>),
    NextsAndTermination(Vec<T>, Termination<E>),
}

impl<T, E> EventGroup<T, E> {
    /// Splits the group into the next values to queue and an optional termination.
    fn into_parts(self) -> (VecDeque<T>, Option<Termination<E>>) {
        match self {
            EventGroup::Next(next) => (VecDeque::from([next]), None),
            EventGroup::Termination(termination) => (VecDeque::new(), Some(termination)),
            EventGroup::NextAndTermination(next, termination) => {
                (VecDeque::from([next]), Some(termination))
            }
            EventGroup::Nexts(items) => (items.into(), None),
            EventGroup::NextsAndTermination(items, termination) => {
                (items.into(), Some(termination))
            }
        }
    }
}

#[derive(Educe)]
#[educe(Debug)]
pub struct ModificationResult<T, E, A, R> {
    send_events: Option<EventGroup<T, E>>,
    drop_outside: Option<A>,
    result: R,
}

impl<T, E, A, R> ModificationResult<T, E, A, R> {
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

    pub fn drop_outside(self, object: A) -> Self {
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

impl<T, E, A> ModificationResult<T, E, A, ()> {
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

impl<T, E, OR, M, D> Context<T, E, OR, M, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Modify the model with callback.
    /// IMPORTANT: It may cause deadlock if call outside APIs inside callback (even drop object inside).
    pub fn modify_model<A, R>(
        &self,
        callback: impl FnOnce(&mut M) -> ModificationResult<T, E, A, R>,
    ) -> Result<R, Error> {
        self.0.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Subscribing { model, .. } => model,
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

    fn sending_impl(
        &self,
        events: EventGroup<T, E>,
        mut lock: MutGuard<'_, State<T, E, OR, M, D>>,
    ) {
        match &mut *lock {
            State::Idle { .. } | State::Subscribing { .. } => {
                let (next_values, termination) = events.into_parts();
                if next_values.is_empty() && termination.is_none() {
                    // Empty `Nexts` is a no-op, consistent with the `Processing` state.
                    drop(lock);
                    return;
                }
                let idle_state = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
                let (observer, model, sub) = match idle_state {
                    State::Subscribing { observer, model } => (observer, model, None),
                    State::Idle {
                        observer,
                        model,
                        sub,
                    } => (observer, model, Some(sub)),
                    State::Processing { .. } | State::Stopped => {
                        drop(lock);
                        unreachable!()
                    }
                };
                *lock = State::Processing {
                    next_values,
                    termination,
                    model,
                    sub,
                };
                drop(lock); // Deliver events outside the lock to avoid potential deadlock
                // If the observer panics inside `deliver_pending_events`, the state
                // would otherwise stay `Processing` forever: later events get queued
                // but never delivered, and the model leaks until dispose. The guard
                // stops the state machine on panic; on normal return it is a no-op.
                // NOTE: This is the only call site of `deliver_pending_events`,
                // which relies on this guard.
                let _stop_on_panic = StopOnPanic(&self.0);
                self.deliver_pending_events(observer);
            }
            State::Processing {
                next_values,
                termination,
                ..
            } => {
                if termination.is_none() {
                    match events {
                        EventGroup::Next(value) => next_values.push_back(value),
                        EventGroup::Termination(tn) => {
                            *termination = Some(tn);
                        }
                        EventGroup::NextAndTermination(value, tn) => {
                            next_values.push_back(value);
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

    pub fn downgrade(&self) -> WeakContext<T, E, OR, M, D> {
        WeakContext(Shared::downgrade(&self.0))
    }

    /// Delivers queued events to the observer one at a time, reacquiring the lock
    /// between events. Every observer call happens outside the lock, and disposal
    /// (`Stopped`) takes effect between any two events — including within a batch
    /// queued via `EventGroup::Nexts`.
    ///
    /// Panic safety is provided by the `StopOnPanic` guard at the (only) call site
    /// in `sending_impl`. If this function gains a new caller, that caller must set
    /// up the same guard.
    fn deliver_pending_events(&self, mut observer: OR) {
        loop {
            let step = self.0.lock_mut(|mut lock| match &mut *lock {
                State::Idle { .. } | State::Subscribing { .. } => {
                    drop(lock);
                    unreachable!()
                }
                State::Processing { next_values, .. } => {
                    if let Some(value) = next_values.pop_front() {
                        drop(lock);
                        return DeliveryStep::Next(observer, value);
                    }
                    let old_state = std::mem::replace(&mut *lock, State::Stopped); // Placeholder
                    let State::Processing {
                        termination,
                        model,
                        sub,
                        ..
                    } = old_state
                    else {
                        drop(lock);
                        unreachable!()
                    };
                    match termination {
                        Some(termination) => {
                            // The state stays `Stopped`: the subscription is over.
                            drop(lock);
                            drop(model); // Drop outside the lock to avoid potential deadlock
                            DeliveryStep::Terminate(observer, termination, sub)
                        }
                        None => {
                            *lock = match sub {
                                Some(sub) => State::Idle {
                                    observer,
                                    model,
                                    sub,
                                },
                                None => State::Subscribing { observer, model },
                            };
                            drop(lock);
                            DeliveryStep::Finished
                        }
                    }
                }
                State::Stopped => {
                    drop(lock);
                    DeliveryStep::Stopped(observer)
                }
            });
            match step {
                DeliveryStep::Next(mut obs, value) => {
                    obs.on_next(value);
                    observer = obs;
                }
                DeliveryStep::Terminate(obs, termination, sub_to_drop) => {
                    obs.on_termination(termination);
                    // Preserve termination-before-disposal ordering. If the callback panics,
                    // stack unwinding still drops the subscription.
                    drop(sub_to_drop);
                    return;
                }
                DeliveryStep::Finished => return,
                DeliveryStep::Stopped(observer) => {
                    drop(observer); // Drop outside the lock to avoid potential deadlock
                    return;
                }
            }
        }
    }
}

/// One step of `deliver_pending_events`. Computed under the lock, acted on
/// outside it; the observer is threaded through so it is never used or dropped
/// while the lock is held.
enum DeliveryStep<T, E, OR, D: Disposable> {
    /// Deliver one `next` value, then loop.
    Next(OR, T),
    /// Deliver the termination, then drop the subscription. The state is already `Stopped`.
    Terminate(OR, Termination<E>, Option<Subscription<D>>),
    /// No pending events; the observer was stored back into the state.
    Finished,
    /// The subscription was stopped; drop the observer outside the lock.
    Stopped(OR),
}

/// Sets the state to `Stopped` when dropped while panicking.
/// The panic must have happened outside the lock (observer calls are made outside the lock),
/// so locking here is safe on the panicking thread.
struct StopOnPanic<'a, T, E, OR, M, D: Disposable>(&'a SharedState<T, E, OR, M, D>);

impl<T, E, OR, M, D: Disposable> Drop for StopOnPanic<'_, T, E, OR, M, D> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
        }
    }
}

struct ContextDisposable<T, E, OR, M, D: Disposable>(SharedState<T, E, OR, M, D>);

impl<T, E, OR, M, D: Disposable> Disposable for ContextDisposable<T, E, OR, M, D> {
    fn dispose(self) {
        let _old_state = safe_lock!(mem_replace: self.0, State::Stopped);
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakContext<T, E, OR, M, D: Disposable = ()>(WeakSharedState<T, E, OR, M, D>);

impl<T, E, OR, M, D: Disposable> WeakContext<T, E, OR, M, D> {
    pub fn upgrade(&self) -> Option<Context<T, E, OR, M, D>> {
        Some(Context(self.0.upgrade()?))
    }
}

fn assert_observer<OR>() {
    debug_assert!(
        {
            fn type_name_without_generics<T>() -> &'static str {
                std::any::type_name::<T>()
                    .split_once('<')
                    .map_or_else(|| std::any::type_name::<T>(), |(name, _)| name)
            }

            type_name_without_generics::<OR>()
                != type_name_without_generics::<
                    crate::utils::subscribe_with_auto_dispose_on_termination::AutoDisposeOnTerminationObserver<
                        (),
                        (),
                    >,
                >()
        },
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. Using subscribe_with_context_bound_disposal handles \"auto dispose on termination\"."
    );
}
