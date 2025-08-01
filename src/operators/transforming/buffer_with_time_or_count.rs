use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::{SafeLock, SafeLockOption};
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
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let context = Shared::new(Mutable::new(BufferWithTimeOrCountContext {
            values: Vec::default(),
            timer: None,
        }));
        let buffer_observer = BufferWithTimeOrCountObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            context: context.clone(),
            count: self.count,
            time_span: self.time_span,
            scheduler: self.scheduler,
        };
        buffer_observer.setup_emit_timer(self.delay);
        self.source.subscribe(buffer_observer) + context
    }
}

struct BufferWithTimeOrCountContext<T> {
    values: Vec<T>,
    timer: Option<BoxedDisposal<'static>>,
}

impl<T> Disposable for Shared<Mutable<BufferWithTimeOrCountContext<T>>> {
    fn dispose(self) {
        if let Some(timer) = self.safe_lock_mut(|e| e.timer.take()) {
            timer.dispose();
        }
    }
}

struct BufferWithTimeOrCountObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<BufferWithTimeOrCountContext<T>>>,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
}

impl<T, OR, S> BufferWithTimeOrCountObserver<T, OR, S> {
    fn setup_emit_timer<E>(&self, delay: Option<Duration>)
    where
        T: NecessarySend + 'static,
        OR: Observer<Vec<T>, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        if self.observer.safe_lock_is_none() {
            return;
        }
        let observer = self.observer.clone();
        let context = self.context.clone();
        let disposal = self.scheduler.schedule_periodically(
            move |_| {
                let values = context.safe_lock_mut(|e| std::mem::take(&mut e.values));
                !observer.safe_lock_on_next_if_some(values)
            },
            self.time_span,
            delay,
        );
        if let Some(timer) = self
            .context
            .safe_lock_mut(|e| e.timer.replace(BoxedDisposal::new(disposal)))
        {
            timer.dispose();
        }
    }
}

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeOrCountObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<Vec<T>, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.context.lock_mut();
        lock.values.push(value);
        if lock.values.len() >= self.count.get() {
            let values = std::mem::take(&mut lock.values);
            drop(lock);
            self.setup_emit_timer(Some(self.time_span));
            self.observer.safe_lock_on_next_if_some(values);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.clone().dispose();
        if let Some(mut observer) = self.observer.safe_lock_take() {
            match termination {
                Termination::Completed => {
                    let values = self
                        .context
                        .safe_lock_mut(|e| std::mem::take(&mut e.values));
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
