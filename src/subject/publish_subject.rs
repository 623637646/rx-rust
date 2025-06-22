use super::Subject;
use crate::{
    observable::Observable,
    observer::{
        Observer, Termination,
        boxed_observer::BoxedObserver,
        observer_collection::{Key, ObserverCollection},
    },
    subscription::{Subscription, disposable::Disposable},
};
use educe::Educe;
use std::sync::{Arc, Mutex};

enum State<'or, T, E> {
    Processing(ObserverCollection<BoxedObserver<'or, T, E>>),
    Terminated(Termination<E>),
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E>(Arc<Mutex<State<'or, T, E>>>);

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(State::Processing(
            ObserverCollection::new(),
        ))))
    }
}

impl<T, E> Default for PublishSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let mut lock = self.0.lock().unwrap();
        match &mut *lock {
            State::Processing(observer_collection) => {
                let key = observer_collection.insert(BoxedObserver::new(observer));
                drop(lock);
                Subscription::new_with_disposal(PublishSubjectDisposal { state: self.0, key })
            }
            State::Terminated(termination) => {
                let termination = termination.clone();
                drop(lock);
                observer.on_termination(termination);
                Subscription::new_none_disposal()
            }
        }
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.0.lock().unwrap();
        match &mut *lock {
            State::Processing(observer_collection) => {
                if let Some(mut observer_collection_agent) = observer_collection.borrow_agent() {
                    drop(lock);
                    observer_collection_agent.on_next(value);
                    let mut lock = self.0.lock().unwrap();
                    match &mut *lock {
                        State::Processing(observer_collection) => {
                            observer_collection.return_agent(observer_collection_agent)
                        }
                        State::Terminated(termination) => {
                            let termination = termination.clone();
                            drop(lock);
                            observer_collection_agent.on_termination(termination);
                        }
                    };
                } else {
                    panic!("No support for regression calls on_next");
                }
            }
            State::Terminated(_) => {}
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut lock = self.0.lock().unwrap();
        match &mut *lock {
            State::Processing(observer_collection) => {
                if let Some(observer_collection_agent) = observer_collection.borrow_agent() {
                    drop(lock);
                    _ = std::mem::replace(
                        &mut *self.0.lock().unwrap(),
                        State::Terminated(termination.clone()),
                    );
                    observer_collection_agent.on_termination(termination);
                } else {
                    drop(lock);
                    _ = std::mem::replace(
                        &mut *self.0.lock().unwrap(),
                        State::Terminated(termination.clone()),
                    );
                }
            }
            State::Terminated(_) => {}
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        match &*self.0.lock().unwrap() {
            State::Processing(_) => None,
            State::Terminated(termination) => Some(termination.clone()),
        }
    }
}

struct PublishSubjectDisposal<'or, T, E> {
    state: Arc<Mutex<State<'or, T, E>>>,
    key: Key,
}

impl<T, E> Disposable for PublishSubjectDisposal<'_, T, E> {
    fn dispose(self) {
        match &mut *self.state.lock().unwrap() {
            State::Processing(observer_collection) => {
                observer_collection.remove(self.key);
            }
            State::Terminated(_) => {}
        };
    }
}
