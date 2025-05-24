use super::{Subject, publish_subject::PublishSubject};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::instant_lock::{InstantMutLock, InstantRefLock},
};
use educe::Educe;
use std::sync::{Arc, RwLock};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorSubject<'or, T, E> {
    value: Arc<RwLock<T>>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> BehaviorSubject<'_, T, E> {
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
            publish_subject: PublishSubject::default(),
        }
    }

    pub fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }

    pub fn value(&self) -> T
    where
        T: Clone,
    {
        self.value.lock_ref(T::clone)
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
    fn subscribe(self, mut observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated() {
            observer.on_termination(terminated);
            Subscription::new_none_disposal()
        } else {
            observer.on_next(self.value.lock_ref(T::clone));
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            self.value.lock_mut(|v| *v = value.clone());
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E, BehaviorObservable<'or, T, E>>
    for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
    fn into_observable(self) -> BehaviorObservable<'or, T, E> {
        BehaviorObservable(self)
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorObservable<'or, T, E>(BehaviorSubject<'or, T, E>);

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BehaviorObservable<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.0.subscribe(observer)
    }
}
