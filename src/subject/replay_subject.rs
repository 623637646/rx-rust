use super::{Subject, publish_subject::PublishSubject};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ReplaySubject<'or, T, E> {
    values: Arc<Mutex<VecDeque<T>>>,
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
            values: Arc::new(Mutex::new(vec)),
            buffer_size,
            publish_subject: PublishSubject::default(),
        }
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for ReplaySubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn subscribe(self, mut observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated() {
            match &terminated {
                Termination::Completed => {
                    let values: Vec<_> = self.values.lock().unwrap().iter().cloned().collect();
                    for value in values {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(terminated);
            Subscription::new_none_disposal()
        } else {
            let values: Vec<_> = self.values.lock().unwrap().iter().cloned().collect();
            for value in values {
                observer.on_next(value);
                let _a = self.values.lock().unwrap();
            }
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for ReplaySubject<'_, T, E>
where
    T: Clone,
    E: Clone + Send,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            let mut lock = self.values.lock().unwrap();
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
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }
}
