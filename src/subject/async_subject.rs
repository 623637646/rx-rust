use super::{Subject, publish_subject::PublishSubject};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::sync::{Arc, Mutex};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct AsyncSubject<'or, T, E> {
    value: Arc<Mutex<Option<T>>>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> AsyncSubject<'_, T, E> {
    pub fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
            publish_subject: PublishSubject::default(),
        }
    }
}

impl<T, E> Default for AsyncSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for AsyncSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + Send + 'sub,
    'or: 'sub,
{
    fn subscribe(self, mut observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated() {
            match &terminated {
                Termination::Completed => {
                    if let Some(value) = { self.value.lock().unwrap().clone() } {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(terminated);
            Subscription::new_none_disposal()
        } else {
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for AsyncSubject<'_, T, E>
where
    T: Clone,
    E: Clone + Send,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            *self.value.lock().unwrap() = Some(value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match &termination {
            Termination::Completed => {
                if let Some(value) = { self.value.lock().unwrap().clone() } {
                    self.publish_subject.on_next(value);
                }
            }
            Termination::Error(_) => {}
        }
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for AsyncSubject<'or, T, E>
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
