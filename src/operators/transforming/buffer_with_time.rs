use crate::utils::safe_lock::{SafeLock, SafeLockOption, SafeLockVec};
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::time::Duration;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTime<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_span,
            scheduler,
            delay,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTime<OE, S>
where
    T: NecessarySend + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let observer = BufferWithTimeObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            values: Shared::new(Mutable::new(Vec::default())),
        };
        let observer_cloned = observer.clone();
        let disposal = self.scheduler.schedule_periodically(
            move |_| {
                let values = observer_cloned.values.safe_lock_mem_take();
                !observer_cloned.observer.safe_lock_on_next_if_some(values)
            },
            self.time_span,
            self.delay,
        );
        self.source.subscribe(observer) + disposal
    }
}

#[derive(Educe)]
#[educe(Clone)]
struct BufferWithTimeObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferWithTimeObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.safe_lock_push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(mut observer) = self.observer.safe_lock_take() {
            match termination {
                Termination::Completed => {
                    let values = self.values.safe_lock_mem_take();
                    if !values.is_empty() {
                        observer.on_next(values);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
