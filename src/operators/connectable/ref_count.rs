use super::connectable_observable::ConnectableObservable;
use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::observable::Observable;
use crate::observer::Observer;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
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
    S: Observable<'or, 'sub, T, E> + Observer<T, E> + Clone + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let sub = self.source.clone().subscribe(observer);
        self.state.lock_mut(|mut lock| {
            match &mut *lock {
                State::Initialized => {
                    *lock = State::Subscribed(1, self.source.connect());
                }
                State::Subscribed(count, _) => *count += 1,
                State::Unsubscribed => panic!("Already Unsubscribed"),
            };
        });
        sub + self.state
    }
}

impl Disposable for Shared<Mutable<State<'_>>> {
    fn dispose(self) {
        self.lock_mut(|mut lock| match &mut *lock {
            State::Initialized => unreachable!(),
            State::Subscribed(count, _) => {
                *count -= 1;
                if *count == 0 {
                    let state = std::mem::replace(&mut *lock, State::Unsubscribed);
                    drop(lock);
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
        });
    }
}
