use super::{Observable, ref_count_observable::RefCount};
use crate::{observer::Observer, subscription::Subscription, utils::instant_lock::InstantMutLock};
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
            .lock_mut(Option::take)
            .expect("Already connected")
            .subscribe(self.subject)
    }

    pub fn ref_count<'sub>(self) -> RefCount<'sub, OE, S> {
        RefCount::new(self)
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
