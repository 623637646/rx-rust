use super::{Subject, publish_subject::PublishSubject};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ReplaySubject<'or, T, E> {
    values: Shared<Mutable<VecDeque<T>>>,
    buffer_size: Option<usize>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> ReplaySubject<'_, T, E> {
    pub fn new(buffer_size: Option<usize>) -> Self {
        let vec = match buffer_size {
            Some(size) => VecDeque::with_capacity(size),
            None => VecDeque::new(),
        };
        Self {
            values: Shared::new(Mutable::new(vec)),
            buffer_size,
            publish_subject: PublishSubject::default(),
        }
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for ReplaySubject<'or, T, E>
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
            match &terminated {
                Termination::Completed => {
                    let values: Vec<_> = self.values.lock_ref().iter().cloned().collect();
                    for value in values {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(terminated);
            Subscription::default()
        } else {
            let values: Vec<_> = self.values.lock_ref().iter().cloned().collect();
            for value in values {
                observer.on_next(value);
            }
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for ReplaySubject<'_, T, E>
where
    T: Clone,
    E: Clone + NecessarySend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            let mut lock = self.values.lock_mut();
            if let Some(buffer_size) = self.buffer_size {
                if lock.len() == buffer_size {
                    if lock.pop_front().is_some() {
                        // only push if the buffer is not 0
                        lock.push_back(value.clone());
                    }
                } else {
                    lock.push_back(value.clone());
                }
            } else {
                lock.push_back(value.clone());
            }
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for ReplaySubject<'or, T, E>
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
