//! A single-consumer pipe between an [`Observer`] and an [`Observable`].
//!
//! Unlike the multicast subjects, a unicast subject serves exactly one observer, so it can buffer
//! the events that arrive before the subscription instead of dropping them, and move each value to
//! the observer instead of cloning it.
//!
//! The two ends are separate values: [`UnicastSender`] is the [`Observer`] and
//! [`UnicastObservable`] is the [`Observable`]. Neither is [`Clone`], so the type system guarantees
//! one sender and one observer. That is also why it does not implement the [`Subject`] trait and
//! cannot be used with [`ObservableExt::multicast`].
//!
//! # Flavors
//!
//! The pipe exists before its observer subscribes, so its type has to name that observer:
//!
//! | | The observer `OR` itself | A boxed observer |
//! |---|---|---|
//! | Single-threaded | [`local`] | [`local_boxed`] |
//! | Thread-safe | [`shared`] | [`shared_boxed`] |
//! | The mode `M` of the context | [`new`] | [`new_boxed`] |
//!
//! - [`local`] / [`shared`] hold the observer unboxed: subscribing allocates nothing and every
//!   event is a static call. The [`UnicastObservable`] subscribes exactly one observer type `OR`,
//!   usually inferred from the [`subscribe`](Observable::subscribe) call.
//! - [`local_boxed`] / [`shared_boxed`] hold the boxed observer of the mode, and their
//!   [`BoxedUnicastObservable`] subscribes any observer. Use them when the observable's type has to
//!   be written out (stored, returned, emitted as an item as `window` and `group_by` do), or for an
//!   observer that holds its own sender, whose type would otherwise contain itself.
//!
//! The thread-safe flavors keep the pipe behind an `Arc<Mutex<_>>`. [`shared_boxed`] subscribes
//! only `Send` observers; the sender of [`shared`] is `Send` when the observer is. [`new`] and
//! [`new_boxed`] take the mode as a parameter.
//!
//! # Behavior
//!
//! Values sent before the subscription are buffered and replayed on subscription, followed by the
//! termination if there was one. Once the observer is gone — its subscription disposed, or the
//! [`UnicastObservable`] dropped without subscribing — later events are dropped.
//!
//! Dropping the sender without terminating it ends the stream without a termination: the observer
//! still receives every value sent before, whenever it subscribes, and is then dropped silently.
//!
//! # Releasing the observer
//!
//! Disposing only raises a flag, which does not name the observer's type: that is what lets an
//! observer, or an operator such as `take`, hold the subscription of its own pipe. The observer
//! itself is held by the sender's side (in the pipe until the first event after the subscription,
//! then in the sender itself, which lets it deliver without a lock), and is released at the first
//! of:
//!
//! - the end of the notification the disposal happened in, the usual case;
//! - the next event the sender sends, which is dropped too;
//! - the drop of the sender.
//!
//! So an observer disposed outside of its own notification stays alive until the producer sends
//! again or goes away. [`UnicastSender::is_closed`] turns true at once, so a producer that may go
//! quiet can drop its sender to release the observer.
//!
//! An observer that holds its own sender forms a cycle — observer → sender → observer — that the
//! disposal does not break. Such an observer must drop the sender itself, when its `on_next`
//! answers [`Flow::Stop`] and when its subscription is disposed; otherwise both leak.
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observable::ObservableExt,
//!     observer::{Observer, Termination},
//!     subject::unicast_subject,
//! };
//! use std::{
//!     convert::Infallible,
//!     sync::{Arc, Mutex},
//! };
//!
//! let (mut sender, observable) = unicast_subject::shared::<i32, Infallible, _>();
//!
//! // The values sent before the subscription are buffered instead of being dropped.
//! let _ = sender.on_next(111);
//! let _ = sender.on_next(222);
//!
//! let values = Arc::new(Mutex::new(Vec::new()));
//! let values_observer = Arc::clone(&values);
//! let subscription = observable.subscribe_with_callback(
//!     move |value| values_observer.lock().unwrap().push(value),
//!     |_| {},
//! );
//! assert_eq!(&*values.lock().unwrap(), &[111, 222]);
//!
//! let _ = sender.on_next(333);
//! assert_eq!(&*values.lock().unwrap(), &[111, 222, 333]);
//!
//! sender.on_termination(Termination::Completed);
//! drop(subscription);
//! ```
//!
//! [`Subject`]: crate::subject::Subject
//! [`ObservableExt::multicast`]: crate::observable::ObservableExt::multicast

use crate::thread_mode::mutable::{MutableBoolHelper, MutableHelper};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{
        Event, Flow, Observer, Termination,
        boxed_observer::{IntoBoxedObserver, ObserverMode},
    },
    thread_mode::{Local, Shared, ThreadMode},
    utils::{on_panic::on_panic, pending_events::PendingEvents},
};
use educe::Educe;

/// The sending end of a [`BoxedUnicastObservable`]: a [`UnicastSender`] holding the boxed observer
/// of its mode.
pub type BoxedUnicastSender<'or, T, E, M> =
    UnicastSender<T, E, M, <M as ObserverMode>::BoxedObserver<'or, T, E>>;

/// Creates a single-threaded unicast subject that holds its observer `OR` unboxed. See the
/// [module documentation](self).
#[allow(clippy::type_complexity)]
pub fn local<T, E, OR>() -> (
    UnicastSender<T, E, Local, OR>,
    UnicastObservable<T, E, Local, OR>,
) {
    new()
}

/// Creates a thread-safe unicast subject that holds its observer `OR` unboxed. See the
/// [module documentation](self).
#[allow(clippy::type_complexity)]
pub fn shared<T, E, OR>() -> (
    UnicastSender<T, E, Shared, OR>,
    UnicastObservable<T, E, Shared, OR>,
) {
    new()
}

/// Creates a single-threaded unicast subject that subscribes any observer, boxing it. See the
/// [module documentation](self).
pub fn local_boxed<'or, T, E>() -> (
    BoxedUnicastSender<'or, T, E, Local>,
    BoxedUnicastObservable<'or, T, E, Local>,
) {
    new_boxed()
}

/// Creates a thread-safe unicast subject that subscribes any `Send` observer, boxing it. See the
/// [module documentation](self).
pub fn shared_boxed<'or, T, E>() -> (
    BoxedUnicastSender<'or, T, E, Shared>,
    BoxedUnicastObservable<'or, T, E, Shared>,
) {
    new_boxed()
}

/// Creates a unicast subject of the mode `M` that holds its observer `OR` unboxed. See the
/// [module documentation](self).
#[allow(clippy::type_complexity)]
pub fn new<T, E, M: ThreadMode, OR>() -> (UnicastSender<T, E, M, OR>, UnicastObservable<T, E, M, OR>)
{
    let pipe = Pipe {
        is_closed: M::Flag::default(),
        state: M::ptr(State::Pending(PendingEvents::new())),
    };
    (
        UnicastSender {
            pipe: pipe.clone(),
            observer: None,
        },
        UnicastObservable(Some(pipe)),
    )
}

/// Creates a unicast subject of the mode `M` that subscribes any observer, boxing it. See the
/// [module documentation](self).
pub fn new_boxed<'or, T, E, M: ObserverMode>() -> (
    BoxedUnicastSender<'or, T, E, M>,
    BoxedUnicastObservable<'or, T, E, M>,
) {
    let (sender, observable) = new();
    (sender, BoxedUnicastObservable(observable))
}

/// The state of the pipe, behind its one lock. Together with [`Pipe::is_closed`]:
///
/// | The observer is | The state | The flag |
/// |---|---|---|
/// | not subscribed yet, or being replayed to | [`State::Pending`] | down |
/// | parked, waiting for the sender | [`State::Attached`] | down, or raised by a disposal |
/// | held by the sender | [`State::Vacant`] | down, or raised by a disposal |
/// | released | [`State::Vacant`] | raised |
///
/// A disposal only raises the flag, and the sender drops what it left behind. Every other closing
/// empties the state too, except while the sender holds the observer: then nothing else touches
/// the state, so the sender closes the pipe by raising the flag alone, without the lock.
enum State<T, E, OR> {
    /// The events waiting for the subscription or for the running replay. A dropped sender leaves
    /// them here, to be delivered when the observable end is subscribed to.
    Pending(PendingEvents<T, E>),
    /// The observer, parked by the replay.
    Attached(OR),
    /// The sender holds the observer, or the pipe is closed.
    Vacant,
}

/// The pipe, shared by the sending and the observable end.
#[derive(Educe)]
#[educe(Clone(bound()))]
struct Pipe<T, E, M: ThreadMode, OR> {
    /// Never lowered, and read without a lock, which is what lets the sender check it around every
    /// event it delivers. See [`State`].
    is_closed: M::Flag,
    state: M::Ptr<State<T, E, OR>>,
}

impl<T, E, M: ThreadMode, OR> Pipe<T, E, M, OR> {
    /// Closes `state` under its lock, returning what it held, to be dropped after the lock.
    fn close_state(&self, state: &mut State<T, E, OR>) -> State<T, E, OR> {
        self.is_closed.write(true);
        std::mem::replace(state, State::Vacant)
    }

    fn close(&self) {
        let previous_state = self.state.with_mut(|state| self.close_state(state));
        drop(previous_state); // Drop outside the lock
    }
}

/// The sending end of a unicast subject.
///
/// Dropping it without terminating it ends the stream without a termination. See the
/// [module documentation](self).
pub struct UnicastSender<T, E, M: ThreadMode, OR> {
    pipe: Pipe<T, E, M, OR>,
    /// The observer, from the first event that picks it up from [`State::Attached`] until the pipe
    /// ends. Holding it here is what lets an event be delivered without the lock.
    observer: Option<OR>,
}

impl<T, E, M: ThreadMode, OR> Drop for UnicastSender<T, E, M, OR> {
    fn drop(&mut self) {
        if let Some(observer) = self.observer.take() {
            // Held here, so the state is vacant: raising the flag closes the pipe.
            self.pipe.is_closed.write(true);
            drop(observer);
            return;
        }
        // A pending queue stays for the replay to deliver, ending with the termination queued last
        // if any (terminating consumes the sender, so this also runs right after it). Any other
        // state is closed.
        let previous_state = self.pipe.state.with_mut(|current| match current {
            State::Pending(_) => None,
            state => Some(self.pipe.close_state(state)),
        });
        drop(previous_state); // Drop outside the lock
    }
}

/// What became of an event sent while the sender does not hold the observer.
enum Unheld<V> {
    /// It waits in the queue.
    Queued,
    /// The pipe is closed, so it was dropped.
    Discarded,
    /// The sender picked the observer up from [`State::Attached`]; the payload is still to deliver.
    Taken(V),
}

impl<T, E, M: ThreadMode, OR> UnicastSender<T, E, M, OR> {
    /// Returns whether every later event is dropped: the subscription was disposed, the observer
    /// answered [`Flow::Stop`] or panicked, or the [`UnicastObservable`] was dropped without being
    /// subscribed to.
    ///
    /// It turns true at once, while the observer may stay alive until the sender releases it, as
    /// the [module documentation](self#releasing-the-observer) describes.
    pub fn is_closed(&self) -> bool {
        self.pipe.is_closed.read()
    }

    /// Sends the event that `payload` makes while the sender does not hold the observer. On
    /// [`Unheld::Taken`], the sender holds it from now on.
    fn send_unheld<V>(&mut self, payload: V, into_event: fn(V) -> Event<T, E>) -> Unheld<V> {
        // What is not delivered is handed back, to be dropped after the lock.
        let (unheld, observer, rejected, discarded) =
            self.pipe.state.with_mut(|current| match current {
                // Terminating consumes the sender, so nothing arrives after the termination.
                State::Pending(pending) => {
                    let rejected = pending.push(into_event(payload));
                    (Unheld::Queued, None, rejected, None)
                }
                State::Attached(_) if !self.pipe.is_closed.read() => {
                    let State::Attached(observer) = std::mem::replace(current, State::Vacant)
                    else {
                        unreachable!()
                    };
                    (Unheld::Taken(payload), Some(observer), None, None)
                }
                // Disposed while parked, or closed: see `State`.
                State::Attached(_) | State::Vacant => {
                    let closed = self.pipe.close_state(current);
                    (Unheld::Discarded, None, None, Some((payload, closed)))
                }
            });
        // Asserted after the lock, which a failing assertion would otherwise poison.
        debug_assert!(rejected.is_none());
        drop((rejected, discarded)); // Drop outside the lock
        if observer.is_some() {
            self.observer = observer;
        }
        unheld
    }
}

impl<T, E, M: ThreadMode, OR: Observer<T, E>> Observer<T, E> for UnicastSender<T, E, M, OR> {
    fn on_next(&mut self, value: T) -> Flow {
        if self.observer.is_some() {
            return self.send_held(value);
        }
        match self.send_unheld(value, Event::Next) {
            Unheld::Queued => Flow::Continue,
            Unheld::Discarded => Flow::Stop,
            Unheld::Taken(value) => self.send_held(value),
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        let termination = if self.observer.is_some() {
            termination
        } else {
            match self.send_unheld(termination, Event::Termination) {
                Unheld::Taken(termination) => termination,
                Unheld::Queued | Unheld::Discarded => return,
            }
        };
        let observer = self.observer.take().expect("the sender holds the observer");
        // The drop of the sender, right after this, closes the pipe.
        if self.pipe.is_closed.read() {
            drop(observer);
            drop(termination);
        } else {
            observer.on_termination(termination); // Notify outside the lock
        }
    }
}

impl<T, E, M: ThreadMode, OR: Observer<T, E>> UnicastSender<T, E, M, OR> {
    /// Sends `value` to the observer held by the sender, without the lock.
    ///
    /// The flag is read before the notification, to skip an observer that is gone, and after it,
    /// since disposing from inside the notification is how a consumer usually stops: the observer
    /// is then released at once instead of at the next event.
    fn send_held(&mut self, value: T) -> Flow {
        if !self.pipe.is_closed.read() {
            let observer = self
                .observer
                .as_mut()
                .expect("the sender holds the observer");
            // A panicking observer closes the pipe, as it does during the replay; the sender then
            // drops it at the next event or on its own drop.
            let close_on_panic = on_panic(|| self.pipe.is_closed.write(true));
            let flow = observer.on_next(value); // Notify without the lock
            drop(close_on_panic);
            if flow.is_continue() && !self.pipe.is_closed.read() {
                return Flow::Continue;
            }
        }
        // The state is vacant, so raising the flag closes the pipe. An undelivered value is dropped
        // after the observer.
        self.pipe.is_closed.write(true);
        drop(self.observer.take());
        Flow::Stop
    }
}

/// The observable end of a unicast subject, which subscribes exactly the observer `OR`. See the
/// [module documentation](self).
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
pub struct UnicastObservable<T, E, M: ThreadMode, OR>(Option<Pipe<T, E, M, OR>>);

impl<T, E, M: ThreadMode, OR> Drop for UnicastObservable<T, E, M, OR> {
    fn drop(&mut self) {
        // `None` once subscribed to.
        if let Some(pipe) = self.0.take() {
            pipe.close();
        }
    }
}

impl<T, E, M: ThreadMode, OR> ObservableTypes for UnicastObservable<T, E, M, OR> {
    type Item = T;
    type Error = E;
    /// The events come from wherever the sender is fed, which is what `M` says.
    type Mode = M;
    type Disposal = Disposal<M>;
}

impl<T, E, M, OR> Observable<OR> for UnicastObservable<T, E, M, OR>
where
    M: ThreadMode,
    OR: Observer<T, E>,
{
    fn subscribe(mut self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let pipe = self
            .0
            .take()
            .expect("the pipe is taken by either subscribing or dropping");
        deliver(&pipe, observer);
        DisposeOnDrop::new(Disposal(pipe.is_closed))
    }
}

/// The observable end of a boxed unicast subject, which subscribes any observer by boxing it. See
/// the [module documentation](self).
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
pub struct BoxedUnicastObservable<'or, T, E, M: ObserverMode>(
    UnicastObservable<T, E, M, M::BoxedObserver<'or, T, E>>,
);

impl<'or, T, E, M: ObserverMode> ObservableTypes for BoxedUnicastObservable<'or, T, E, M> {
    type Item = T;
    type Error = E;
    /// The events come from wherever the sender is fed, which is what `M` says.
    type Mode = M;
    type Disposal = Disposal<M>;
}

impl<'or, T, E, M, OR> Observable<OR> for BoxedUnicastObservable<'or, T, E, M>
where
    M: ObserverMode,
    OR: IntoBoxedObserver<'or, T, E, M>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.0.subscribe(M::boxed(observer))
    }
}

/// The disposal of a [`UnicastObservable`] or [`BoxedUnicastObservable`] subscription.
///
/// It only raises the flag that closes the pipe; see the
/// [module documentation](self#releasing-the-observer) for when the observer is released.
pub struct Disposal<M: ThreadMode>(M::Flag);

impl<M: ThreadMode> Disposable for Disposal<M> {
    fn dispose(self) {
        self.0.write(true);
    }
}

enum Step<T, E, OR> {
    Next(OR, T),
    Terminate(OR, Termination<E>),
    /// Nothing left: the observer is parked in [`State::Attached`].
    Park,
}

/// Replays the events waiting in [`State::Pending`] to `observer`, one at a time, then parks it.
///
/// The lock is retaken for every event, so events the sender adds meanwhile keep their order. No
/// disposal can happen during the replay, since the subscription is handed out afterwards: the
/// observer stops it by answering [`Flow::Stop`] instead.
///
/// If the sender is already gone without a termination, the parked observer is dropped silently
/// with the last pipe, once the caller lets go of it.
fn deliver<T, E, M: ThreadMode, OR: Observer<T, E>>(pipe: &Pipe<T, E, M, OR>, mut observer: OR) {
    loop {
        // No disposal exists yet, and the sender closes the pipe only once it holds the observer.
        // Asserted outside the lock, which a failing assertion would otherwise poison.
        debug_assert!(!pipe.is_closed.read(), "the replay runs on an open pipe");
        let step: Step<T, E, OR> = pipe.state.with_mut(|current| {
            // The replay holds the observer, so neither the state nor the sender can.
            let State::Pending(pending) = &mut *current else {
                unreachable!()
            };
            match pending.pop() {
                Some(Event::Next(value)) => Step::Next(observer, value),
                Some(Event::Termination(termination)) => {
                    // The termination comes last, so the queue dropped here is empty.
                    drop(pipe.close_state(current));
                    Step::Terminate(observer, termination)
                }
                None => {
                    *current = State::Attached(observer);
                    Step::Park
                }
            }
        });
        match step {
            Step::Next(next_observer, value) => {
                observer = next_observer;
                // A panic unwinds the observer away while the state is still pending, so close
                // the pipe instead of letting events pile up for a replay that never resumes.
                let close_on_panic = on_panic(|| pipe.close());
                let flow = observer.on_next(value); // Notify outside the lock
                drop(close_on_panic);
                if flow.is_stop() {
                    pipe.close();
                    drop(observer); // Drop outside the lock
                    return;
                }
            }
            Step::Terminate(next_observer, termination) => {
                // The pipe is closed already, so a panic here needs no guard.
                Observer::<T, E>::on_termination(next_observer, termination); // Notify outside the lock
                return;
            }
            Step::Park => return,
        }
    }
}
