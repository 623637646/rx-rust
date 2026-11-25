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
//! The thread-safe flavors keep the pipe behind an `Arc<Mutex<_>>`, which may hold the buffered
//! events as well as the observer: the two ends of [`shared`] are `Send` when the values, the error
//! and the observer are. [`shared_boxed`] subscribes only `Send` observers, so its ends need the
//! values and the error to be. [`new`] and [`new_boxed`] take the mode as a parameter.
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
//! # Handing the observer over
//!
//! The replay runs inside `subscribe`, on the thread that calls it. What the sender sends meanwhile
//! — from another thread, or from inside a notification of the replay, the sender being free then
//! — waits behind the values being replayed, and is replayed too. Once nothing is left, the observer
//! is parked in the pipe, and the next event the sender sends takes it: from then on it is notified
//! on the sender's thread, without a lock. That may be before `subscribe` has returned, when the
//! sender is on another thread.
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
//! Closing the pipe and releasing the observer are one step when the observer answers
//! [`Flow::Break`], and when it panics during the replay. When it panics as the sender delivers to
//! it, the pipe closes at once, but the sender still holds the observer, until the next event or
//! its own drop, as after a disposal.
//!
//! The buffer itself does not outlive the hand-over: once the sender has picked up the observer
//! and `subscribe` has returned, it is freed, and the sender delivers with nothing but the
//! observer and the flag.
//!
//! An observer that holds its own sender forms a cycle — observer → sender → observer — that the
//! disposal does not break. Such an observer must drop the sender itself, when its `on_next`
//! answers [`Flow::Break`] and when its subscription is disposed; otherwise both leak.
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
//! let (mut sender, observable) = unicast_subject::shared::<_, Infallible, _>();
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

use crate::{
    delegate_disposal,
    disposable::{dispose_on_drop::DisposeOnDrop, flag_disposal::FlagDisposal},
    observable::{Observable, ObservableTypes},
    observer::{
        Event, Flow, Observer, Termination,
        boxed_observer::{IntoBoxedObserver, ObserverMode},
    },
    thread_mode::{
        Local, Shared, ThreadMode,
        mutable::{MutableHelper, RaiseOnlyFlag},
    },
    utils::{on_panic::on_panic, pending_events::PendingEvents},
};
use educe::Educe;
use std::collections::VecDeque;

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
    let pipe: Pipe<_, _, M, _> = Pipe {
        is_closed: M::Flag::default(),
        state: M::ptr(State::Pending(PendingEvents::new())),
    };
    (
        UnicastSender {
            is_closed: pipe.is_closed.clone(),
            state: SenderState::ViaPipe(pipe.state.clone()),
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
/// | parked, waiting for the sender | [`State::Parked`] | down, or raised by a disposal |
/// | held by the sender | [`State::Vacant`] | down, or raised by a disposal |
/// | released | [`State::Vacant`] | raised |
///
/// A disposal only raises the flag, and the sender drops what it left behind. Every other closing
/// empties the state too. Once the sender takes the observer, it lets go of the shared state and
/// closes the pipe by raising the flag alone, without the lock.
enum State<T, E, OR> {
    /// The events waiting for the subscription or for the running replay. A dropped sender leaves
    /// them here, to be delivered when the observable end is subscribed to.
    Pending(PendingEvents<T, E>),
    /// The observer, parked by the replay.
    Parked(OR),
    /// The sender holds the observer, or the pipe is closed.
    Vacant,
}

/// The pipe, held by the observable end until it is subscribed to. The sender holds a clone of
/// the flag, and of the state until it takes the observer (see [`SenderState`]).
struct Pipe<T, E, M: ThreadMode, OR> {
    /// Never lowered, and read without a lock, which is what lets the sender check it around every
    /// event it delivers. See [`State`].
    is_closed: M::Flag,
    state: M::Ptr<State<T, E, OR>>,
}

/// Closes `state` under its lock, returning what it held, to be dropped after the lock.
fn close_state<T, E, OR>(
    is_closed: &impl RaiseOnlyFlag,
    state: &mut State<T, E, OR>,
) -> State<T, E, OR> {
    is_closed.raise();
    std::mem::replace(state, State::Vacant)
}

impl<T, E, M: ThreadMode, OR> Pipe<T, E, M, OR> {
    fn close(&self) {
        let previous_state = self
            .state
            .with_mut(|state| close_state(&self.is_closed, state));
        drop(previous_state); // Drop outside the lock
    }
}

/// How the sender reaches its observer. Taking it releases the sender's shared state pointer; once
/// the subscribing call returns too, that allocation is gone.
enum SenderState<T, E, M: ThreadMode, OR> {
    /// The observer is not subscribed yet, being replayed to or parked: events go through the
    /// state, under its lock.
    ViaPipe(M::Ptr<State<T, E, OR>>),
    /// The sender took the observer from [`State::Parked`], and delivers without a lock.
    Direct(OR),
    /// The pipe is closed and the sender has let go of everything; the flag is raised.
    Closed,
}

/// The sending end of a unicast subject.
///
/// Dropping it without terminating it ends the stream without a termination. See the
/// [module documentation](self).
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastSender<T, E, M: ThreadMode, OR> {
    #[educe(Debug(ignore))]
    is_closed: M::Flag,
    #[educe(Debug(ignore))]
    state: SenderState<T, E, M, OR>,
}

impl<T, E, M: ThreadMode, OR> Drop for UnicastSender<T, E, M, OR> {
    fn drop(&mut self) {
        let SenderState::ViaPipe(state) = &self.state else {
            // An observer held here is dropped with the sender, after raising the flag.
            self.is_closed.raise();
            return;
        };
        // A pending queue stays for the replay to deliver, ending with the termination queued last
        // if any (terminating consumes the sender, so this also runs right after it). Any other
        // state is closed.
        let previous_state = state.with_mut(|current| match current {
            State::Pending(_) => None,
            state => Some(close_state(&self.is_closed, state)),
        });
        drop(previous_state); // Drop outside the lock
    }
}

/// What an event sent while the sender does not hold the observer left to do once the lock of the
/// state is released.
enum SendAction<T, E, OR, V> {
    /// It waits in the queue; what the queue gave back, which nothing should be, is to be dropped.
    Queued(Option<Event<T, E>>),
    /// The sender picked the observer up from [`State::Parked`]; the payload is still to deliver.
    Taken(OR, V),
    /// The pipe is closed: the payload and what the state held are to be dropped.
    Discarded(V, State<T, E, OR>),
}

/// What [`UnicastSender::prepare_send`] decided for an event.
enum Prepared<V> {
    /// The sender holds the observer: the payload is to be delivered to it, without the lock.
    Deliver(V),
    /// The event was queued or dropped, and this is the answer to give.
    Answer(Flow),
}

impl<T, E, M: ThreadMode, OR> UnicastSender<T, E, M, OR> {
    /// Returns whether every later event is dropped: the subscription was disposed, the observer
    /// answered [`Flow::Break`] or panicked, or the [`UnicastObservable`] was dropped without being
    /// subscribed to.
    ///
    /// It turns true at once, while the observer may stay alive until the sender releases it, as
    /// the [module documentation](self#releasing-the-observer) describes.
    pub fn is_closed(&self) -> bool {
        self.is_closed.is_raised()
    }

    /// Queues or discards the event that `payload` makes, or prepares to deliver it directly. On
    /// [`Prepared::Deliver`], the sender holds the observer from now on.
    fn prepare_send<V>(
        &mut self,
        payload: V,
        into_event: impl FnOnce(V) -> Event<T, E>,
    ) -> Prepared<V> {
        let state = match &self.state {
            SenderState::Direct(_) => return Prepared::Deliver(payload),
            SenderState::Closed => return Prepared::Answer(Flow::Break(())),
            SenderState::ViaPipe(state) => state,
        };
        // What is not delivered is handed back, to be dropped after the lock.
        let action = state.with_mut(|current| match current {
            State::Pending(pending) => SendAction::Queued(pending.push(into_event(payload))),
            State::Parked(_) if !self.is_closed.is_raised() => {
                let State::Parked(observer) = std::mem::replace(current, State::Vacant) else {
                    unreachable!()
                };
                SendAction::Taken(observer, payload)
            }
            // Disposed while parked, or closed: see `State`.
            State::Parked(_) | State::Vacant => {
                SendAction::Discarded(payload, close_state(&self.is_closed, current))
            }
        });
        match action {
            SendAction::Queued(rejected) => {
                // Terminating consumes the sender, so nothing arrives after the termination.
                // Asserted after the lock, which a failing assertion would otherwise poison.
                debug_assert!(rejected.is_none());
                drop(rejected);
                Prepared::Answer(Flow::Continue(()))
            }
            SendAction::Taken(observer, payload) => {
                self.state = SenderState::Direct(observer);
                Prepared::Deliver(payload)
            }
            SendAction::Discarded(payload, previous_state) => {
                self.state = SenderState::Closed;
                // Outside the lock, the observer first, if it was parked there.
                drop(previous_state);
                drop(payload);
                Prepared::Answer(Flow::Break(()))
            }
        }
    }
}

impl<T, E, M: ThreadMode, OR: Observer<T, E>> Observer<T, E> for UnicastSender<T, E, M, OR> {
    fn on_next(&mut self, value: T) -> Flow {
        match self.prepare_send(value, Event::Next) {
            Prepared::Deliver(value) => self.send_direct(value),
            Prepared::Answer(flow) => flow,
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        let Prepared::Deliver(termination) = self.prepare_send(termination, Event::Termination)
        else {
            return;
        };
        let SenderState::Direct(observer) = std::mem::replace(&mut self.state, SenderState::Closed)
        else {
            unreachable!("the sender holds the observer");
        };
        // The drop of the sender, right after this, closes the pipe.
        if self.is_closed.is_raised() {
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
    fn send_direct(&mut self, value: T) -> Flow {
        if !self.is_closed.is_raised() {
            let SenderState::Direct(observer) = &mut self.state else {
                unreachable!("the sender holds the observer");
            };
            // A panicking observer closes the pipe, as it does during the replay; the sender then
            // drops it at the next event or on its own drop.
            let close_on_panic = on_panic(|| self.is_closed.raise());
            let flow = observer.on_next(value); // Notify without the lock
            close_on_panic.disarm();
            if flow.is_continue() && !self.is_closed.is_raised() {
                return Flow::Continue(());
            }
        }
        // The shared state has been released, so raising the flag closes the pipe. An undelivered
        // value is dropped after the observer.
        self.is_closed.raise();
        drop(std::mem::replace(&mut self.state, SenderState::Closed));
        Flow::Break(())
    }
}

/// The observable end of a unicast subject, which subscribes exactly the observer `OR`. See the
/// [module documentation](self).
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
#[derive(Educe)]
#[educe(Debug)]
pub struct UnicastObservable<T, E, M: ThreadMode, OR>(
    #[educe(Debug(ignore))] Option<Pipe<T, E, M, OR>>,
);

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
        replay(&pipe, observer);
        DisposeOnDrop::new(FlagDisposal(pipe.is_closed)).map_inner_into()
    }
}

/// The observable end of a boxed unicast subject, which subscribes any observer by boxing it. See
/// the [module documentation](self).
///
/// [`Observable::subscribe`] consumes it, so the pipe cannot be subscribed to twice.
#[derive(Educe)]
#[educe(Debug)]
pub struct BoxedUnicastObservable<'or, T, E, M: ObserverMode>(
    #[educe(Debug(ignore))] UnicastObservable<T, E, M, M::BoxedObserver<'or, T, E>>,
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

delegate_disposal!(
    /// The disposal of a [`UnicastObservable`] or [`BoxedUnicastObservable`] subscription.
    ///
    /// It only raises the flag that closes the pipe; see the
    /// [module documentation](self#releasing-the-observer) for when the observer is released.
    Disposal<M>,
    FlagDisposal<<M as ThreadMode>::Flag>,
    where M: ThreadMode
);

/// What the replay does next, decided under the lock of the state.
enum ReplayStep<E> {
    /// Deliver the values just moved into the batch, then ask for the next step.
    Deliver,
    /// Nothing is left: the observer is parked in [`State::Parked`].
    Parked,
    /// Terminate the observer. The pipe is closed already.
    Terminate(Termination<E>),
}

/// Replays the events waiting in [`State::Pending`] to `observer`, then parks it.
///
/// Each time it runs out of values, the replay takes the lock once and moves every value queued so
/// far into a batch of its own, which it then delivers without the lock; the termination stays in
/// the queue, so nothing can still be queued after it. Events the sender adds meanwhile queue up
/// behind the batch, so they keep their order. No disposal can happen during the replay, since the
/// subscription is handed out afterwards, so nothing can stop it between two values of a batch but
/// the observer itself, answering [`Flow::Break`] or unwinding: the pipe is then closed, dropping
/// what is still queued, then the observer is dropped, then the batch.
///
/// If the sender is already gone without a termination, the parked observer is dropped silently
/// with the last pipe, once the caller lets go of it.
fn replay<T, E, M: ThreadMode, OR: Observer<T, E>>(pipe: &Pipe<T, E, M, OR>, observer: OR) {
    // The values taken from the queue under one lock, delivered one at a time without it. It
    // trades its allocation with the queue's at each batch, so `new` allocates nothing here.
    let mut batch = VecDeque::new();
    // Until it is parked or terminated. Declared after the batch, so that every other end drops it
    // before the values left there, an unwind included.
    let mut observer = Some(observer);
    // A panic unwinds the observer away while the state is still pending, so close the pipe
    // instead of letting events pile up for a replay that never resumes. The guard is the last
    // declared, so the pipe is closed before the observer is dropped.
    let close_on_panic = on_panic(|| pipe.close());
    loop {
        // No disposal exists yet, and the sender closes the pipe only once it holds the observer.
        // Asserted outside the lock, which a failing assertion would otherwise poison.
        debug_assert!(
            !pipe.is_closed.is_raised(),
            "the replay runs on an open pipe"
        );
        let step = pipe.state.with_mut(|current| {
            // The replay holds the observer, so neither the state nor the sender can.
            let State::Pending(pending) = &mut *current else {
                unreachable!()
            };
            if pending.take_values_into(&mut batch) {
                return ReplayStep::Deliver;
            }
            match pending.take_termination() {
                Some(termination) => {
                    // The termination comes last, so the queue dropped here is empty.
                    drop(close_state(&pipe.is_closed, current));
                    ReplayStep::Terminate(termination)
                }
                None => {
                    *current = State::Parked(observer.take().expect(OBSERVER_REPLAYED_TO));
                    ReplayStep::Parked
                }
            }
        });
        match step {
            ReplayStep::Deliver => {
                let observer = observer.as_mut().expect(OBSERVER_REPLAYED_TO);
                while let Some(value) = batch.pop_front() {
                    // Notify outside the lock
                    if observer.on_next(value).is_break() {
                        close_on_panic.disarm();
                        pipe.close();
                        return;
                    }
                }
            }
            ReplayStep::Parked => {
                close_on_panic.disarm();
                return;
            }
            ReplayStep::Terminate(termination) => {
                // The pipe is closed already, so a panic here needs no guard.
                close_on_panic.disarm();
                let observer = observer.take().expect(OBSERVER_REPLAYED_TO);
                observer.on_termination(termination); // Notify outside the lock
                return;
            }
        }
    }
}

/// What a replay expects of the observer it runs for.
const OBSERVER_REPLAYED_TO: &str = "a replay holds its observer until it parks or terminates it";

#[cfg(test)]
mod tests;
