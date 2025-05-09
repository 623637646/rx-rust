use super::Subject;
use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    subscription::Subscription,
    utils::unique_key_store::UniqueKeyStore,
};
use educe::Educe;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E> {
    observers: Arc<Mutex<UniqueKeyStore<BoxedObserver<'or, T, E>>>>,
    terminated: Arc<RwLock<Option<Termination<E>>>>,
}

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self {
            observers: Arc::new(Mutex::new(UniqueKeyStore::new())),
            terminated: Arc::new(RwLock::new(None)),
        }
    }

    pub fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.terminated.read().unwrap().as_ref().cloned()
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
    E: Clone + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated.read().unwrap().as_ref().cloned() {
            observer.on_termination(terminated);
            return Subscription::new_none_disposal();
        }
        let observers = self.observers;
        let key = observers
            .lock()
            .unwrap()
            .insert(BoxedObserver::new(observer));
        Subscription::new_with_disposal_callback(move || {
            observers.lock().unwrap().remove(key);
        })
    }
}

impl<T, E> ObservableExt for PublishSubject<'_, T, E> {}

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        for observer in self.observers.lock().unwrap().iter_mut() {
            observer.on_next(value.clone());
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut terminated = self.terminated.write().unwrap();
        if terminated.is_some() {
            return;
        }
        *terminated = Some(termination.clone());
        for observer in self.observers.lock().unwrap().drain() {
            observer.on_termination(termination.clone());
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
}
