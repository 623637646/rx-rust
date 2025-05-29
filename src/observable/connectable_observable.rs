use super::Observable;
use crate::{observer::Observer, subscription::Subscription};
use educe::Educe;
use std::sync::{Arc, Mutex};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConnectableObservable<OE, S> {
    source: Arc<Mutex<Option<OE>>>,
    subject: S,
}

impl<OE, S> ConnectableObservable<OE, S> {
    pub fn new(source: OE) -> Self
    where
        S: Default,
    {
        Self {
            source: Arc::new(Mutex::new(Some(source))),
            subject: <_>::default(),
        }
    }

    pub fn connect<'or, 'sub, T, E>(self) -> Subscription<'sub>
    where
        OE: Observable<'or, 'sub, T, E>,
        S: Observer<T, E> + Send + 'or,
    {
        self.source
            .lock()
            .unwrap()
            .take()
            .expect("Already connected")
            .subscribe(self.subject)
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'or, 'sub, T, E> for ConnectableObservable<OE, S>
where
    S: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.subject.subscribe(observer)
    }
}
