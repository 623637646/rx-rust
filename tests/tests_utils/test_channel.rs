use educe::Educe;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    utils::{
        mutable::{Mutable, MutableHelper},
        pending_events::EventBatch,
        serialized_delivery::SerializedDelivery,
        types::{MaybeSend, Shared},
    },
};

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
    let channel = Shared::new(Mutable::new(Channel {
        state: ChannelState::Initialized,
        delivery: None,
    }));
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

/// Everything the channel owns, behind the single lock of this module.
///
/// The state is what says whether a use of the channel is legal, and what [`ChannelChecker`]
/// reads; the delivery is where the observer is parked. They are one cell because every use of the
/// channel reads the first to reach the second, and a lock taken once cannot be taken in the wrong
/// order. `delivery` is `Some` exactly while `state` is [`ChannelState::Subscribed`]: it is filled
/// by the subscription and emptied by whichever of the termination and the disposal comes first.
///
/// The delivery handle is cloned or taken out under the lock and used after it was released, so
/// nothing is notified nor dropped while the lock is held: the values of the tests run arbitrary
/// code when they are dropped, which is what the tests use to re-enter the pipeline.
struct Channel<'or, T, E> {
    state: ChannelState<E>,
    delivery: Option<Delivery<'or, T, E>>,
}

type SharedChannel<'or, T, E> = Shared<Mutable<Channel<'or, T, E>>>;

type Delivery<'or, T, E> = SerializedDelivery<T, E, BoxedObserver<'or, T, E>, ()>;

pub(crate) struct SenderObserver<'or, T, E> {
    channel: SharedChannel<'or, T, E>,
}

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        // A parked delivery is a subscribed channel, so this asks the two questions at once.
        let delivery = self.channel.with_ref(|channel| channel.delivery.clone());
        // Panic outside the lock, which leaves it usable, and drops the value outside it too.
        let delivery = delivery.expect("the channel takes a value only while it is subscribed to");
        let delivered = delivery.send(EventBatch::Next(value)); // Notify outside the lock
        debug_assert!(
            delivered,
            "a subscribed channel has an observer to deliver to"
        );
    }

    fn on_termination(self, termination: Termination<E>) {
        let terminated = match &termination {
            Termination::Completed => ChannelState::Completed,
            Termination::Error(error) => ChannelState::Error(error.clone()),
        };
        // The channel ends before the observer it notifies does, so that a re-entrant use of it
        // sees a channel that is over.
        let delivery = self.channel.with_mut(|channel| match &channel.state {
            ChannelState::Subscribed => {
                channel.state = terminated;
                channel.delivery.take()
            }
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => None,
        });
        // Panic outside the lock, which leaves it usable, and drops the termination outside it too.
        let delivery = delivery.expect("the channel ends only while it is subscribed to");
        let notified = delivery.send(EventBatch::Termination(termination)); // Notify outside the lock
        debug_assert!(
            notified,
            "a subscribed channel has an observer to terminate"
        );
    }
}

pub(crate) struct ReceiverObservable<'or, T, E> {
    channel: SharedChannel<'or, T, E>,
}

impl<'or, T, E> Observable<'or, T, E> for ReceiverObservable<'or, T, E> {
    type D = ReceiverObservableDisposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        // Kept out of the closure below, so that a refused subscription drops the observer outside
        // the lock, while the assertion unwinds.
        let mut observer = Some(BoxedObserver::new(observer));
        let initialized = self.channel.with_mut(|channel| match &channel.state {
            ChannelState::Initialized => {
                // The channel says it is subscribed to and parks the observer in one step, which
                // the single lock of this module makes free: nothing could send in between anyway,
                // as the channel has a single producer and it is the one subscribing right now.
                channel.state = ChannelState::Subscribed;
                channel.delivery = Some(SerializedDelivery::idle(
                    observer.take().expect("the observer is parked once"),
                    (),
                ));
                true
            }
            ChannelState::Subscribed
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => false,
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
        let outcome = self.channel.with_mut(|channel| match &channel.state {
            ChannelState::Subscribed => {
                channel.state = ChannelState::Unsubscribed;
                Ok(channel.delivery.take())
            }
            // The channel ended on its own, which already released the delivery.
            ChannelState::Completed | ChannelState::Error(_) => Ok(None),
            ChannelState::Initialized | ChannelState::Unsubscribed => {
                Err("the channel is disposed only once, and only after being subscribed to")
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
        self.0.with_ref(|channel| channel.state.clone())
    }
}
