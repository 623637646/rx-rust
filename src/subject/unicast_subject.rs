//! A single-consumer pipe between an [`Observer`] and an [`Observable`].
//!
//! Unlike the multicast subjects, a unicast subject serves exactly one observer, which is what
//! lets it buffer the events that arrive before the subscription instead of dropping them, and
//! lets it move each value to that observer instead of cloning it.
//!
//! The two ends are separate values: [`UnicastSender`] is the [`Observer`] and
//! [`UnicastObservable`] is the [`Observable`]. Neither is [`Clone`], so the type system, rather
//! than a runtime check, is what guarantees that the pipe is fed by one sender and consumed by one
//! observer. That is also why a unicast subject does not implement the [`Subject`] trait, whose
//! implementors are both an [`Observable`] and an [`Observer`] at the same time, and why it cannot
//! be used to multicast a source through [`ObservableExt::multicast`].
//!
//! [`Subject`]: crate::subject::Subject
//! [`ObservableExt::multicast`]: crate::observable::ObservableExt::multicast

use crate::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Event, Observer, Termination, boxed_observer::BoxedObserver},
    safe_lock,
    utils::{
        pending_events::PendingEvents,
        types::{MaybeSend, Mutable, MutableHelper, Shared},
    },
};
use educe::Educe;

/// Creates a unicast subject, giving back its sending and its observable end.
///
/// Values sent before the subscription are buffered and replayed to the observer when it
/// subscribes, followed by the termination if the sender already terminated. Once the observer is
/// gone, by disposing its subscription or by dropping the [`UnicastObservable`] without
/// subscribing, later events are dropped.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::unicast_subject::unicast_subject,
/// };
/// use std::{
///     convert::Infallible,
///     sync::{Arc, Mutex},
/// };
///
/// let (mut sender, observable) = unicast_subject::<i32, Infallible>();
///
/// // The values sent before the subscription are buffered instead of being dropped.
/// sender.on_next(111);
/// sender.on_next(222);
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let values_observer = Arc::clone(&values);
/// let subscription = observable.subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     |_| {},
/// );
/// assert_eq!(&*values.lock().unwrap(), &[111, 222]);
///
/// sender.on_next(333);
/// assert_eq!(&*values.lock().unwrap(), &[111, 222, 333]);
///
/// sender.on_termination(Termination::Completed);
/// drop(subscription);
/// ```
pub fn unicast_subject<'or, T, E>() -> (UnicastSender<'or, T, E>, UnicastObservable<'or, T, E>) {
    new_pair(PendingEvents::new())
}

/// Creates a unicast subject whose buffer is pre-allocated for `capacity` values.
///
/// The capacity is only a hint: the buffer still grows as needed. See [`unicast_subject`] for the
/// behavior of the returned pair.
pub fn unicast_subject_with_capacity<'or, T, E>(
    capacity: usize,
) -> (UnicastSender<'or, T, E>, UnicastObservable<'or, T, E>) {
    new_pair(PendingEvents::with_capacity(capacity))
}

fn new_pair<'or, T, E>(
    pending: PendingEvents<T, E>,
) -> (UnicastSender<'or, T, E>, UnicastObservable<'or, T, E>) {
    let state = Shared::new(Mutable::new(State::Buffering(pending)));
    (UnicastSender(state.clone()), UnicastObservable(Some(state)))
}

#[derive(Educe)]
#[educe(Debug)]
enum State<'or, T, E> {
    /// No observer has subscribed yet: the events wait in the queue.
    Buffering(PendingEvents<T, E>),
    /// The observer has subscribed and is idle.
    Attached(BoxedObserver<'or, T, E>),
    /// The observer is being delivered to outside the lock, so it is not held here. The events
    /// that arrive while delivering wait in the queue.
    Delivering(PendingEvents<T, E>),
    /// The observer is gone, either because it was terminated or because the subscription was
    /// disposed. Every later event is dropped.
    Closed,
}

type SharedState<'or, T, E> = Shared<Mutable<State<'or, T, E>>>;

/// The sending end of a unicast subject. See [`unicast_subject`].
///
/// Dropping the sender without terminating it closes the pipe, which drops the observer without
/// notifying it: no event can reach it anymore, because the sender was the only way in, but a
/// producer that gave up halfway has not completed anything either.
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastSender<'or, T, E>(SharedState<'or, T, E>);

impl<T, E> Drop for UnicastSender<'_, T, E> {
    fn drop(&mut self) {
        // Terminating the pipe consumes the sender, so this also runs right after the last event
        // was queued. That event still has to reach the observer, whether it is waiting in the
        // queue for a late subscriber or for the delivery that is running.
        let previous_state = self.0.lock_mut(|mut lock| match &mut *lock {
            State::Buffering(pending) | State::Delivering(pending) if pending.is_terminated() => {
                None
            }
            state => Some(std::mem::replace(state, State::Closed)),
        });
        drop(previous_state); // Drop outside the lock to avoid potential deadlock
    }
}

impl<T, E> UnicastSender<'_, T, E> {
    /// Returns whether the observer is gone, which happens when its subscription is disposed or
    /// when the [`UnicastObservable`] is dropped without being subscribed to.
    ///
    /// Every later event is dropped, so a producer can use this to stop producing.
    pub fn is_disposed(&self) -> bool {
        // The sender is consumed by terminating the pipe, so a closed pipe seen from here is
        // always a pipe whose observer went away.
        self.0.lock_ref(|lock| matches!(*lock, State::Closed))
    }
}

impl<T, E> Observer<T, E> for UnicastSender<'_, T, E> {
    fn on_next(&mut self, value: T) {
        let delivery = self.0.lock_mut(|mut lock| match &mut *lock {
            State::Buffering(pending) | State::Delivering(pending) => {
                // Terminating consumes the sender, so no value can arrive after the termination.
                debug_assert!(!pending.is_terminated());
                pending.push_next(value);
                None
            }
            state @ State::Attached(_) => {
                let State::Attached(observer) =
                    std::mem::replace(state, State::Delivering(PendingEvents::new()))
                else {
                    unreachable!()
                };
                Some((observer, value))
            }
            State::Closed => {
                drop(lock);
                drop(value); // Drop outside the lock to avoid potential deadlock
                None
            }
        });
        if let Some((observer, value)) = delivery {
            deliver(&self.0, observer, Some(value));
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let delivery = self.0.lock_mut(|mut lock| match &mut *lock {
            State::Buffering(pending) | State::Delivering(pending) => {
                // Terminating consumes the sender, so it cannot be terminated twice.
                debug_assert!(!pending.is_terminated());
                pending.set_termination(termination);
                None
            }
            state @ State::Attached(_) => {
                let State::Attached(observer) = std::mem::replace(state, State::Closed) else {
                    unreachable!()
                };
                Some((observer, termination))
            }
            State::Closed => {
                drop(lock);
                drop(termination); // Drop outside the lock to avoid potential deadlock
                None
            }
        });
        if let Some((observer, termination)) = delivery {
            observer.on_termination(termination); // Notify outside the lock
        }
    }
}

/// The observable end of a unicast subject. See [`unicast_subject`].
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastObservable<'or, T, E>(Option<SharedState<'or, T, E>>);

impl<T, E> Drop for UnicastObservable<'_, T, E> {
    fn drop(&mut self) {
        // `None` when it has been subscribed to, which moves the shared state into the disposal.
        if let Some(state) = self.0.take() {
            close(&state);
        }
    }
}

impl<'or, T, E> Observable<'or, T, E> for UnicastObservable<'or, T, E> {
    type D = Disposal<'or, T, E>;

    fn subscribe(
        mut self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let state = self
            .0
            .take()
            .expect("the shared state is taken by either subscribing or dropping");
        let is_open = state.lock_mut(|mut lock| match &mut *lock {
            State::Buffering(pending) => {
                // The buffered events keep waiting in the state, where the delivery below picks
                // them up one at a time.
                let pending = std::mem::take(pending);
                *lock = State::Delivering(pending);
                true
            }
            // The sender was dropped before this subscription, so nothing can ever arrive.
            State::Closed => false,
            // Subscribing consumes the only `UnicastObservable` of the pipe, so an observer can
            // only be attached once.
            State::Attached(_) | State::Delivering(_) => unreachable!(),
        });
        if is_open {
            deliver(&state, BoxedObserver::new(observer), None);
        } else {
            drop(observer); // Drop outside the lock to avoid potential deadlock
        }
        Subscription::new(Disposal(state))
    }
}

/// The disposal of a [`UnicastObservable`] subscription.
#[derive(Educe)]
#[educe(Debug)]
pub struct Disposal<'or, T, E>(SharedState<'or, T, E>);

impl<T, E> Disposable for Disposal<'_, T, E> {
    fn dispose(self) {
        close(&self.0);
    }
}

/// Closes the pipe, so that every later event is dropped.
fn close<T, E>(state: &SharedState<'_, T, E>) {
    let previous_state = safe_lock!(mem_replace: state, State::Closed);
    drop(previous_state); // Drop outside the lock to avoid potential deadlock
}

enum Step<'or, T, E> {
    /// One more value to deliver.
    Next(BoxedObserver<'or, T, E>, T),
    /// The last event of the pipe.
    Terminate(BoxedObserver<'or, T, E>, Termination<E>),
    /// Nothing left to deliver: the observer is parked in [`State::Attached`].
    Park,
    /// The subscription was disposed while delivering, so the observer is handed back to be
    /// dropped outside the lock.
    Close(BoxedObserver<'or, T, E>),
}

/// Delivers `first_value` and then the events waiting in [`State::Delivering`], one at a time.
///
/// The lock is reacquired between two events, so an event that arrives while delivering is
/// delivered in arrival order, and a disposal takes effect immediately: the loop then drops the
/// observer instead of delivering to it. The delivery ends by parking the observer back into
/// [`State::Attached`], by terminating it, or by dropping it.
fn deliver<'or, T, E>(
    state: &SharedState<'or, T, E>,
    mut observer: BoxedObserver<'or, T, E>,
    first_value: Option<T>,
) {
    if let Some(value) = first_value {
        // The observer was taken out of `State::Attached`, so it was not disposed yet.
        observer.on_next(value);
    }
    loop {
        let step = state.lock_mut(|mut lock| {
            let pending = match &mut *lock {
                State::Delivering(pending) => pending,
                State::Closed => return Step::Close(observer),
                // The observer stays out of the state until the delivery ends, and a pipe that is
                // being delivered to has been subscribed to.
                State::Buffering(_) | State::Attached(_) => unreachable!(),
            };
            match pending.pop() {
                Some(Event::Next(value)) => Step::Next(observer, value),
                Some(Event::Termination(termination)) => {
                    *lock = State::Closed;
                    Step::Terminate(observer, termination)
                }
                None => {
                    *lock = State::Attached(observer);
                    Step::Park
                }
            }
        });
        match step {
            Step::Next(next_observer, value) => {
                observer = next_observer;
                observer.on_next(value); // Notify outside the lock
            }
            Step::Terminate(next_observer, termination) => {
                next_observer.on_termination(termination); // Notify outside the lock
                return;
            }
            Step::Park => return,
            Step::Close(next_observer) => {
                drop(next_observer); // Drop outside the lock to avoid potential deadlock
                return;
            }
        }
    }
}
