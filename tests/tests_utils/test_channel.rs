use educe::Educe;
use rx_rust::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
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
        let mut observer = match &mut *self.0.lock_mut() {
            State::Initialized => panic!(),
            State::Subscribed(boxed_observer) => boxed_observer.take().unwrap(),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => panic!(),
        };
        observer.on_next(value);
        match &mut *self.0.lock_mut() {
            State::Initialized => panic!(),
            State::Subscribed(boxed_observer) => *boxed_observer = Some(observer),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => {}
        };
    }

    fn on_termination(self, termination: Termination<E>) {
        let observer = match std::mem::replace(
            &mut *self.0.lock_mut(),
            State::Terminated(termination.clone()),
        ) {
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
    E: NecessarySend + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        match std::mem::replace(
            &mut *self.0.lock_mut(),
            State::Subscribed(Some(BoxedObserver::new(observer))),
        ) {
            State::Initialized => {}
            State::Subscribed(_) => panic!(),
            State::Terminated(_) => panic!(),
            State::Unsubscribed => panic!(),
        }

        Subscription::new_with_disposal_callback(move || {
            let change = match &*self.0.lock_ref() {
                State::Initialized => panic!(),
                State::Subscribed(_) => true,
                State::Terminated(_) => false,
                State::Unsubscribed => panic!(),
            };
            if change {
                _ = std::mem::replace(&mut *self.0.lock_mut(), State::Unsubscribed);
            }
        })
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelChecker<'or, T, E>(Shared<Mutable<State<'or, T, E>>>);

impl<T, E> ChannelChecker<'_, T, E> {
    pub(crate) fn is_initialized(&self) -> bool {
        matches!(&*self.0.lock_ref(), State::Initialized)
    }

    pub(crate) fn is_subscribed(&self) -> bool {
        matches!(&*self.0.lock_ref(), State::Subscribed(_))
    }

    pub(crate) fn is_completed(&self) -> bool {
        matches!(
            &*self.0.lock_ref(),
            State::Terminated(Termination::Completed)
        )
    }

    pub(crate) fn is_error(&self, expected: E) -> bool
    where
        E: PartialEq,
    {
        matches!(
            &*self.0.lock_ref(),
            State::Terminated(Termination::Error(e)) if *e == expected
        )
    }

    pub(crate) fn is_unsubscribed(&self) -> bool {
        matches!(&*self.0.lock_ref(), State::Unsubscribed)
    }
}
