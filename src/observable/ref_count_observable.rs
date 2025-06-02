use super::{Observable, connectable_observable::ConnectableObservable};
use crate::{observer::Observer, subscription::Subscription};
use educe::Educe;
use std::sync::{Arc, Mutex};

enum State<'sub> {
    Initialized,
    Subscribed(usize, Subscription<'sub>),
    Unsubscribed,
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct RefCount<'sub, OE, S> {
    source: ConnectableObservable<OE, S>,
    state: Arc<Mutex<State<'sub>>>,
}

impl<OE, S> RefCount<'_, OE, S> {
    pub fn new(source: ConnectableObservable<OE, S>) -> Self {
        Self {
            source,
            state: Arc::new(Mutex::new(State::Initialized)),
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'or, 'sub, T, E> for RefCount<'sub, OE, S>
where
    OE: Observable<'or, 'sub, T, E>,
    S: Observable<'or, 'sub, T, E> + Observer<T, E> + Send + 'or + Clone,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let source_cloned = self.source.clone();
        let sub = self.source.subscribe(observer);
        let mut state_lock = self.state.lock().unwrap();
        match &mut *state_lock {
            State::Initialized => {
                _ = std::mem::replace(
                    &mut *state_lock,
                    State::Subscribed(1, source_cloned.connect()),
                );
            }
            State::Subscribed(count, _) => *count += 1,
            State::Unsubscribed => panic!("Already Unsubscribed"),
        };

        let state_cloned = self.state.clone();
        sub + Subscription::new_with_disposal_callback(move || {
            let mut state_lock = state_cloned.lock().unwrap();
            match &mut *state_lock {
                State::Initialized => unreachable!(),
                State::Subscribed(count, _) => {
                    *count -= 1;
                    if *count == 0 {
                        let state = std::mem::replace(&mut *state_lock, State::Unsubscribed);
                        match state {
                            State::Initialized => unreachable!(),
                            State::Subscribed(_, subscription) => {
                                subscription.unsubscribe();
                            }
                            State::Unsubscribed => unreachable!(),
                        }
                    }
                }
                State::Unsubscribed => unreachable!(),
            };
        })
    }
}
