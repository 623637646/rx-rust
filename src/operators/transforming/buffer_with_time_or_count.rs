use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{MutGuard, Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{safe_lock, safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
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
        buffer_observer.setup_emit_timer(None, self.delay);
        self.source.subscribe(buffer_observer) + context
    }
}

struct BufferWithTimeOrCountContext<T> {
    values: Vec<T>,
    timer: Option<BoxedDisposal<'static>>,
}

impl<T> Disposable for Shared<Mutable<BufferWithTimeOrCountContext<T>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
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
    fn setup_emit_timer<E>(
        &self,
        mut lock: Option<MutGuard<'_, BufferWithTimeOrCountContext<T>>>,
        delay: Option<Duration>,
    ) where
        T: NecessarySend + 'static,
        OR: Observer<Vec<T>, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        let observer = self.observer.clone();
        let context = self.context.clone();
        let disposal = self.scheduler.schedule_periodically(
            move |_| {
                let values = safe_lock!(mem_take: context, values);
                !safe_lock_option_observer!(on_next: observer, values)
            },
            self.time_span,
            delay,
        );
        let old_timer = if let Some(lock) = lock.as_mut() {
            lock.timer.replace(BoxedDisposal::new(disposal))
        } else {
            safe_lock_option!(replace: self.context, timer, BoxedDisposal::new(disposal))
        };
        drop(lock);
        if let Some(timer) = old_timer {
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
        self.context.lock_mut(|mut lock| {
            lock.values.push(value);
            if lock.values.len() >= self.count.get() {
                let values = std::mem::take(&mut lock.values);
                self.setup_emit_timer(Some(lock), Some(self.time_span));
                safe_lock_option_observer!(on_next: self.observer, values);
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.clone().dispose();
        if let Some(mut observer) = safe_lock_option!(take: self.observer) {
            match termination {
                Termination::Completed => {
                    let values = safe_lock!(mem_take: self.context, values);
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
