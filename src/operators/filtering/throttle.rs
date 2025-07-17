use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::time::Duration;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Throttle<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Throttle<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Throttle<OE, S>
where
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let disposal = Shared::new(Mutable::new(None));
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            scheduler: self.scheduler,
            disposal: disposal.clone(),
        }) + SharedDisposal::new(disposal)
    }
}

struct ThrottleObserver<OR, S> {
    observer: OR,
    time_span: Duration,
    scheduler: S,
    disposal: Shared<Mutable<Option<BoxedDisposal<'static>>>>, // Non-Null means is cooling down.
}

impl<T, E, OR, S> Observer<T, E> for ThrottleObserver<OR, S>
where
    OR: Observer<T, E>,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.disposal.lock_mut();
        if lock.is_some() {
            return;
        }
        let disposal = self.disposal.clone();
        *lock = Some(BoxedDisposal::new(self.scheduler.clone().schedule(
            move || {
                disposal.lock_mut().take().unwrap().dispose();
            },
            Some(self.time_span),
        )));
        drop(lock);
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
