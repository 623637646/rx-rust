use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::{
        Subscription,
        disposable::{BoxedDisposal, CallbackDisposal, Disposable},
    },
    utils::instant_lock::{InstantMutLock, InstantRefLock},
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeOrCount<OE, S> {
    source: OE,
    count: usize,
    time_pan: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTimeOrCount<OE, S> {
    pub fn new(
        source: OE,
        count: usize,
        time_pan: Duration,
        scheduler: S,
        delay: Option<Duration>,
    ) -> Self {
        Self {
            source,
            count,
            time_pan,
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
        let observer = BufferWithTimeObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            values: Arc::new(Mutex::new(Vec::default())),
            count: self.count,
            time_pan: self.time_pan,
            scheduler: self.scheduler,
            timer: Arc::new(Mutex::new(None)),
        };
        observer.setup_emit_timer(self.delay);
        let observer_cloned = observer.clone();
        let disposal = CallbackDisposal::new(move || {
            if let Some(timer) = observer_cloned.timer.lock_mut(Option::take) {
                timer.dispose();
            }
            observer_cloned.observer.lock_mut(Option::take);
        });
        self.source.subscribe(observer) + disposal
    }
}

impl<OE, S> ObservableExt for BufferWithTimeOrCount<OE, S> {}

#[derive(Educe)]
#[educe(Clone)]
struct BufferWithTimeObserver<T, OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
    count: usize,
    time_pan: Duration,
    scheduler: S,
    timer: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<T, OR, S> BufferWithTimeObserver<T, OR, S> {
    fn setup_emit_timer<E>(&self, delay: Option<Duration>)
    where
        T: Send + 'static,
        OR: Observer<Vec<T>, E> + Send + 'static,
        S: Scheduler + Clone + Send + 'static,
    {
        if self.observer.lock_ref(Option::is_none) {
            return;
        }
        let self_cloned = self.clone();
        let disposal = self.scheduler.schedule(
            move || {
                let has_observer = self_cloned.observer.lock_mut(|v| {
                    if let Some(observer) = v {
                        let values = self_cloned.values.lock_mut(std::mem::take);
                        observer.on_next(values);
                        true
                    } else {
                        false
                    }
                });
                if has_observer {
                    self_cloned.setup_emit_timer(Some(self_cloned.time_pan));
                }
            },
            delay,
        );
        let timer = self
            .timer
            .lock_mut(|v| v.replace(BoxedDisposal::new(disposal)));
        if let Some(timer) = timer {
            timer.dispose();
        }
    }
}

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeObserver<T, OR, S>
where
    T: Send + 'static,
    OR: Observer<Vec<T>, E> + Send + 'static,
    S: Scheduler + Clone + Send + 'static,
{
    fn on_next(&mut self, value: T) {
        let should_setup_emit_timer = self.values.lock_mut(|v| {
            v.push(value);
            if v.len() >= self.count {
                self.observer.lock_mut(|o| {
                    if let Some(observer) = o {
                        observer.on_next(std::mem::take(v));
                        true
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        });
        if should_setup_emit_timer {
            self.setup_emit_timer(Some(self.time_pan));
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(timer) = self.timer.lock_mut(Option::take) {
            timer.dispose();
        }
        if let Some(mut observer) = self.observer.lock_mut(Option::take) {
            match termination {
                Termination::Completed => {
                    let values = self.values.lock_mut(std::mem::take);
                    if !values.is_empty() {
                        observer.on_next(values);
                    }
                    observer.on_termination(Termination::Completed);
                }
                Termination::Error(error) => {
                    observer.on_termination(Termination::Error(error));
                }
            }
        }
    }
}
