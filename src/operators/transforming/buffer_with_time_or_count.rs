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
use std::{num::NonZeroUsize, time::Duration};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeOrCount<OE, S> {
    source: OE,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    pub fn new(
        source: OE,
        count: NonZeroUsize,
        time_span: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> Self {
        Self {
            source,
            count,
            time_span,
            scheduler,
            delay,
        }
    }
}

impl<'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTimeOrCount<OE, S>
where
    T: NecessarySend + 'static,
    OE: Observable<'static, 'sub, T, E>,
    S: Scheduler + Clone + NecessarySend + 'static,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let timer = Shared::new(Mutable::new(None));
        let buffer_observer = BufferWithTimeOrCountObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            values: Shared::new(Mutable::new(Vec::default())),
            count: self.count,
            time_span: self.time_span,
            scheduler: self.scheduler,
            timer: timer.clone(),
        };
        buffer_observer.setup_emit_timer(self.delay);
        self.source.subscribe(buffer_observer) + SharedDisposal::new(timer)
    }
}

#[derive(Educe)]
#[educe(Clone)]
struct BufferWithTimeOrCountObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    values: Shared<Mutable<Vec<T>>>,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    timer: Shared<Mutable<Option<BoxedDisposal<'static>>>>,
}

impl<T, OR, S> BufferWithTimeOrCountObserver<T, OR, S> {
    fn setup_emit_timer<E>(&self, delay: Option<Duration>)
    where
        T: NecessarySend + 'static,
        OR: Observer<Vec<T>, E> + NecessarySend + 'static,
        S: Scheduler + Clone + NecessarySend + 'static,
    {
        if self.observer.lock_ref().is_none() {
            return;
        }
        let self_cloned = self.clone();
        let disposal = self.scheduler.clone().schedule_periodically(
            move |_| {
                let mut lock = self_cloned.observer.lock_mut();
                if let Some(observer) = &mut *lock {
                    let values = std::mem::take(&mut *self_cloned.values.lock_mut());
                    observer.on_next(values);
                    drop(lock);
                    false
                } else {
                    true
                }
            },
            self.time_span,
            delay,
        );
        let timer = self.timer.lock_mut().replace(BoxedDisposal::new(disposal));
        if let Some(timer) = timer {
            timer.dispose();
        }
    }
}

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeOrCountObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<Vec<T>, E> + NecessarySend + 'static,
    S: Scheduler + Clone + NecessarySend + 'static,
{
    fn on_next(&mut self, value: T) {
        let mut observer_lock = self.observer.lock_mut();
        if let Some(observer) = &mut *observer_lock {
            let mut values_lock = self.values.lock_mut();
            values_lock.push(value);
            if values_lock.len() >= self.count.get() {
                let values = std::mem::take(&mut *values_lock);
                drop(values_lock);
                observer.on_next(values);
                drop(observer_lock);
                self.setup_emit_timer(Some(self.time_span));
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(timer) = { self.timer.lock_mut().take() } {
            timer.dispose();
        }
        if let Some(mut observer) = { self.observer.lock_mut().take() } {
            match termination {
                Termination::Completed => {
                    let values = std::mem::take(&mut *self.values.lock_mut());
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
