use super::Subject;
use crate::{
    observable::Observable,
    observer::{
        Observer, Termination, boxed_observer::BoxedObserver,
        observer_collection::ObserverCollection,
    },
    subscription::Subscription,
    utils::instant_lock::{InstantMutLock, InstantRefLock},
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

    pub fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.0.lock_ref(|v| match v {
            State::Processing(_) => None,
            State::Terminated(termination) => Some(termination.clone()),
        })
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
        self.0.lock_mut(|v| match v {
            State::Processing(observer_collection) => {
                let key = observer_collection.insert(BoxedObserver::new(observer));

                let this = self.clone();
                Subscription::new_with_disposal_callback(move || {
                    this.0.lock_mut(|v| match v {
                        State::Processing(observer_collection) => {
                            observer_collection.remove(key);
                        }
                        State::Terminated(_) => {}
                    });
                })
            }
            State::Terminated(termination) => {
                observer.on_termination(termination.clone());
                Subscription::new_none_disposal()
            }
        })
    }
}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        self.0.lock_mut(|v| match v {
            State::Processing(observer_collection) => observer_collection.on_next(value),
            State::Terminated(_) => {}
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        if self.terminated().is_some() {
            return;
        }
        let state = std::mem::replace(
            &mut *self.0.lock().unwrap(),
            State::Terminated(termination.clone()),
        );
        match state {
            State::Processing(observer_collection) => {
                observer_collection.on_termination(termination)
            }
            State::Terminated(_) => unreachable!(),
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E, PublishObservable<'or, T, E>>
    for PublishSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn into_observable(self) -> PublishObservable<'or, T, E> {
        PublishObservable(self)
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishObservable<'or, T, E>(PublishSubject<'or, T, E>);

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for PublishObservable<'or, T, E>
where
    T: 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.0.subscribe(observer)
    }
}
