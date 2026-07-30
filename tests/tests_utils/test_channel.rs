use educe::Educe;
use rx_rust::{
    disposable::{Disposable, chain_disposal::ChainDisposal},
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    safe_lock,
    subject::unicast_subject::{self, UnicastObservable, UnicastSender, unicast_subject},
    utils::types::{MaybeSend, Mutable, MutableHelper, Shared},
};

/// A strict single-consumer channel for the tests.
///
/// The event delivery is done by a unicast subject, so that the tests do not depend on a second
/// implementation of it. On top of that, this channel panics whenever it is used out of order, for
/// example when a value is sent before the subscription or after the end of the channel, which the
/// unicast subject itself allows.
pub(crate) fn test_channel<'or, T, E>() -> (
    SenderObserver<'or, T, E>,
    ReceiverObservable<'or, T, E>,
    ChannelChecker<E>,
) {
    let state = Shared::new(Mutable::new(ChannelState::Initialized));
    let (sender, observable) = unicast_subject();
    (
        SenderObserver {
            sender,
            state: state.clone(),
        },
        ReceiverObservable {
            observable,
            state: state.clone(),
        },
        ChannelChecker(state),
    )
}

pub(crate) struct SenderObserver<'or, T, E> {
    sender: UnicastSender<'or, T, E>,
    state: Shared<Mutable<ChannelState<E>>>,
}

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        self.state.lock_ref(|lock| match &*lock {
            ChannelState::Subscribed => {}
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => {
                drop(lock);
                panic!()
            }
        });
        self.sender.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        // Terminate the channel itself first. Then terminate the observer of the channel.
        let terminated = match &termination {
            Termination::Completed => ChannelState::Completed,
            Termination::Error(error) => ChannelState::Error(error.clone()),
        };
        match safe_lock!(mem_replace: self.state, terminated) {
            ChannelState::Subscribed => {}
            ChannelState::Initialized
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => panic!(),
        }
        self.sender.on_termination(termination);
    }
}

pub(crate) struct ReceiverObservable<'or, T, E> {
    observable: UnicastObservable<'or, T, E>,
    state: Shared<Mutable<ChannelState<E>>>,
}

impl<'or, T, E> Observable<'or, T, E> for ReceiverObservable<'or, T, E> {
    type D = ChainDisposal<ReceiverObservableDisposal<E>, unicast_subject::Disposal<'or, T, E>>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        match safe_lock!(mem_replace: self.state, ChannelState::Subscribed) {
            ChannelState::Initialized => {}
            ChannelState::Subscribed
            | ChannelState::Completed
            | ChannelState::Error(_)
            | ChannelState::Unsubscribed => panic!(),
        }
        self.observable
            .subscribe(observer)
            .preceded_by(ReceiverObservableDisposal(self.state))
    }
}

pub(crate) struct ReceiverObservableDisposal<E>(Shared<Mutable<ChannelState<E>>>);

impl<E> Disposable for ReceiverObservableDisposal<E> {
    fn dispose(self) {
        self.0.lock_mut(|mut lock| match &*lock {
            ChannelState::Initialized | ChannelState::Unsubscribed => {
                drop(lock);
                panic!()
            }
            ChannelState::Subscribed => *lock = ChannelState::Unsubscribed,
            ChannelState::Completed | ChannelState::Error(_) => {}
        });
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelChecker<E>(Shared<Mutable<ChannelState<E>>>);

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
