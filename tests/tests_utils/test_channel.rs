use educe::Educe;
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    safe_lock, safe_lock_option,
    utils::types::{MaybeSend, Mutable, MutableHelper, Shared},
};

/// A strict single-consumer channel for the tests.
///
/// It is deliberately not built on any observable of this crate. It is the source that drives
/// almost every test, so sharing an implementation with the code under test would let a change
/// there fail the tests of the operators that have nothing to do with it, and would let a bug of
/// that implementation hide itself behind the very tests that should catch it.
///
/// It is kept dumb instead: it holds nothing, buffers nothing and replays nothing. It panics
/// whenever it is used out of order, which is what a test double is for: subscribing twice, or
/// sending a value before the subscription, after the end of the channel, after the subscription
/// was disposed, or from inside the delivery of another value.
pub(crate) fn test_channel<'or, T, E>() -> (
    SenderObserver<'or, T, E>,
    ReceiverObservable<'or, T, E>,
    ChannelChecker<E>,
) {
    let state = Shared::new(Mutable::new(ChannelState::Initialized));
    let observer = Shared::new(Mutable::new(None));
    (
        SenderObserver {
            state: state.clone(),
            observer: observer.clone(),
        },
        ReceiverObservable {
            state: state.clone(),
            observer,
        },
        ChannelChecker(state),
    )
}

/// The state of the channel, which is what [`ChannelChecker`] reads. It is kept apart from the
/// observer below so that the checker does not have to name the type of the values.
type SharedState<E> = Shared<Mutable<ChannelState<E>>>;

/// The observer of the channel, which is empty until the subscription, while a value is being
/// delivered to it, and once it is gone.
///
/// It is only ever locked while the state above is locked, so that the two of them are read and
/// written as one. Nothing is dropped nor notified while either of them is locked: the values of
/// the tests run arbitrary code when they are dropped, which is what the tests use to re-enter the
/// pipeline.
type SharedObserver<'or, T, E> = Shared<Mutable<Option<BoxedObserver<'or, T, E>>>>;

pub(crate) struct SenderObserver<'or, T, E> {
    state: SharedState<E>,
    observer: SharedObserver<'or, T, E>,
}

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        // The observer is taken out of its slot while it is notified, because it is notified
        // outside the lock: a re-entrant send then finds the slot empty instead of aliasing it.
        let observer = self.state.lock_ref(|lock| match &*lock {
            ChannelState::Subscribed => safe_lock_option!(take: self.observer),
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => {
                drop(lock);
                panic!("the channel takes a value only while it is subscribed to")
            }
        });
        let mut observer =
            observer.expect("the channel takes no value while it is delivering another one");
        observer.on_next(value); // Notify outside the lock

        // A disposal that ran while the value was being delivered found the slot empty, so the
        // observer is dropped here instead: this is the earliest the channel can let go of it.
        let leftover = self.state.lock_ref(|lock| match &*lock {
            ChannelState::Subscribed => {
                let previous = safe_lock_option!(replace: self.observer, observer);
                debug_assert!(previous.is_none());
                None
            }
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => Some(observer),
        });
        drop(leftover); // Drop outside the lock to avoid potential deadlock
    }

    fn on_termination(self, termination: Termination<E>) {
        let terminated = match &termination {
            Termination::Completed => ChannelState::Completed,
            Termination::Error(error) => ChannelState::Error(error.clone()),
        };
        // The channel ends before the observer it notifies does.
        let observer = self.state.lock_mut(|mut lock| match &*lock {
            ChannelState::Subscribed => {
                *lock = terminated;
                safe_lock_option!(take: self.observer)
            }
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => {
                drop(lock);
                panic!("the channel ends only while it is subscribed to")
            }
        });
        let observer = observer.expect("the channel does not end while it is delivering a value");
        observer.on_termination(termination); // Notify outside the lock
    }
}

pub(crate) struct ReceiverObservable<'or, T, E> {
    state: SharedState<E>,
    observer: SharedObserver<'or, T, E>,
}

impl<'or, T, E> Observable<'or, T, E> for ReceiverObservable<'or, T, E> {
    type D = ReceiverObservableDisposal<'or, T, E>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = BoxedObserver::new(observer);
        self.state.lock_mut(|mut lock| match &*lock {
            ChannelState::Initialized => {
                *lock = ChannelState::Subscribed;
                let previous = safe_lock_option!(replace: self.observer, observer);
                debug_assert!(previous.is_none());
            }
            ChannelState::Subscribed
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => {
                drop(lock);
                // The observer is dropped by the unwinding, which the line above keeps outside of
                // the lock.
                panic!("the channel is subscribed to only once")
            }
        });
        Subscription::new(ReceiverObservableDisposal {
            state: self.state,
            observer: self.observer,
        })
    }
}

pub(crate) struct ReceiverObservableDisposal<'or, T, E> {
    state: SharedState<E>,
    observer: SharedObserver<'or, T, E>,
}

impl<T, E> Disposable for ReceiverObservableDisposal<'_, T, E> {
    fn dispose(self) {
        // The observer is released here, unless a value is being delivered to it: the slot is
        // empty then, and the sender drops it as soon as that delivery is over.
        let observer = self.state.lock_mut(|mut lock| match &*lock {
            ChannelState::Subscribed => {
                *lock = ChannelState::Unsubscribed;
                safe_lock_option!(take: self.observer)
            }
            // The channel ended on its own, which already took the observer out.
            ChannelState::Completed | ChannelState::Error(_) => None,
            ChannelState::Initialized | ChannelState::Unsubscribed => {
                drop(lock);
                panic!("the channel is disposed only once, and only after being subscribed to")
            }
        });
        drop(observer); // Drop outside the lock to avoid potential deadlock
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
        safe_lock!(clone: self.0)
    }
}
