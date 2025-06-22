use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::{
        Subscription,
        disposable::{BoxedDisposal, Disposable},
    },
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

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
    S: Scheduler + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'static) -> Subscription<'sub> {
        let disposal = Arc::new(Mutex::new(None));
        self.source.subscribe(ThrottleObserver {
            observer,
            time_span: self.time_span,
            scheduler: self.scheduler,
            disposal: disposal.clone(),
        }) + ThrottleDisposable { disposal }
    }
}

struct ThrottleObserver<OR, S> {
    observer: OR,
    time_span: Duration,
    scheduler: S,
    disposal: Arc<Mutex<Option<BoxedDisposal<'static>>>>, // Non-Null means is cooling down.
}

impl<T, E, OR, S> Observer<T, E> for ThrottleObserver<OR, S>
where
    OR: Observer<T, E>,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.disposal.lock().unwrap();
        if lock.is_some() {
            return;
        }
        let disposal = self.disposal.clone();
        *lock = Some(BoxedDisposal::new(self.scheduler.schedule(
            move || {
                disposal.lock().unwrap().take();
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

struct ThrottleDisposable {
    disposal: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl Disposable for ThrottleDisposable {
    fn dispose(self) {
        if let Some(disposal) = self.disposal.lock().unwrap().take() {
            disposal.dispose();
        }
    }
}
