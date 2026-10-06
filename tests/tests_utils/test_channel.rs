use educe::Educe;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableTypes, Subscription},
    observer::boxed_observer::{IntoBoxedObserver, ObserverMode},
    observer::{Flow, Observer, Termination},
    utils::{pending_events::EventBatch, serialized_delivery::SerializedDelivery},
};
use std::mem;
use std::sync::{Arc, Mutex};

mod channels;

// Every test binary compiles the helpers and most never open several channels: the same reason
// `tests_utils` allows `dead_code`.
#[allow(unused_imports)]
pub(crate) use channels::test_channels;

/// A strict single-consumer channel for the tests.
///
/// It is deliberately not built on any observable or subject of this crate. It is the source that
/// drives almost every test, so sharing an implementation with the code under test would let a
/// change there fail the tests of the operators that have nothing to do with it, and would let a
/// bug of that implementation hide itself behind the very tests that should catch it.
///
/// The one thing it does share is the [`SerializedDelivery`] the observer is parked in: that is
/// what lets an event be delivered with no lock of this module held, and it is covered by its own
/// tests. Everything above it is kept dumb: the channel buffers nothing and replays nothing. It
/// panics whenever it is used out of order, which is what a test double is for: subscribing twice,
/// disposing twice, or sending a value before the subscription, after the end of the channel, or
/// after the subscription was disposed.
///
/// A value sent from inside the delivery of another one is not out of order: the delivery the
/// observer is parked in serializes it behind the one that is running. That is what lets a test
/// re-enter the pipeline from a callback, or from the drop of a value.
pub(crate) fn test_channel<'or, T, E>() -> (
    SenderObserver<'or, T, E>,
    ReceiverObservable<'or, T, E>,
    ChannelChecker<'or, T, E>,
) {
    let channel = new_channel();
    (
        SenderObserver {
            channel: channel.clone(),
        },
        ReceiverObservable {
            channel: channel.clone(),
        },
        ChannelChecker(channel),
    )
}

fn new_channel<'or, T, E>() -> SharedChannel<'or, T, E> {
    Arc::new(Mutex::new(Channel::Initialized))
}

/// Everything the channel owns, behind the single lock of this module: its state, which says
/// whether a use of the channel is legal, and, while it is subscribed to, the delivery the observer
/// is parked in. [`ChannelState`] is what [`ChannelChecker`] reads of it, without the delivery.
///
/// The delivery lives in [`Channel::Subscribed`], so that a channel holds one exactly while it is
/// subscribed to: the subscription parks it, and whichever of the termination and the disposal
/// comes first takes it out.
///
/// The delivery handle is cloned or taken out under the lock and used after it was released, so
/// nothing is notified nor dropped while the lock is held: the values of the tests run arbitrary
/// code when they are dropped, which is what the tests use to re-enter the pipeline.
enum Channel<'or, T, E> {
    Initialized,
    Subscribed(Delivery<'or, T, E>),
    Completed,
    Error(E),
    Unsubscribed,
}

impl<T, E> Channel<'_, T, E>
where
    E: Clone,
{
    fn state(&self) -> ChannelState<E> {
        match self {
            Channel::Initialized => ChannelState::Initialized,
            Channel::Subscribed(_) => ChannelState::Subscribed,
            Channel::Completed => ChannelState::Completed,
            Channel::Error(error) => ChannelState::Error(error.clone()),
            Channel::Unsubscribed => ChannelState::Unsubscribed,
        }
    }
}

type SharedChannel<'or, T, E> = Arc<Mutex<Channel<'or, T, E>>>;

type Delivery<'or, T, E> = SerializedDelivery<Shared, T, E, SendBoxedObserver<'or, T, E>, ()>;

pub(crate) struct SenderObserver<'or, T, E> {
    channel: SharedChannel<'or, T, E>,
}

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) -> Flow {
        let delivery = self.channel.with_ref(|channel| match channel {
            Channel::Subscribed(delivery) => Some(delivery.clone()),
            Channel::Initialized
            | Channel::Completed
            | Channel::Error(_)
            | Channel::Unsubscribed => None,
        });
        // Panic outside the lock, which leaves it usable, and drops the value outside it too.
        let delivery = delivery.expect("the channel takes a value only while it is subscribed to");
        // A subscribed channel always has an observer to deliver to, so the flow says whether
        // the stream ended during this very delivery: by a disposal or a termination from inside
        // it, or by the observer itself. It is handed to whoever drives the channel.
        delivery.send(EventBatch::Next(value)) // Notify outside the lock
    }

    fn on_termination(self, termination: Termination<E>) {
        let terminated = match &termination {
            Termination::Completed => Channel::Completed,
            Termination::Error(error) => Channel::Error(error.clone()),
        };
        // The channel ends before the observer it notifies does, so that a re-entrant use of it
        // sees a channel that is over.
        let outcome = self
            .channel
            .with_mut(|channel| match mem::replace(channel, terminated) {
                Channel::Subscribed(delivery) => Ok(delivery),
                // Puts the state back, and hands the refused one out to be dropped outside the
                // lock.
                previous => Err(mem::replace(channel, previous)),
            });
        // Panic outside the lock, which leaves it usable, and drops the termination outside it too.
        let Ok(delivery) = outcome else {
            panic!("the channel ends only while it is subscribed to");
        };
        // A subscribed channel always has an observer to terminate, and terminating ends the
        // delivery, so the flow it answers is `Flow::Stop` either way and says nothing more.
        let _ = delivery.send(EventBatch::Termination(termination)); // Notify outside the lock
    }
}

pub(crate) struct ReceiverObservable<'or, T, E> {
    channel: SharedChannel<'or, T, E>,
}

impl<'or, T, E> ObservableTypes for ReceiverObservable<'or, T, E> {
    type Item = T;
    type Error = E;
    type Mode = Shared;
    type Disposal = ReceiverObservableDisposal<'or, T, E>;
}

impl<'or, T, E, OR> Observable<OR> for ReceiverObservable<'or, T, E>
where
    OR: IntoBoxedObserver<'or, T, E, Shared>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        // Kept out of the closure below, so that a refused subscription drops the observer outside
        // the lock, while the assertion unwinds.
        let mut observer = Some(Shared::boxed(observer));
        let initialized = self.channel.with_mut(|channel| match channel {
            Channel::Initialized => {
                *channel = Channel::Subscribed(SerializedDelivery::idle(
                    observer.take().expect("the observer is parked once"),
                    (),
                ));
                true
            }
            Channel::Subscribed(_)
            | Channel::Completed
            | Channel::Error(_)
            | Channel::Unsubscribed => false,
        });
        // Panic outside the lock, which leaves it usable.
        assert!(initialized, "the channel is subscribed to only once");

        Subscription::new(ReceiverObservableDisposal {
            channel: self.channel,
        })
    }
}

pub(crate) struct ReceiverObservableDisposal<'or, T, E> {
    channel: SharedChannel<'or, T, E>,
}

impl<T, E> Disposable for ReceiverObservableDisposal<'_, T, E> {
    fn dispose(self) {
        // The channel is closed before the observer is released, so that a re-entrant use of it
        // sees a channel that is over.
        let outcome = self.channel.with_mut(|channel| {
            match mem::replace(channel, Channel::Unsubscribed) {
                Channel::Subscribed(delivery) => Ok(Some(delivery)),
                // The channel ended on its own, which already released the delivery.
                previous @ (Channel::Completed | Channel::Error(_)) => {
                    *channel = previous;
                    Ok(None)
                }
                previous @ (Channel::Initialized | Channel::Unsubscribed) => {
                    *channel = previous;
                    Err("the channel is disposed only once, and only after being subscribed to")
                }
            }
        });
        match outcome {
            // The observer is dropped outside the lock, unless a value is being delivered to it:
            // the delivery drops it as soon as it looks for its next event.
            Ok(Some(delivery)) => delivery.stop(),
            Ok(None) => {}
            Err(message) => panic!("{message}"), // Panic outside the lock, which leaves it usable
        }
    }
}

/// The read-only end of the channel, which reads the state and nothing else, so it stays usable
/// after the channel released its observer.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelChecker<'or, T, E>(SharedChannel<'or, T, E>);

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChannelState<E> {
    Initialized,
    Subscribed,
    Completed,
    Error(E),
    Unsubscribed,
}

impl<T, E> ChannelChecker<'_, T, E> {
    pub(crate) fn state(&self) -> ChannelState<E>
    where
        E: Clone,
    {
        self.0.with_ref(Channel::state)
    }
}
