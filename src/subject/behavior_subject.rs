use super::{Subject, publish_subject::PublishSubject};
use crate::safe_lock;
use crate::utils::types::{Mutable, MaybeSend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Keeps the latest value and emits it immediately to new subscribers.
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
        safe_lock!(clone: self.value)
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + MaybeSend + 'sub,
    E: Clone + MaybeSend + 'sub,
    'or: 'sub,
{
    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated() {
            observer.on_termination(terminated);
            Subscription::default()
        } else {
            observer.on_next(safe_lock!(clone: self.value));
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            safe_lock!(set: self.value, value.clone());
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for BehaviorSubject<'or, T, E>
where
    T: Clone + MaybeSend + 'sub,
    E: Clone + MaybeSend + 'sub,
    'or: 'sub,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }
}
