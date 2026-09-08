use crate::tests_utils::shared_sender::SharedSender;
use educe::Educe;
use rx_rust::utils::mutable::MutableExt;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    utils::mutable::{Mutable, MutableHelper},
    utils::types::{MaybeSend, Shared},
};

/// A strict single-consumer channel for the tests.
///
/// It is deliberately not built on any observable or subject of this crate. It is the source that
/// drives almost every test, so sharing an implementation with the code under test would let a
/// change there fail the tests of the operators that have nothing to do with it, and would let a
/// bug of that implementation hide itself behind the very tests that should catch it.
///
/// The one thing it does share is the delivery the observer is parked in, through the
/// [`SharedSender`] below: that is what lets an event be delivered with no lock of this module
/// held, and it is covered by its own tests. Everything above it is kept dumb: the channel holds
/// nothing, buffers nothing and replays nothing. It panics whenever it is used out of order, which
/// is what a test double is for: subscribing twice, disposing twice, or sending a value before the
/// subscription, after the end of the channel, or after the subscription was disposed.
///
/// A value sent from inside the delivery of another one is not out of order: the delivery the
/// observer is parked in serializes it behind the one that is running. That is what lets a test
/// re-enter the pipeline from a callback, or from the drop of a value.
pub(crate) fn test_channel<'or, T, E>() -> (
    SenderObserver<'or, T, E>,
    ReceiverObservable<'or, T, E>,
    ChannelChecker<E>,
) {
    let state = Shared::new(Mutable::new(ChannelState::Initialized));
    let sender = Sender::default();
    (
        SenderObserver {
            state: state.clone(),
            sender: sender.clone(),
        },
        ReceiverObservable {
            state: state.clone(),
            sender,
        },
        ChannelChecker(state),
    )
}

/// The state of the channel, which is what says whether a use of it is legal, and what
/// [`ChannelChecker`] reads.
///
/// The observer is not here: it is parked in the [`Sender`] below, which delivers to it with no
/// lock of this module held. The channel therefore never holds two locks at once, and the checker
/// does not have to name the type of the values.
type SharedState<E> = Shared<Mutable<ChannelState<E>>>;

/// The observer of the channel, which is empty until the subscription and once the channel is
/// over. Nothing is notified nor dropped while the state above is locked: the values of the tests
/// run arbitrary code when they are dropped, which is what the tests use to re-enter the pipeline.
type Sender<'or, T, E> = SharedSender<T, E, BoxedObserver<'or, T, E>>;

pub(crate) struct SenderObserver<'or, T, E> {
    state: SharedState<E>,
    sender: Sender<'or, T, E>,
}

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let subscribed = self
            .state
            .with_ref(|lock| matches!(*lock, ChannelState::Subscribed));
        // Panic outside the lock, which leaves it usable, and drops the value outside it too.
        assert!(
            subscribed,
            "the channel takes a value only while it is subscribed to"
        );
        let delivered = self.sender.on_next(value); // Notify outside the lock
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
        let subscribed = self.state.with_mut(|lock| match &*lock {
            ChannelState::Subscribed => {
                *lock = terminated;
                true
            }
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => false,
        });
        // Panic outside the lock, which leaves it usable.
        assert!(
            subscribed,
            "the channel ends only while it is subscribed to"
        );
        let notified = self.sender.on_termination(termination); // Notify outside the lock
        debug_assert!(
            notified,
            "a subscribed channel has an observer to terminate"
        );
    }
}

pub(crate) struct ReceiverObservable<'or, T, E> {
    state: SharedState<E>,
    sender: Sender<'or, T, E>,
}

impl<'or, T, E> Observable<'or, T, E> for ReceiverObservable<'or, T, E> {
    type D = ReceiverObservableDisposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = BoxedObserver::new(observer);
        let initialized = self.state.with_mut(|lock| match &*lock {
            ChannelState::Initialized => {
                *lock = ChannelState::Subscribed;
                true
            }
            ChannelState::Subscribed
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => false,
        });
        // Panic outside the lock, which leaves it usable. The observer is dropped by the
        // unwinding, which the line above keeps outside of the lock too.
        assert!(initialized, "the channel is subscribed to only once");

        // The channel says it is subscribed to before the observer is parked, so the two are not
        // one step. Nothing can send in between: the channel has a single producer, and it is the
        // one that is subscribing right now.
        let was_empty = self.sender.set(observer);
        debug_assert!(was_empty, "the channel parks one observer at a time");

        Subscription::new(ReceiverObservableDisposal {
            state: self.state,
            sender: self.sender,
        })
    }
}

pub(crate) struct ReceiverObservableDisposal<'or, T, E> {
    state: SharedState<E>,
    sender: Sender<'or, T, E>,
}

impl<T, E> Disposable for ReceiverObservableDisposal<'_, T, E> {
    fn dispose(self) {
        // The channel is closed before the observer is released, so that a re-entrant use of it
        // sees a channel that is over.
        let outcome = self.state.with_mut(|lock| match &*lock {
            ChannelState::Subscribed => {
                *lock = ChannelState::Unsubscribed;
                Ok(true)
            }
            // The channel ended on its own, which already emptied the sender.
            ChannelState::Completed | ChannelState::Error(_) => Ok(false),
            ChannelState::Initialized | ChannelState::Unsubscribed => {
                Err("the channel is disposed only once, and only after being subscribed to")
            }
        });
        match outcome {
            // The observer is dropped outside the lock, unless a value is being delivered to it:
            // the delivery drops it as soon as it looks for its next event.
            Ok(true) => {
                self.sender.stop();
            }
            Ok(false) => {}
            Err(message) => panic!("{message}"), // Panic outside the lock, which leaves it usable
        }
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelChecker<E>(SharedState<E>);

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChannelState<E> {
    Initialized,
    Subscribed,
    Completed,
    Error(E),
    Unsubscribed,
}

impl<E> ChannelChecker<E> {
    pub(crate) fn state(&self) -> ChannelState<E>
    where
        E: Clone,
    {
        self.0.clone_value()
    }
}
