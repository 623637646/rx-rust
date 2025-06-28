use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::{Subscription, disposable::AutoDisposal},
};
use educe::Educe;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// An observable that delays the next value and completed events from the source observable by a duration.
/// The error will be emitted immediately.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
    /// Creates a new `Delay` observable.
    ///
    /// # Arguments
    ///
    /// * `source` - The source observable to delay.
    /// * `delay` - The duration to delay each emission.
    /// * `scheduler` - The scheduler to use for timing the delay.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Delay<OE, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'static) -> Subscription<'sub> {
        let delay_observer = DelayObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            delay: self.delay,
            scheduler: self.scheduler,
            timers: Vec::new(),
        };
        self.source.subscribe(delay_observer)
    }
}

struct DelayObserver<OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: S,
    timers: Vec<(AutoDisposal<'static>, Arc<AtomicBool>)>,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<OR, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        // clean up finished timers
        self.timers
            .retain(|(_, finished)| !finished.load(Ordering::SeqCst));

        let observer = self.observer.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let finished_cloned = finished.clone();
        let timer = self.scheduler.schedule(
            move || {
                if let Some(observer) = observer.lock().unwrap().as_mut() {
                    observer.on_next(value)
                }
                finished_cloned.store(true, Ordering::SeqCst);
            },
            Some(self.delay),
        );
        self.timers.push((timer, finished));
    }

    fn on_termination(self, termination: Termination<E>) {
        match &termination {
            Termination::Completed => {
                let timer_holder = Arc::new(Mutex::new(None));
                let timer_holder_cloned = timer_holder.clone();
                *timer_holder.lock().unwrap() = Some(self.scheduler.schedule(
                    move || {
                        if let Some(observer) = { self.observer.lock().unwrap().take() } {
                            observer.on_termination(termination);
                        }
                        drop(self.timers); // keep the timers alive
                        drop(timer_holder_cloned); // keep the holder alive
                    },
                    Some(self.delay),
                ));
            }
            Termination::Error(_) => {
                if let Some(observer) = { self.observer.lock().unwrap().take() } {
                    observer.on_termination(termination);
                }
            }
        }
    }
}
