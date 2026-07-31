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
        types::{MaybeSend, Mutable, MutableBool, MutableBoolHelper, MutableHelper, Shared},
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
/// # Releasing the observer
///
/// Disposing the subscription does not necessarily drop the observer where it happens: between two
/// events the sender holds it, which is what lets it deliver a value without taking a lock, and
/// only the sender can let go of it. It does so at the first of these:
///
/// - the end of the notification the disposal happened in, which is the usual case, because a
///   consumer that stops a stream normally does it from inside the notification of a value;
/// - the next event the sender sends, which is dropped along with the observer;
/// - the drop of the sender.
///
/// So an observer whose subscription is disposed between two events, by a consumer that is not the
/// one being notified, stays alive until the producer sends again or goes away. A producer that
/// might go quiet for a long time can use [`UnicastSender::is_disposed`], which is true as soon as
/// the disposal happens, to drop its sender and release the observer with it.
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
    let pipe = Shared::new(Pipe {
        is_disposed: MutableBool::new(false),
        state: Mutable::new(State::Pending(pending)),
    });
    (
        UnicastSender {
            pipe: pipe.clone(),
            observer: None,
        },
        UnicastObservable(Some(pipe)),
    )
}

#[derive(Educe)]
#[educe(Debug)]
enum State<'or, T, E> {
    /// The observer is not held here, so the events that arrive wait in the queue: before the
    /// subscription, and while the buffered events are being replayed outside the lock.
    Pending(PendingEvents<T, E>),
    /// The observer has subscribed and is idle, waiting for the sender to pick it up.
    Attached(BoxedObserver<'or, T, E>),
    /// The sender holds the observer and delivers to it on its own, so nothing waits here: the
    /// sender is the only one that queues events, and it has nothing left to queue them for.
    Held,
    /// The observer is gone, either because it was terminated or because the subscription was
    /// disposed. Every later event is dropped.
    Closed,
}

/// The pipe itself, which its sending and its observable end share.
#[derive(Educe)]
#[educe(Debug)]
struct Pipe<'or, T, E> {
    /// Whether the observer went away, which is the one thing the sender still has to learn from
    /// here once it holds the observer itself. Reading it takes no lock, which is what lets the
    /// sender check it before and after every event it delivers on its own.
    ///
    /// This is not a copy of [`State::Closed`], and only [`close`] raises it: it says that the
    /// observer was taken away from the pipe, not that the pipe is over. Terminating the pipe and
    /// dropping the sender close the state without touching it, because the sender is gone by then
    /// and the sender is its only reader.
    is_disposed: MutableBool,
    state: Mutable<State<'or, T, E>>,
}

type SharedPipe<'or, T, E> = Shared<Pipe<'or, T, E>>;

/// The sending end of a unicast subject. See [`unicast_subject`].
///
/// Dropping the sender without terminating it closes the pipe, which drops the observer without
/// notifying it: no event can reach it anymore, because the sender was the only way in, but a
/// producer that gave up halfway has not completed anything either. That is also what releases an
/// observer whose subscription was disposed while the sender was idle, as [`unicast_subject`]
/// describes.
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastSender<'or, T, E> {
    pipe: SharedPipe<'or, T, E>,
    /// The observer, held here instead of in the shared state so that sending an event takes no
    /// lock at all: the sender is the only producer of the pipe, so nothing else has to reach the
    /// observer while it is idle.
    ///
    /// It is taken out of [`State::Attached`] by the first event that finds it parked there, and
    /// stays here until the pipe ends. The state of the pipe is [`State::Held`] meanwhile, which
    /// carries nothing: nothing can be queued behind an observer that only the sender feeds.
    ///
    /// The price is that [`close`] cannot drop the observer anymore, because the observer is not
    /// in the state it closes. The sender drops it instead, as soon as it sees
    /// [`Pipe::is_disposed`], and at the latest when the sender itself is dropped.
    observer: Option<BoxedObserver<'or, T, E>>,
}

impl<T, E> Drop for UnicastSender<'_, T, E> {
    fn drop(&mut self) {
        // The pipe is over, so the observer held here is dropped without being notified, like the
        // one the state below holds.
        let observer = self.observer.take();
        // Terminating the pipe consumes the sender, so this also runs right after the last event
        // was queued. That event still has to reach the observer, whether it is waiting in the
        // queue for a late subscriber or for the delivery that is running.
        let previous_state = self.pipe.state.lock_mut(|mut lock| match &mut *lock {
            State::Pending(pending) if pending.is_terminated() => None,
            state => Some(std::mem::replace(state, State::Closed)),
        });
        drop(observer); // Drop outside the lock to avoid potential deadlock
        drop(previous_state); // Drop outside the lock to avoid potential deadlock
    }
}

impl<T, E> UnicastSender<'_, T, E> {
    /// Returns whether the observer is gone, which happens when its subscription is disposed or
    /// when the [`UnicastObservable`] is dropped without being subscribed to.
    ///
    /// Every later event is dropped, so a producer can use this to stop producing.
    pub fn is_disposed(&self) -> bool {
        // This is exactly what the flag says, and reading it takes no lock. The state would say
        // the same, because the only other way to close it consumes the sender.
        self.pipe.is_disposed.read()
    }
}

impl<T, E> Observer<T, E> for UnicastSender<'_, T, E> {
    fn on_next(&mut self, value: T) {
        if self.observer.is_some() {
            self.send_held(value);
            return;
        }
        let delivery = self.pipe.state.lock_mut(|mut lock| match &mut *lock {
            State::Pending(pending) => {
                // Terminating consumes the sender, so no value can arrive after the termination.
                let rejected = pending.push(Event::Next(value));
                debug_assert!(rejected.is_none());
                drop(lock);
                drop(rejected); // Drop outside the lock to avoid potential deadlock
                None
            }
            state @ State::Attached(_) => {
                let State::Attached(observer) = std::mem::replace(state, State::Held) else {
                    unreachable!()
                };
                Some((observer, value))
            }
            // The sender takes the fast path above while it holds the observer, so it never looks
            // at a state it is itself the subject of.
            State::Held => unreachable!(),
            State::Closed => {
                drop(lock);
                drop(value); // Drop outside the lock to avoid potential deadlock
                None
            }
        });
        if let Some((observer, value)) = delivery {
            // The observer stays here from now on, so this is the last event that has to look for
            // it in the state of the pipe.
            self.observer = Some(observer);
            self.send_held(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(observer) = self.observer.take() {
            // The observer is held here, so the state only has to be closed, and the drop of the
            // sender that runs right after this has nothing left to close or to hand over. It is
            // [`State::Held`], unless a disposal closed it while the observer was held here.
            let previous_state = safe_lock!(mem_replace: self.pipe.state, State::Closed);
            let is_disposed = matches!(previous_state, State::Closed);
            drop(previous_state); // Drop outside the lock to avoid potential deadlock
            if is_disposed {
                // The subscription was disposed while the observer was held here, so nothing is
                // notified anymore: the observer is only dropped, as `close` could not.
                drop(observer);
                drop(termination);
            } else {
                observer.on_termination(termination); // Notify outside the lock
            }
            return;
        }
        let delivery = self.pipe.state.lock_mut(|mut lock| match &mut *lock {
            State::Pending(pending) => {
                // Terminating consumes the sender, so it cannot be terminated twice.
                let rejected = pending.push(Event::Termination(termination));
                debug_assert!(rejected.is_none());
                drop(lock);
                drop(rejected); // Drop outside the lock to avoid potential deadlock
                None
            }
            state @ State::Attached(_) => {
                let State::Attached(observer) = std::mem::replace(state, State::Closed) else {
                    unreachable!()
                };
                Some((observer, termination))
            }
            // Terminating while the sender holds the observer is the fast path above.
            State::Held => unreachable!(),
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

impl<T, E> UnicastSender<'_, T, E> {
    /// Sends `value` to the observer held by the sender, without taking the lock.
    ///
    /// The flag is what tells the sender that the observer went away while it was held here, so it
    /// is read before the notification, to drop the value instead of delivering it, and after it,
    /// to release an observer that was disposed by the notification itself. Nothing else is
    /// needed: the state cannot change under a pipe whose only producer is the caller.
    fn send_held(&mut self, value: T) {
        debug_assert!(self.observer.is_some());
        if self.pipe.is_disposed.read() {
            let observer = self.observer.take();
            // The state is closed already, and no lock is held here anyway, so both are simply
            // dropped where they are.
            drop(observer);
            drop(value);
            return;
        }
        if let Some(observer) = &mut self.observer {
            observer.on_next(value); // Notify without taking the lock
        }
        // Disposing from inside that notification is how a consumer usually stops a stream, so the
        // flag is read once more to release the observer right away instead of at the next event.
        if self.pipe.is_disposed.read() {
            let observer = self.observer.take();
            drop(observer);
        }
    }
}

/// The observable end of a unicast subject. See [`unicast_subject`].
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastObservable<'or, T, E>(Option<SharedPipe<'or, T, E>>);

impl<T, E> Drop for UnicastObservable<'_, T, E> {
    fn drop(&mut self) {
        // `None` when it has been subscribed to, which moves the shared state into the disposal.
        if let Some(pipe) = self.0.take() {
            close(&pipe);
        }
    }
}

impl<'or, T, E> Observable<'or, T, E> for UnicastObservable<'or, T, E> {
    type D = Disposal<'or, T, E>;

    fn subscribe(
        mut self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let pipe = self
            .0
            .take()
            .expect("the shared state is taken by either subscribing or dropping");
        // The replay takes the pipe as it finds it: it parks the observer when nothing waits, it
        // replays what does, and it drops the observer when the sender is already gone. Asking the
        // state about that beforehand would only be one more lock for the answer it takes anyway.
        // The observer is parked into `State::Attached`, from where the next event the sender
        // sends picks it up for good, unless the replay ends the pipe with a buffered termination.
        let is_live = deliver(&pipe, BoxedObserver::new(observer));
        // A pipe that is over stays over, so the subscription has nothing left to dispose of and
        // does not have to keep the pipe alive until the consumer drops it.
        Subscription::new(Disposal(is_live.then_some(pipe)))
    }
}

/// The disposal of a [`UnicastObservable`] subscription.
///
/// It holds no pipe when the pipe was already over by the end of the subscription, which is the
/// only thing there is to know about it: a pipe that is over cannot be disposed of anymore, and a
/// pipe that is not cannot become so on its own. Holding nothing releases the pipe right away
/// instead of when the subscription is dropped, and costs nothing to carry: a [`Shared`] is a
/// pointer, so wrapping it in an [`Option`] does not make it any bigger.
#[derive(Educe)]
#[educe(Debug)]
pub struct Disposal<'or, T, E>(Option<SharedPipe<'or, T, E>>);

impl<T, E> Disposable for Disposal<'_, T, E> {
    fn dispose(self) {
        if let Some(pipe) = self.0 {
            close(&pipe);
        }
    }
}

/// Closes the pipe, so that every later event is dropped.
///
/// The observer is dropped here when the state holds it. When the sender holds it instead, only
/// the flag below can reach the sender: the observer is then dropped by the sender, on its next
/// event or when it is dropped itself.
fn close<T, E>(pipe: &SharedPipe<'_, T, E>) {
    // Raised before the state is replaced, so that the sender never delivers an event to an
    // observer that the state has already given up on.
    pipe.is_disposed.write(true);
    let previous_state = safe_lock!(mem_replace: pipe.state, State::Closed);
    drop(previous_state); // Drop outside the lock to avoid potential deadlock
}

enum Step<'or, T, E> {
    /// One more value to deliver.
    Next(BoxedObserver<'or, T, E>, T),
    /// The last event of the pipe.
    Terminate(BoxedObserver<'or, T, E>, Termination<E>),
    /// Nothing left to deliver: the observer is parked in [`State::Attached`].
    Park,
    /// The pipe is over, either before the observer subscribed or by a disposal that happened
    /// while delivering, so the observer is handed back to be dropped outside the lock.
    Close(BoxedObserver<'or, T, E>),
}

/// Delivers the events waiting in [`State::Pending`] to `observer`, one at a time.
///
/// This is the replay of the events that were buffered before the subscription, and it is the
/// whole of what subscribing does: an empty queue simply parks the observer, and a pipe that is
/// over drops it, so the caller has nothing to check beforehand. The lock is reacquired between
/// two events, so an event that the sender adds while replaying is delivered in arrival order, and
/// a disposal takes effect immediately: the loop then drops the observer instead of delivering to
/// it. The delivery ends by parking the observer into [`State::Attached`], by terminating it, or
/// by dropping it.
///
/// Returns whether the observer was parked, which is the only ending that leaves the pipe alive:
/// the other two are the pipe being over, which it stays.
fn deliver<'or, T, E>(
    pipe: &SharedPipe<'or, T, E>,
    mut observer: BoxedObserver<'or, T, E>,
) -> bool {
    loop {
        let step = pipe.state.lock_mut(|mut lock| {
            let pending = match &mut *lock {
                State::Pending(pending) => pending,
                State::Closed => return Step::Close(observer),
                // This replay holds the observer until it parks it below, so neither the state nor
                // the sender can be holding it at the same time.
                State::Attached(_) | State::Held => unreachable!(),
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
                return false;
            }
            Step::Park => return true,
            Step::Close(next_observer) => {
                drop(next_observer); // Drop outside the lock to avoid potential deadlock
                return false;
            }
        }
    }
}
