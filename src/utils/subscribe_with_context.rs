use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
    },
    observable::Subscription,
    observer::{Observer, Termination},
    utils::{
        pending_events::{EventBatch, PendingEvents},
        types::{MaybeSend, MutGuard, Mutable, MutableHelper, Shared, WeakShared},
    },
};
use educe::Educe;

delegate_disposal!(
    Disposal<'or, D>,
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

/// Creates a subscription backed by a shared, serialized context containing the downstream
/// observer and a mutable model.
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
    F: FnOnce(SubscriptionContext<T, E, OR, M>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let state = Shared::new(Mutable::new(State::Idle {
        observer,
        model,
        subscription: Subscription::default(),
    }));
    let context = SubscriptionContext {
        state: state.clone(),
    };
    let disposable = SubscriptionContextDisposal { state };
    let subscription = builder(context);
    subscription.preceded_by(disposable.into_boxed()).map_into()
}

/// Type-erased disposal returned when the context owns the source subscription.
pub type BoundSubscriptionDisposal<'or> = BoxedDisposal<'or>;

/// Creates a context subscription whose source subscription is owned by the context.
///
/// Owning the source subscription lets the context dispose it automatically when the observer
/// terminates, including when termination occurs synchronously while `builder` is running.
pub fn subscribe_with_context_bound_subscription<'or, T, E, OR, D, M, F>(
    observer: OR,
    model: M,
    builder: F,
) -> Subscription<BoundSubscriptionDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
    F: FnOnce(SubscriptionContext<T, E, OR, M, D>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let state = Shared::new(Mutable::new(State::Subscribing { observer, model }));
    let context = SubscriptionContext {
        state: state.clone(),
    };
    let disposable = SubscriptionContextDisposal {
        state: state.clone(),
    };
    let subscription = builder(context);
    let subscription_to_drop = state.lock_mut(|mut lock| {
        let state = std::mem::replace(&mut *lock, State::Placeholder);
        match state {
            State::Subscribing { observer, model } => {
                *lock = State::Idle {
                    observer,
                    model,
                    subscription,
                };
                None
            }
            State::Idle { .. } => unreachable!(),
            State::Delivering {
                pending,
                model,
                subscription: None,
            } => {
                *lock = State::Delivering {
                    pending,
                    model,
                    subscription: Some(subscription),
                };
                None
            }
            State::Delivering {
                subscription: Some(_),
                ..
            } => unreachable!(),
            State::Stopped => {
                *lock = State::Stopped;
                Some(subscription)
            }
            State::Placeholder => unreachable!(),
        }
    });
    drop(subscription_to_drop); // Drop outside the lock to avoid potential deadlock
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
        subscription: Subscription<D>,
    },
    Delivering {
        pending: PendingEvents<T, E>,
        model: M,
        subscription: Option<Subscription<D>>, // Back to Subscribing if None, otherwise to Idle.
    },
    Stopped,
    Placeholder,
}

type SharedState<T, E, OR, M, D> = Shared<Mutable<State<T, E, OR, M, D>>>;
type WeakSharedState<T, E, OR, M, D> = WeakShared<Mutable<State<T, E, OR, M, D>>>;

/// Shared state used by operator observers to serialize model updates and downstream events.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    state: SharedState<T, E, OR, M, D>,
}

pub struct DropUndecided;
pub struct DropDecided<T>(Option<T>);

/// Effects produced by a model update while the subscription context is locked.
#[derive(Educe)]
#[educe(Debug)]
pub struct ModelUpdate<T, E, R = (), DO = DropUndecided, const EVENTS_DECIDED: bool = false> {
    events: Option<EventBatch<T, E>>,
    drop_outside: DO,
    result: R,
}

impl<T, E, R> ModelUpdate<T, E, R> {
    pub fn new(result: R) -> Self {
        Self {
            events: None,
            drop_outside: DropUndecided,
            result,
        }
    }
}

impl<T, E> ModelUpdate<T, E> {
    pub fn empty() -> Self {
        Self::new(())
    }
}

impl<T, E, R, const EVENTS_DECIDED: bool> ModelUpdate<T, E, R, DropUndecided, EVENTS_DECIDED> {
    pub fn with_drop_outside<DO>(
        self,
        drop_outside: DO,
    ) -> ModelUpdate<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        ModelUpdate {
            events: self.events,
            drop_outside: DropDecided(Some(drop_outside)),
            result: self.result,
        }
    }

    pub fn without_drop_outside<DO>(self) -> ModelUpdate<T, E, R, DropDecided<DO>, EVENTS_DECIDED> {
        ModelUpdate {
            events: self.events,
            drop_outside: DropDecided(None),
            result: self.result,
        }
    }
}

impl<T, E, R, DO> ModelUpdate<T, E, R, DO, false> {
    pub fn with_next_event(self, next: T) -> ModelUpdate<T, E, R, DO, true> {
        ModelUpdate {
            events: Some(EventBatch::Next(next)),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    pub fn with_termination_event(
        self,
        termination: Termination<E>,
    ) -> ModelUpdate<T, E, R, DO, true> {
        ModelUpdate {
            events: Some(EventBatch::Termination(termination)),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    pub fn with_next_and_termination_events(
        self,
        next: T,
        termination: Termination<E>,
    ) -> ModelUpdate<T, E, R, DO, true> {
        ModelUpdate {
            events: Some(EventBatch::NextAndTermination(next, termination)),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    pub fn with_events(self, events: EventBatch<T, E>) -> ModelUpdate<T, E, R, DO, true> {
        ModelUpdate {
            events: Some(events),
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }

    pub fn without_events(self) -> ModelUpdate<T, E, R, DO, true> {
        ModelUpdate {
            events: None,
            drop_outside: self.drop_outside,
            result: self.result,
        }
    }
}

/// Returned when an update requires the active model after the context has stopped.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub struct ContextStopped;

impl<T, E, OR, M, D> SubscriptionContext<T, E, OR, M, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Tries to update the active model while the context is locked.
    ///
    /// The callback must not call external APIs or drop values that can re-enter this context.
    /// Return such values through [`ModelUpdate::with_drop_outside`] instead.
    /// If the context has stopped, the callback is not invoked and [`ContextStopped`] is returned.
    pub fn try_update_model<R, DO, const EVENTS_DECIDED: bool>(
        &self,
        callback: impl FnOnce(&mut M) -> ModelUpdate<T, E, R, DO, EVENTS_DECIDED>,
    ) -> Result<R, ContextStopped> {
        // Keep the callback outside the closure so that, if the context is already stopped, its
        // captures are dropped only after the lock is released.
        let mut callback = Some(callback);
        self.state.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Subscribing { model, .. } => model,
                State::Idle { model, .. } => model,
                State::Delivering { model, .. } => model,
                State::Stopped => return Err(ContextStopped),
                State::Placeholder => unreachable!(),
            };
            let callback = callback
                .take()
                .expect("active model callback must only be called once");
            let ModelUpdate {
                events,
                drop_outside,
                result,
            } = callback(model);
            if let Some(events) = events {
                self.dispatch_events(events, lock);
            } else {
                drop(lock);
            }
            drop(drop_outside); // Drop outside the lock to avoid potential deadlock
            Ok(result)
        })
    }

    pub fn send_next(&self, value: T) {
        self.state
            .lock_mut(|lock| self.dispatch_events(EventBatch::Next(value), lock))
    }

    pub fn send_termination(&self, termination: Termination<E>) {
        self.state
            .lock_mut(|lock| self.dispatch_events(EventBatch::Termination(termination), lock))
    }

    pub fn send_next_and_termination(&self, next: T, termination: Termination<E>) {
        self.state.lock_mut(|lock| {
            self.dispatch_events(EventBatch::NextAndTermination(next, termination), lock)
        })
    }

    pub fn send_events(&self, events: EventBatch<T, E>) {
        self.state
            .lock_mut(|lock| self.dispatch_events(events, lock))
    }

    fn dispatch_events(
        &self,
        events: EventBatch<T, E>,
        mut lock: MutGuard<'_, State<T, E, OR, M, D>>,
    ) {
        match &mut *lock {
            state @ (State::Idle { .. } | State::Subscribing { .. }) => {
                let mut pending = PendingEvents::new();
                let rejected = pending.push_batch(events);
                debug_assert!(rejected.is_none(), "a new queue accepts every batch");
                // The first value is delivered directly, so it never enters the queue.
                let first_next = pending.pop_next();
                if first_next.is_none() && pending.is_empty() {
                    // Empty `NextBatch` is a no-op, consistent with the `Delivering` state.
                    drop(lock);
                    return;
                }
                let idle_or_subscribing = std::mem::replace(state, State::Placeholder);
                let (mut observer, model, subscription) = match idle_or_subscribing {
                    State::Subscribing { observer, model } => (observer, model, None),
                    State::Idle {
                        observer,
                        model,
                        subscription,
                    } => (observer, model, Some(subscription)),
                    _ => {
                        drop(lock);
                        unreachable!()
                    }
                };
                *lock = State::Delivering {
                    pending,
                    model,
                    subscription,
                };
                drop(lock); // Deliver events outside the lock to avoid potential deadlock
                if let Some(first_next) = first_next {
                    let stop_on_panic = StopOnPanic { state: &self.state };
                    observer.on_next(first_next);
                    drop(stop_on_panic);
                }
                self.deliver_pending_events(observer);
            }
            State::Delivering { pending, .. } => {
                // The events are rejected once the termination is queued, since it is the last
                // event of the stream.
                let rejected = pending.push_batch(events);
                drop(lock);
                drop(rejected); // Drop outside the lock to avoid potential deadlock
            }
            State::Stopped => {
                drop(lock);
            }
            State::Placeholder => unreachable!(),
        };
    }

    pub fn downgrade(&self) -> WeakSubscriptionContext<T, E, OR, M, D> {
        WeakSubscriptionContext {
            state: Shared::downgrade(&self.state),
        }
    }

    /// Delivers queued events to the observer one at a time, reacquiring the lock
    /// between events. Every observer call happens outside the lock, and disposal
    /// (`Stopped`) takes effect between any two events — including within a batch
    /// queued via `EventBatch::NextBatch`.
    fn deliver_pending_events(&self, mut observer: OR) {
        loop {
            let step = self.state.lock_mut(|mut lock| match &mut *lock {
                State::Idle { .. } | State::Subscribing { .. } | State::Placeholder => {
                    drop(lock);
                    unreachable!()
                }
                State::Delivering { pending, .. } => {
                    if let Some(value) = pending.pop_next() {
                        drop(lock);
                        return DeliveryStep::Next(observer, value);
                    }
                    let termination = pending.take_termination();
                    let old_state = std::mem::replace(&mut *lock, State::Placeholder);
                    let State::Delivering {
                        model,
                        subscription,
                        ..
                    } = old_state
                    else {
                        drop(lock);
                        unreachable!()
                    };
                    match termination {
                        Some(termination) => {
                            *lock = State::Stopped;
                            // The state stays `Stopped`: the subscription is over.
                            drop(lock);
                            drop(model); // Drop the remaining model outside the lock.
                            DeliveryStep::Terminate(observer, termination, subscription)
                        }
                        None => {
                            *lock = match subscription {
                                Some(subscription) => State::Idle {
                                    observer,
                                    model,
                                    subscription,
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
                    let stop_on_panic = StopOnPanic { state: &self.state };
                    obs.on_next(value);
                    drop(stop_on_panic);
                    observer = obs;
                }
                DeliveryStep::Terminate(obs, termination, subscription_to_drop) => {
                    let stop_on_panic = StopOnPanic { state: &self.state };
                    obs.on_termination(termination);
                    drop(stop_on_panic);
                    // Preserve termination-before-disposal ordering. If the callback panics,
                    // stack unwinding still drops the subscription.
                    drop(subscription_to_drop);
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
struct StopOnPanic<'a, T, E, OR, M, D: Disposable> {
    state: &'a SharedState<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> Drop for StopOnPanic<'_, T, E, OR, M, D> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            stop_state(self.state);
        }
    }
}

struct SubscriptionContextDisposal<T, E, OR, M, D: Disposable> {
    state: SharedState<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> Disposable for SubscriptionContextDisposal<T, E, OR, M, D> {
    fn dispose(self) {
        stop_state(&self.state);
    }
}

fn stop_state<T, E, OR, M, D: Disposable>(state: &SharedState<T, E, OR, M, D>) {
    struct DeferredDrop<T, E, OR, D: Disposable> {
        _observer: Option<OR>,
        _subscription: Option<Subscription<D>>,
        _events: Option<PendingEvents<T, E>>,
    }

    let _deferred_drop = state.lock_mut(|mut lock| {
        if matches!(&*lock, State::Stopped) {
            return None;
        }

        let state = std::mem::replace(&mut *lock, State::Placeholder);
        let (model, deferred_drop) = match state {
            State::Subscribing { observer, model } => {
                let deferred_drop = DeferredDrop {
                    _observer: Some(observer),
                    _subscription: None,
                    _events: None,
                };
                (model, deferred_drop)
            }
            State::Idle {
                observer,
                model,
                subscription,
            } => {
                let deferred_drop = DeferredDrop {
                    _observer: Some(observer),
                    _subscription: Some(subscription),
                    _events: None,
                };
                (model, deferred_drop)
            }
            State::Delivering {
                pending,
                model,
                subscription,
            } => {
                let deferred_drop = DeferredDrop {
                    _observer: None,
                    _subscription: subscription,
                    _events: Some(pending),
                };
                (model, deferred_drop)
            }
            State::Stopped | State::Placeholder => unreachable!(),
        };

        *lock = State::Stopped;

        Some((deferred_drop, model))
    });
}

/// A non-owning reference to a [`SubscriptionContext`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakSubscriptionContext<T, E, OR, M, D: Disposable = ()> {
    state: WeakSharedState<T, E, OR, M, D>,
}

impl<T, E, OR, M, D: Disposable> WeakSubscriptionContext<T, E, OR, M, D> {
    pub fn upgrade(&self) -> Option<SubscriptionContext<T, E, OR, M, D>> {
        self.state
            .upgrade()
            .map(|state| SubscriptionContext { state })
    }
}

fn debug_assert_observer_compatibility<OR>() {
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
        "Do not combine subscribe_with_auto_dispose_on_termination with a context subscription. Using subscribe_with_context_bound_subscription handles \"auto dispose on termination\"."
    );
}
