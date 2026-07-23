use crate::{
    delegate_disposal,
    disposable::{
        Disposable, DisposableExt, boxed_disposal::BoxedDisposal, chain_disposal::ChainDisposal,
    },
    observable::Subscription,
    observer::{Observer, Termination},
    utils::types::{MaybeSend, MutGuard, Mutable, MutableHelper, Shared, WeakShared},
};
use educe::Educe;
use std::collections::VecDeque;

delegate_disposal!(
    Disposal<'or, D>,
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

fn retain_nothing_on_stop<M>(_: &mut M) {}

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
    subscribe_with_context_retain_state_on_stop(observer, model, builder, retain_nothing_on_stop)
}

/// Creates a context subscription that retains selected model state after stopping.
///
/// `retain_state_on_stop` runs while the context is locked during the transition to `Stopped`.
/// It must only move out the state that needs to remain available; it must not call external
/// APIs, drop values that can re-enter the context, or panic.
pub fn subscribe_with_context_retain_state_on_stop<'or, T, E, OR, D, M, S, F>(
    observer: OR,
    model: M,
    builder: F,
    retain_state_on_stop: fn(&mut M) -> S,
) -> Subscription<Disposal<'or, D>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable,
    M: MaybeSend + 'or,
    S: MaybeSend + 'or,
    F: FnOnce(SubscriptionContext<T, E, OR, M, (), S>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let state = Shared::new(Mutable::new(State::Idle {
        observer,
        model,
        subscription: Subscription::default(),
    }));
    let context = SubscriptionContext {
        state: state.clone(),
        retain_state_on_stop,
    };
    let disposable = SubscriptionContextDisposal {
        state,
        retain_state_on_stop,
    };
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
    F: FnOnce(SubscriptionContext<T, E, OR, M, D, ()>) -> Subscription<D>,
{
    subscribe_with_context_bound_subscription_retain_state_on_stop(
        observer,
        model,
        builder,
        retain_nothing_on_stop,
    )
}

/// Creates a context-owned subscription that retains selected model state after stopping.
///
/// `retain_state_on_stop` runs while the context is locked during the transition to `Stopped`.
/// It must only move out the state that needs to remain available; it must not call external
/// APIs, drop values that can re-enter the context, or panic.
pub fn subscribe_with_context_bound_subscription_retain_state_on_stop<'or, T, E, OR, D, M, S, F>(
    observer: OR,
    model: M,
    builder: F,
    retain_state_on_stop: fn(&mut M) -> S,
) -> Subscription<BoundSubscriptionDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
    S: MaybeSend + 'or,
    F: FnOnce(SubscriptionContext<T, E, OR, M, D, S>) -> Subscription<D>,
{
    debug_assert_observer_compatibility::<OR>();
    let state = Shared::new(Mutable::new(State::Subscribing { observer, model }));
    let context = SubscriptionContext {
        state: state.clone(),
        retain_state_on_stop,
    };
    let disposable = SubscriptionContextDisposal {
        state: state.clone(),
        retain_state_on_stop,
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
                next_values,
                termination,
                model,
                subscription: None,
            } => {
                *lock = State::Delivering {
                    next_values,
                    termination,
                    model,
                    subscription: Some(subscription),
                };
                None
            }
            State::Delivering {
                subscription: Some(_),
                ..
            } => unreachable!(),
            State::Stopped { retained_state } => {
                *lock = State::Stopped { retained_state };
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
enum State<T, E, OR, M, D: Disposable, S> {
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
        next_values: VecDeque<T>,
        termination: Option<Termination<E>>,
        model: M,
        subscription: Option<Subscription<D>>, // Back to Subscribing if None, otherwise to Idle.
    },
    Stopped {
        retained_state: S,
    },
    Placeholder,
}

type SharedState<T, E, OR, M, D, S> = Shared<Mutable<State<T, E, OR, M, D, S>>>;
type WeakSharedState<T, E, OR, M, D, S> = WeakShared<Mutable<State<T, E, OR, M, D, S>>>;

/// Shared state used by operator observers to serialize model updates and downstream events.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscriptionContext<T, E, OR, M, D: Disposable = (), S = ()> {
    state: SharedState<T, E, OR, M, D, S>,
    retain_state_on_stop: fn(&mut M) -> S,
}

/// One atomic batch of downstream observer events.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum EventBatch<T, E> {
    Next(T),
    Termination(Termination<E>),
    NextAndTermination(T, Termination<E>),
    NextBatch(Vec<T>),
    NextBatchAndTermination(Vec<T>, Termination<E>),
}

/// Effects produced by a model update while the subscription context is locked.
#[derive(Educe)]
#[educe(Debug)]
pub struct ModelUpdate<T, E, A, R> {
    events: Option<EventBatch<T, E>>,
    drop_outside: Option<A>,
    result: R,
}

impl<T, E, A, R> ModelUpdate<T, E, A, R> {
    pub fn new(result: R) -> Self {
        Self {
            events: None,
            drop_outside: None,
            result,
        }
    }

    pub fn send_next(self, next: T) -> Self {
        self.assert_no_events_set();
        Self {
            events: Some(EventBatch::Next(next)),
            ..self
        }
    }

    pub fn send_termination(self, termination: Termination<E>) -> Self {
        self.assert_no_events_set();
        Self {
            events: Some(EventBatch::Termination(termination)),
            ..self
        }
    }

    pub fn send_next_and_termination(self, next: T, termination: Termination<E>) -> Self {
        self.assert_no_events_set();
        Self {
            events: Some(EventBatch::NextAndTermination(next, termination)),
            ..self
        }
    }

    pub fn send_events(self, events: EventBatch<T, E>) -> Self {
        self.assert_no_events_set();
        Self {
            events: Some(events),
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
            self.events.is_none(),
            "send_events is already set; calling a send_* method again would silently overwrite the previous events"
        );
    }
}

impl<T, E> ModelUpdate<T, E, (), ()> {
    pub fn new_empty() -> Self {
        Self {
            events: None,
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_next(next: T) -> Self {
        Self {
            events: Some(EventBatch::Next(next)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_termination(termination: Termination<E>) -> Self {
        Self {
            events: Some(EventBatch::Termination(termination)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_next_and_termination(next: T, termination: Termination<E>) -> Self {
        Self {
            events: Some(EventBatch::NextAndTermination(next, termination)),
            drop_outside: None,
            result: (),
        }
    }

    pub fn new_send_events(events: EventBatch<T, E>) -> Self {
        Self {
            events: Some(events),
            drop_outside: None,
            result: (),
        }
    }
}

impl<T, E, A> ModelUpdate<T, E, A, ()> {
    pub fn new_without_result() -> Self {
        Self {
            events: None,
            drop_outside: None,
            result: (),
        }
    }
}

impl<T, E, R> ModelUpdate<T, E, (), R> {
    pub fn ignore_drop_outside(self) -> Self {
        self
    }
}

/// Returned when an update requires the active model after the context has stopped.
#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub struct ContextStopped;

/// A mutable view of the model storage for either phase of a subscription context.
pub enum ModelState<'a, M, S> {
    Active(&'a mut M),
    Stopped(&'a mut S),
}

impl<T, E, OR, M, D, S> SubscriptionContext<T, E, OR, M, D, S>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    /// Tries to update the active model while the context is locked.
    ///
    /// The callback must not call external APIs or drop values that can re-enter this context.
    /// Return such values through [`ModelUpdate::drop_outside`] instead.
    /// If the context has stopped, the callback is not invoked and [`ContextStopped`] is returned.
    pub fn try_update_model<A, R>(
        &self,
        callback: impl FnOnce(&mut M) -> ModelUpdate<T, E, A, R>,
    ) -> Result<R, ContextStopped> {
        // Keep the callback outside the closure so that, if the context is already stopped, its
        // captures are dropped only after `update_model_or_retained_state` has released the lock.
        let mut callback = Some(callback);
        self.update_model_or_retained_state(|model| match model {
            ModelState::Active(model) => {
                let callback = callback
                    .take()
                    .expect("active model callback must only be called once");
                let ModelUpdate {
                    events,
                    drop_outside,
                    result,
                } = callback(model);
                ModelUpdate {
                    events,
                    drop_outside,
                    result: Ok(result),
                }
            }
            ModelState::Stopped(_) => ModelUpdate::new(Err(ContextStopped)),
        })
    }

    /// Updates either the active model or the state retained after stopping.
    ///
    /// The callback runs while the context is locked and follows the same restrictions as
    /// [`SubscriptionContext::try_update_model`].
    pub fn update_model_or_retained_state<A, R>(
        &self,
        callback: impl FnOnce(ModelState<'_, M, S>) -> ModelUpdate<T, E, A, R>,
    ) -> R {
        self.state.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Subscribing { model, .. } => ModelState::Active(model),
                State::Idle { model, .. } => ModelState::Active(model),
                State::Delivering { model, .. } => ModelState::Active(model),
                State::Stopped { retained_state } => ModelState::Stopped(retained_state),
                State::Placeholder => unreachable!(),
            };
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
            result
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
        mut lock: MutGuard<'_, State<T, E, OR, M, D, S>>,
    ) {
        match &mut *lock {
            State::Idle { .. } | State::Subscribing { .. } => {
                let (first_next, next_values, termination) = match events {
                    EventBatch::Next(next) => (Some(next), VecDeque::new(), None),
                    EventBatch::Termination(termination) => {
                        (None, VecDeque::new(), Some(termination))
                    }
                    EventBatch::NextAndTermination(next, termination) => {
                        (Some(next), VecDeque::new(), Some(termination))
                    }
                    EventBatch::NextBatch(items) => {
                        let mut next_values = VecDeque::from(items);
                        let first_next = next_values.pop_front();
                        (first_next, next_values, None)
                    }
                    EventBatch::NextBatchAndTermination(items, termination) => {
                        let mut next_values = VecDeque::from(items);
                        let first_next = next_values.pop_front();
                        (first_next, next_values, Some(termination))
                    }
                };
                if first_next.is_none() && termination.is_none() {
                    // Empty `NextBatch` is a no-op, consistent with the `Delivering` state.
                    drop(lock);
                    return;
                }
                let idle_or_subscribing = std::mem::replace(&mut *lock, State::Placeholder);
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
                    next_values,
                    termination,
                    model,
                    subscription,
                };
                drop(lock); // Deliver events outside the lock to avoid potential deadlock
                if let Some(first_next) = first_next {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        retain_state_on_stop: &self.retain_state_on_stop,
                    };
                    observer.on_next(first_next);
                    drop(stop_on_panic);
                }
                self.deliver_pending_events(observer);
            }
            State::Delivering {
                next_values,
                termination,
                ..
            } => {
                if termination.is_none() {
                    match events {
                        EventBatch::Next(value) => next_values.push_back(value),
                        EventBatch::Termination(tn) => {
                            *termination = Some(tn);
                        }
                        EventBatch::NextAndTermination(value, tn) => {
                            next_values.push_back(value);
                            *termination = Some(tn);
                        }
                        EventBatch::NextBatch(items) => next_values.extend(items),
                        EventBatch::NextBatchAndTermination(items, tn) => {
                            next_values.extend(items);
                            *termination = Some(tn);
                        }
                    };
                }
                drop(lock);
            }
            State::Stopped { .. } => {
                drop(lock);
            }
            State::Placeholder => unreachable!(),
        };
    }

    pub fn downgrade(&self) -> WeakSubscriptionContext<T, E, OR, M, D, S> {
        WeakSubscriptionContext {
            state: Shared::downgrade(&self.state),
            retain_state_on_stop: self.retain_state_on_stop,
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
                State::Delivering { next_values, .. } => {
                    if let Some(value) = next_values.pop_front() {
                        drop(lock);
                        return DeliveryStep::Next(observer, value);
                    }
                    let old_state = std::mem::replace(&mut *lock, State::Placeholder);
                    let State::Delivering {
                        termination,
                        mut model,
                        subscription,
                        ..
                    } = old_state
                    else {
                        drop(lock);
                        unreachable!()
                    };
                    match termination {
                        Some(termination) => {
                            let retained_state = (self.retain_state_on_stop)(&mut model);
                            *lock = State::Stopped { retained_state };
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
                State::Stopped { .. } => {
                    drop(lock);
                    DeliveryStep::Stopped(observer)
                }
            });
            match step {
                DeliveryStep::Next(mut obs, value) => {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        retain_state_on_stop: &self.retain_state_on_stop,
                    };
                    obs.on_next(value);
                    drop(stop_on_panic);
                    observer = obs;
                }
                DeliveryStep::Terminate(obs, termination, subscription_to_drop) => {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        retain_state_on_stop: &self.retain_state_on_stop,
                    };
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
struct StopOnPanic<'a, T, E, OR, M, D: Disposable, S> {
    state: &'a SharedState<T, E, OR, M, D, S>,
    retain_state_on_stop: &'a fn(&mut M) -> S,
}

impl<T, E, OR, M, D: Disposable, S> Drop for StopOnPanic<'_, T, E, OR, M, D, S> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            stop_state(self.state, self.retain_state_on_stop);
        }
    }
}

struct SubscriptionContextDisposal<T, E, OR, M, D: Disposable, S> {
    state: SharedState<T, E, OR, M, D, S>,
    retain_state_on_stop: fn(&mut M) -> S,
}

impl<T, E, OR, M, D: Disposable, S> Disposable for SubscriptionContextDisposal<T, E, OR, M, D, S> {
    fn dispose(self) {
        stop_state(&self.state, &self.retain_state_on_stop);
    }
}

fn stop_state<T, E, OR, M, D: Disposable, S>(
    state: &SharedState<T, E, OR, M, D, S>,
    retain_state_on_stop: &fn(&mut M) -> S,
) {
    struct DeferredDrop<T, E, OR, D: Disposable> {
        _observer: Option<OR>,
        _subscription: Option<Subscription<D>>,
        _events: Option<(VecDeque<T>, Option<Termination<E>>)>,
    }

    let _deferred_drop = state.lock_mut(|mut lock| {
        if matches!(&*lock, State::Stopped { .. }) {
            return None;
        }

        let state = std::mem::replace(&mut *lock, State::Placeholder);
        let (mut model, deferred_drop) = match state {
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
                next_values,
                termination,
                model,
                subscription,
            } => {
                let deferred_drop = DeferredDrop {
                    _observer: None,
                    _subscription: subscription,
                    _events: Some((next_values, termination)),
                };
                (model, deferred_drop)
            }
            State::Stopped { .. } | State::Placeholder => unreachable!(),
        };

        let retained_state = retain_state_on_stop(&mut model);
        *lock = State::Stopped { retained_state };

        Some((deferred_drop, model))
    });
}

/// A non-owning reference to a [`SubscriptionContext`].
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakSubscriptionContext<T, E, OR, M, D: Disposable = (), S = ()> {
    state: WeakSharedState<T, E, OR, M, D, S>,
    retain_state_on_stop: fn(&mut M) -> S,
}

impl<T, E, OR, M, D: Disposable, S> WeakSubscriptionContext<T, E, OR, M, D, S> {
    pub fn upgrade(&self) -> Option<SubscriptionContext<T, E, OR, M, D, S>> {
        self.state.upgrade().map(|state| SubscriptionContext {
            state,
            retain_state_on_stop: self.retain_state_on_stop,
        })
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
