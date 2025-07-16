use super::{Subject, publish_subject::PublishSubject};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorSubject<'or, T, E> {
    value: Shared<Mutable<T>>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> BehaviorSubject<'_, T, E> {
    pub fn new(value: T) -> Self {
        Self {
            value: Shared::new(Mutable::new(value)),
            publish_subject: PublishSubject::default(),
        }
    }

    pub fn value(&self) -> T
    where
        T: Clone,
    {
        self.value.lock_ref().clone()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + NecessarySend + 'sub,
    'or: 'sub,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated() {
            observer.on_termination(terminated);
            Subscription::default()
        } else {
            observer.on_next(self.value.lock_ref().clone());
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone + NecessarySend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            *self.value.lock_mut() = value.clone();
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + NecessarySend + 'sub,
    'or: 'sub,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }
}
