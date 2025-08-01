use super::{Observable, ref_count_observable::RefCount};
use crate::safe_lock_option;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{disposable::subscription::Subscription, observer::Observer};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConnectableObservable<OE, S> {
    source: Shared<Mutable<Option<OE>>>,
    subject: S,
}

impl<OE, S> ConnectableObservable<OE, S> {
    pub fn new(source: OE, subject: S) -> Self {
        Self {
            source: Shared::new(Mutable::new(Some(source))),
            subject,
        }
    }

    pub fn connect<'or, 'sub, T, E>(self) -> Subscription<'sub>
    where
        OE: Observable<'or, 'sub, T, E>,
        S: Observer<T, E> + NecessarySend + 'or,
    {
        safe_lock_option!(take: self.source)
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
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        self.subject.subscribe(observer)
    }
}
