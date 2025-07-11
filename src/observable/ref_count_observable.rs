use super::{Observable, connectable_observable::ConnectableObservable};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observer::Observer,
    subscription::{Subscription, disposable::Disposable},
};
use educe::Educe;

enum State<'sub> {
    Initialized,
    Subscribed(usize, Subscription<'sub>),
    Unsubscribed,
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct RefCount<'sub, OE, S> {
    source: ConnectableObservable<OE, S>,
    state: Shared<Mutable<State<'sub>>>,
}

impl<OE, S> RefCount<'_, OE, S> {
    pub fn new(source: ConnectableObservable<OE, S>) -> Self {
        Self {
            source,
            state: Shared::new(Mutable::new(State::Initialized)),
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'or, 'sub, T, E> for RefCount<'sub, OE, S>
where
    OE: Observable<'or, 'sub, T, E>,
    S: Observable<'or, 'sub, T, E> + Observer<T, E> + NecessarySend + 'or + Clone,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let mut state_lock = self.state.lock_mut();
        match &mut *state_lock {
            State::Initialized => {
                _ = std::mem::replace(
                    &mut *state_lock,
                    State::Subscribed(1, self.source.clone().connect()),
                );
            }
            State::Subscribed(count, _) => *count += 1,
            State::Unsubscribed => panic!("Already Unsubscribed"),
        };
        drop(state_lock);
        self.source.subscribe(observer) + RefCountDisposal { state: self.state }
    }
}

struct RefCountDisposal<'sub> {
    state: Shared<Mutable<State<'sub>>>,
}

impl Disposable for RefCountDisposal<'_> {
    fn dispose(self) {
        let mut state_lock = self.state.lock_mut();
        match &mut *state_lock {
            State::Initialized => unreachable!(),
            State::Subscribed(count, _) => {
                *count -= 1;
                if *count == 0 {
                    let state = std::mem::replace(&mut *state_lock, State::Unsubscribed);
                    match state {
                        State::Initialized => unreachable!(),
                        State::Subscribed(_, subscription) => {
                            subscription.dispose();
                        }
                        State::Unsubscribed => unreachable!(),
                    }
                }
            }
            State::Unsubscribed => unreachable!(),
        };
    }
}
