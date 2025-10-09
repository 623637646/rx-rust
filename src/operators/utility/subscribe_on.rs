use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::Observer,
    scheduler::Scheduler,
    utils::types::{Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubscribeOn<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> SubscribeOn<OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for SubscribeOn<OE, S>
where
    OE: Observable<'or, 'static, T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let sub = Shared::new(Mutable::new(Some(Subscription::default()))); // Placeholder
        let sub_cloned = sub.clone();
        let disposal = self.scheduler.schedule(
            move || {
                sub_cloned.lock_mut(|mut lock| {
                    if lock.is_some() {
                        // Only subscribe if not unsubscribed yet.
                        let sub = self.source.subscribe(observer);
                        lock.replace(sub);
                    }
                });
            },
            None,
        );
        Subscription::new_with_disposal(sub) + disposal
    }
}
