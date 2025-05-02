use super::{Subject, publish_subject::PublishSubject};
use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
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

    pub fn terminated(&self) -> Option<Terminal<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }

    pub fn value(&self) -> T
    where
        T: Clone,
    {
        self.value.read().unwrap().clone()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
    fn subscribe(self, mut observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.publish_subject.terminated() {
            observer.on_terminal(terminated);
            Subscription::new_none_disposal()
        } else {
            observer.on_next(self.value.read().unwrap().clone());
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
        if self.publish_subject.terminated().is_none() {
            *self.value.write().unwrap() = value.clone();
            self.publish_subject.on_next(value);
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        if self.publish_subject.terminated().is_none() {
            self.publish_subject.on_terminal(terminal);
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
}
