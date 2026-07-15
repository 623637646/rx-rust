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
    Disposal<'or, D>, // TODO: Rename?
    ChainDisposal<BoxedDisposal<'or>, D>,
    where D: Disposable
);

fn default_model_mapper<M>(_: &mut M) {}

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
    subscribe_with_context_map_model(observer, model, builder, default_model_mapper)
}

/// `model_mapper` runs while the context state is locked. It must only move the data needed
/// after stopping out of the model; it must not call external APIs or panic.
pub fn subscribe_with_context_map_model<'or, T, E, OR, D, M, M1, F>(
    observer: OR,
    model: M,
    builder: F,
    model_mapper: fn(&mut M) -> M1,
) -> Subscription<Disposal<'or, D>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable,
    M: MaybeSend + 'or,
    M1: MaybeSend + 'or,
    F: FnOnce(Context<T, E, OR, M, (), M1>) -> Subscription<D>,
{
    assert_observer::<OR>();
    let state = Shared::new(Mutable::new(State::Idle {
        observer,
        model,
        sub: Subscription::default(),
    }));
    let context = Context {
        state: state.clone(),
        model_mapper,
    };
    let disposable = ContextDisposable {
        state,
        model_mapper,
    };
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
    F: FnOnce(Context<T, E, OR, M, D, ()>) -> Subscription<D>,
{
    subscribe_with_context_bound_disposal_map_model(observer, model, builder, default_model_mapper)
}

/// `model_mapper` runs while the context state is locked. It must only move the data needed
/// after stopping out of the model; it must not call external APIs or panic.
pub fn subscribe_with_context_bound_disposal_map_model<'or, T, E, OR, D, M, M1, F>(
    observer: OR,
    model: M,
    builder: F,
    model_mapper: fn(&mut M) -> M1,
) -> Subscription<BoundDisposal<'or>>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: MaybeSend + 'or,
    D: Disposable + MaybeSend + 'or,
    M: MaybeSend + 'or,
    M1: MaybeSend + 'or,
    F: FnOnce(Context<T, E, OR, M, D, M1>) -> Subscription<D>,
{
    assert_observer::<OR>();
    let state = Shared::new(Mutable::new(State::Subscribing { observer, model }));
    let context = Context {
        state: state.clone(),
        model_mapper,
    };
    let disposable = ContextDisposable {
        state: state.clone(),
        model_mapper,
    };
    let sub = builder(context);
    let sub_to_drop = state.lock_mut(|mut lock| {
        let state = std::mem::replace(&mut *lock, State::Placeholder);
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
            State::Stopped(model_in_stop) => {
                *lock = State::Stopped(model_in_stop);
                Some(sub)
            }
            State::Placeholder => unreachable!(),
        }
    });
    drop(sub_to_drop); // Drop outside the lock to avoid potential deadlock
    disposable.into_boxed().into_subscription()
}

#[derive(Educe)]
#[educe(Debug)]
enum State<T, E, OR, M, D: Disposable, M1> {
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
    Stopped(M1), // Unsubscribed or disposed
    Placeholder,
}

type SharedState<T, E, OR, M, D, M1> = Shared<Mutable<State<T, E, OR, M, D, M1>>>;
type WeakSharedState<T, E, OR, M, D, M1> = WeakShared<Mutable<State<T, E, OR, M, D, M1>>>;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Context<T, E, OR, M, D: Disposable = (), M1 = ()> {
    state: SharedState<T, E, OR, M, D, M1>,
    model_mapper: fn(&mut M) -> M1,
}

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

impl<T, E, OR, M, D, M1> Context<T, E, OR, M, D, M1>
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
        self.state.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Subscribing { model, .. } => model,
                State::Idle { model, .. } => model,
                State::Processing { model, .. } => model,
                State::Stopped(_) => {
                    drop(lock);
                    return Result::Err(Error::Stopped);
                }
                State::Placeholder => unreachable!(),
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

    pub fn modify_model_or_model_in_stop<A, R>(
        &self,
        callback: impl FnOnce(Result<&mut M, &mut M1>) -> ModificationResult<T, E, A, R>,
    ) -> R {
        self.state.lock_mut(|mut lock| {
            let model = match &mut *lock {
                State::Subscribing { model, .. } => Ok(model),
                State::Idle { model, .. } => Ok(model),
                State::Processing { model, .. } => Ok(model),
                State::Stopped(model_in_stop) => Err(model_in_stop),
                State::Placeholder => unreachable!(),
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
            result
        })
    }

    pub fn send_next(&self, value: T) {
        self.state
            .lock_mut(|lock| self.sending_impl(EventGroup::Next(value), lock))
    }

    pub fn send_termination(&self, termination: Termination<E>) {
        self.state
            .lock_mut(|lock| self.sending_impl(EventGroup::Termination(termination), lock))
    }

    pub fn send_next_and_termination(&self, next: T, termination: Termination<E>) {
        self.state.lock_mut(|lock| {
            self.sending_impl(EventGroup::NextAndTermination(next, termination), lock)
        })
    }

    pub fn send_events(&self, events: EventGroup<T, E>) {
        self.state.lock_mut(|lock| self.sending_impl(events, lock))
    }

    fn sending_impl(
        &self,
        events: EventGroup<T, E>,
        mut lock: MutGuard<'_, State<T, E, OR, M, D, M1>>,
    ) {
        match &mut *lock {
            State::Idle { .. } | State::Subscribing { .. } => {
                let (first_next, next_values, termination) = match events {
                    EventGroup::Next(next) => (Some(next), VecDeque::new(), None),
                    EventGroup::Termination(termination) => {
                        (None, VecDeque::new(), Some(termination))
                    }
                    EventGroup::NextAndTermination(next, termination) => {
                        (Some(next), VecDeque::new(), Some(termination))
                    }
                    EventGroup::Nexts(items) => {
                        let mut next_values = VecDeque::from(items);
                        let first_next = next_values.pop_front();
                        (first_next, next_values, None)
                    }
                    EventGroup::NextsAndTermination(items, termination) => {
                        let mut next_values = VecDeque::from(items);
                        let first_next = next_values.pop_front();
                        (first_next, next_values, Some(termination))
                    }
                };
                if first_next.is_none() && termination.is_none() {
                    // Empty `Nexts` is a no-op, consistent with the `Processing` state.
                    drop(lock);
                    return;
                }
                let idle_or_subscribing = std::mem::replace(&mut *lock, State::Placeholder);
                let (mut observer, model, sub) = match idle_or_subscribing {
                    State::Subscribing { observer, model } => (observer, model, None),
                    State::Idle {
                        observer,
                        model,
                        sub,
                    } => (observer, model, Some(sub)),
                    _ => {
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
                if let Some(first_next) = first_next {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        model_mapper: &self.model_mapper,
                    };
                    observer.on_next(first_next);
                    drop(stop_on_panic);
                }
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
            State::Stopped(_) => {
                drop(lock);
            }
            State::Placeholder => unreachable!(),
        };
    }

    pub fn downgrade(&self) -> WeakContext<T, E, OR, M, D, M1> {
        WeakContext {
            state: Shared::downgrade(&self.state),
            model_mapper: self.model_mapper,
        }
    }

    /// Delivers queued events to the observer one at a time, reacquiring the lock
    /// between events. Every observer call happens outside the lock, and disposal
    /// (`Stopped`) takes effect between any two events — including within a batch
    /// queued via `EventGroup::Nexts`.
    fn deliver_pending_events(&self, mut observer: OR) {
        loop {
            let step = self.state.lock_mut(|mut lock| match &mut *lock {
                State::Idle { .. } | State::Subscribing { .. } | State::Placeholder => {
                    drop(lock);
                    unreachable!()
                }
                State::Processing { next_values, .. } => {
                    if let Some(value) = next_values.pop_front() {
                        drop(lock);
                        return DeliveryStep::Next(observer, value);
                    }
                    let old_state = std::mem::replace(&mut *lock, State::Placeholder);
                    let State::Processing {
                        termination,
                        mut model,
                        sub,
                        ..
                    } = old_state
                    else {
                        drop(lock);
                        unreachable!()
                    };
                    match termination {
                        Some(termination) => {
                            let model_in_stop = (self.model_mapper)(&mut model);
                            *lock = State::Stopped(model_in_stop);
                            // The state stays `Stopped`: the subscription is over.
                            drop(lock);
                            drop(model); // Drop the remaining model outside the lock.
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
                State::Stopped(_) => {
                    drop(lock);
                    DeliveryStep::Stopped(observer)
                }
            });
            match step {
                DeliveryStep::Next(mut obs, value) => {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        model_mapper: &self.model_mapper,
                    };
                    obs.on_next(value);
                    drop(stop_on_panic);
                    observer = obs;
                }
                DeliveryStep::Terminate(obs, termination, sub_to_drop) => {
                    let stop_on_panic = StopOnPanic {
                        state: &self.state,
                        model_mapper: &self.model_mapper,
                    };
                    obs.on_termination(termination);
                    drop(stop_on_panic);
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
struct StopOnPanic<'a, T, E, OR, M, D: Disposable, M1> {
    state: &'a SharedState<T, E, OR, M, D, M1>,
    model_mapper: &'a fn(&mut M) -> M1,
}

impl<T, E, OR, M, D: Disposable, M1> Drop for StopOnPanic<'_, T, E, OR, M, D, M1> {
    fn drop(&mut self) {
        if std::thread::panicking() {
            dispose_state(self.state, self.model_mapper);
        }
    }
}

struct ContextDisposable<T, E, OR, M, D: Disposable, M1> {
    state: SharedState<T, E, OR, M, D, M1>,
    model_mapper: fn(&mut M) -> M1,
}

impl<T, E, OR, M, D: Disposable, M1> Disposable for ContextDisposable<T, E, OR, M, D, M1> {
    fn dispose(self) {
        dispose_state(&self.state, &self.model_mapper);
    }
}

fn dispose_state<T, E, OR, M, D: Disposable, M1>(
    state: &SharedState<T, E, OR, M, D, M1>,
    model_mapper: &fn(&mut M) -> M1,
) {
    struct DropOutside<T, E, OR, D: Disposable> {
        _observer: Option<OR>,
        _sub: Option<Subscription<D>>,
        _events: Option<(VecDeque<T>, Option<Termination<E>>)>,
    }

    let _drop_outside = state.lock_mut(|mut lock| {
        if matches!(&*lock, State::Stopped(_)) {
            return None;
        }

        let state = std::mem::replace(&mut *lock, State::Placeholder);
        let (mut model, drop_outside) = match state {
            State::Subscribing { observer, model } => {
                let drop_outside = DropOutside {
                    _observer: Some(observer),
                    _sub: None,
                    _events: None,
                };
                (model, drop_outside)
            }
            State::Idle {
                observer,
                model,
                sub,
            } => {
                let drop_outside = DropOutside {
                    _observer: Some(observer),
                    _sub: Some(sub),
                    _events: None,
                };
                (model, drop_outside)
            }
            State::Processing {
                next_values,
                termination,
                model,
                sub,
            } => {
                let drop_outside = DropOutside {
                    _observer: None,
                    _sub: sub,
                    _events: Some((next_values, termination)),
                };
                (model, drop_outside)
            }
            State::Stopped(_) | State::Placeholder => unreachable!(),
        };

        let model_in_stop = model_mapper(&mut model);
        *lock = State::Stopped(model_in_stop);

        Some((drop_outside, model))
    });
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WeakContext<T, E, OR, M, D: Disposable = (), M1 = ()> {
    state: WeakSharedState<T, E, OR, M, D, M1>,
    model_mapper: fn(&mut M) -> M1,
}

impl<T, E, OR, M, D: Disposable, M1> WeakContext<T, E, OR, M, D, M1> {
    pub fn upgrade(&self) -> Option<Context<T, E, OR, M, D, M1>> {
        self.state.upgrade().map(|state| Context {
            state,
            model_mapper: self.model_mapper,
        })
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
