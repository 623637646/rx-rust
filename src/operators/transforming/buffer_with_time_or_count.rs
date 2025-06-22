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
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};

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
    T: Send + 'static,
    OE: Observable<'static, 'sub, T, E>,
    S: Scheduler + Clone + Send + 'static,
{
    fn subscribe(self, observer: impl Observer<Vec<T>, E> + Send + 'static) -> Subscription<'sub> {
        let timer = Arc::new(Mutex::new(None));
        let observer = Arc::new(Mutex::new(Some(observer)));
        let buffer_observer = BufferWithTimeOrCountObserver {
            observer: observer.clone(),
            values: Arc::new(Mutex::new(Vec::default())),
            count: self.count,
            time_span: self.time_span,
            scheduler: self.scheduler,
            timer: timer.clone(),
        };
        buffer_observer.setup_emit_timer(self.delay);
        self.source.subscribe(buffer_observer) + BufferWithTimeOrCountDisposal { observer, timer }
    }
}

#[derive(Educe)]
#[educe(Clone)]
struct BufferWithTimeOrCountObserver<T, OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
    count: NonZeroUsize,
    time_span: Duration,
    scheduler: S,
    timer: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<T, OR, S> BufferWithTimeOrCountObserver<T, OR, S> {
    fn setup_emit_timer<E>(&self, delay: Option<Duration>)
    where
        T: Send + 'static,
        OR: Observer<Vec<T>, E> + Send + 'static,
        S: Scheduler + Clone + Send + 'static,
    {
        if self.observer.lock().unwrap().is_none() {
            return;
        }
        let self_cloned = self.clone();
        let disposal = self.scheduler.schedule(
            move || {
                let mut lock = self_cloned.observer.lock().unwrap();
                if let Some(observer) = &mut *lock {
                    let values = std::mem::take(&mut *self_cloned.values.lock().unwrap());
                    observer.on_next(values);
                    drop(lock);
                    self_cloned.setup_emit_timer(Some(self_cloned.time_span));
                }
            },
            delay,
        );
        let timer = self
            .timer
            .lock()
            .unwrap()
            .replace(BoxedDisposal::new(disposal));
        if let Some(timer) = timer {
            timer.dispose();
        }
    }
}

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeOrCountObserver<T, OR, S>
where
    T: Send + 'static,
    OR: Observer<Vec<T>, E> + Send + 'static,
    S: Scheduler + Clone + Send + 'static,
{
    fn on_next(&mut self, value: T) {
        if self.observer.lock().unwrap().is_none() {
            return;
        }

        let mut values_lock = self.values.lock().unwrap();
        values_lock.push(value);
        if values_lock.len() >= self.count.get() {
            let mut observer_lock = self.observer.lock().unwrap();
            if let Some(observer) = &mut *observer_lock {
                let values = std::mem::take(&mut *values_lock);
                drop(values_lock);
                observer.on_next(values);
                drop(observer_lock);
                self.setup_emit_timer(Some(self.time_span));
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(timer) = { self.timer.lock().unwrap().take() } {
            timer.dispose();
        }
        if let Some(mut observer) = { self.observer.lock().unwrap().take() } {
            match termination {
                Termination::Completed => {
                    let values = std::mem::take(&mut *self.values.lock().unwrap());
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

struct BufferWithTimeOrCountDisposal<OR> {
    observer: Arc<Mutex<Option<OR>>>,
    timer: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<OR> Disposable for BufferWithTimeOrCountDisposal<OR> {
    fn dispose(self) {
        if let Some(timer) = { self.timer.lock().unwrap().take() } {
            timer.dispose();
        }
        self.observer.lock().unwrap().take();
    }
}
