use crate::tests_utils::types::TestMutableHelper;
use educe::Educe;
use rx_rust::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    safe_lock,
    utils::types::{MaybeSend, Mutable, MutableHelper, Shared},
};

enum State<'or, T, E> {
    Initialized,
    Subscribed(Option<BoxedObserver<'or, T, E>>),
    Terminated(Termination<E>),
    Unsubscribed,
}

pub(crate) fn test_channel<'or, T, E>() -> (
    SenderObserver<'or, T, E>,
    ReceiverObservable<'or, T, E>,
    ChannelChecker<'or, T, E>,
) {
    let state = Shared::new(Mutable::new(State::Initialized));
    (
        SenderObserver(state.clone()),
        ReceiverObservable(state.clone()),
        ChannelChecker(state),
    )
}

pub(crate) struct SenderObserver<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

impl<T, E> Observer<T, E> for SenderObserver<'_, T, E>
where
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let mut observer = match safe_lock!(mem_replace: self.0, State::Subscribed(None)) {
            State::Initialized => panic!(),
            State::Subscribed(boxed_observer) => boxed_observer.unwrap(),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => panic!(),
        };

        observer.on_next(value);
        self.0.lock_mut(|mut lock| match &mut *lock {
            State::Initialized => panic!(),
            State::Subscribed(boxed_observer) => *boxed_observer = Some(observer),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => {}
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let observer = match safe_lock!(mem_replace: self.0, State::Terminated(termination.clone()))
        {
            State::Initialized => panic!(),
            State::Subscribed(boxed_observer) => boxed_observer.unwrap(),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => panic!(),
        };
        observer.on_termination(termination);
    }
}

pub(crate) struct ReceiverObservable<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for ReceiverObservable<'or, T, E>
where
    T: 'sub,
    E: MaybeSend + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<'sub> {
        match safe_lock!(mem_replace:
            self.0,
            State::Subscribed(Some(BoxedObserver::new(observer)))
        ) {
            State::Initialized => {}
            State::Subscribed(_) => panic!(),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => panic!(),
        }

        Subscription::new_with_disposal_callback(move || {
            self.0.lock_mut(|mut lock| match &mut *lock {
                State::Initialized | State::Unsubscribed => {
                    drop(lock);
                    panic!()
                }
                State::Subscribed(boxed_observer) => {
                    let boxed_observer = boxed_observer.take();
                    *lock = State::Unsubscribed;
                    drop(lock);
                    drop(boxed_observer) // Drop outside the lock to avoid potential deadlock
                }
                State::Terminated(_) => {
                    drop(lock);
                }
            });
        })
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelChecker<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

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
        match &*self.0.test_lock_ref() {
            State::Initialized => ChannelState::Initialized,
            State::Subscribed(_) => ChannelState::Subscribed,
            State::Terminated(termination) => match termination {
                Termination::Completed => ChannelState::Completed,
                Termination::Error(error) => ChannelState::Error(error.clone()),
            },
            State::Unsubscribed => ChannelState::Unsubscribed,
        }
    }
}
