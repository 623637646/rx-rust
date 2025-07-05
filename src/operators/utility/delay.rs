use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::{
        Subscription,
        disposable::{AutoDisposal, SharedDisposal},
    },
};
use educe::Educe;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
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
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'static) -> Subscription<'sub> {
        let timer = Arc::new(Mutex::new(None));
        let delay_observer = DelayObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            delay: self.delay,
            scheduler: self.scheduler,
            values: Arc::new(Mutex::new(VecDeque::new())),
            timer: timer.clone(),
        };
        self.source.subscribe(delay_observer) + SharedDisposal::new(timer)
    }
}

type DelayObserverValues<T> = Arc<Mutex<VecDeque<(Instant, Option<T>)>>>; // None means completed

struct DelayObserver<T, OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: S,
    values: DelayObserverValues<T>,
    timer: Arc<Mutex<Option<AutoDisposal<'static>>>>,
}

impl<T, OR, S> DelayObserver<T, OR, S> {
    fn setup_emit_timer_if_needed<E>(&mut self)
    where
        T: Send + 'static,
        OR: Observer<T, E> + Send + 'static,
        S: Scheduler,
    {
        if self.timer.lock().unwrap().is_some() {
            return;
        }
        let values = self.values.clone();
        let observer = self.observer.clone();
        let timer = self.timer.clone();
        *self.timer.lock().unwrap() = Some(self.scheduler.schedule_recursive(
            move |_| {
                if let Some((instant, value)) = { values.lock().unwrap().pop_front() } {
                    if let Some(value) = value {
                        //  next
                        if let Some(observer) = observer.lock().unwrap().as_mut() {
                            observer.on_next(value);
                            if let Some((next_instant, _)) = values.lock().unwrap().front() {
                                let delay = next_instant.duration_since(instant);
                                Some(delay)
                            } else {
                                timer.lock().unwrap().take().unwrap();
                                None
                            }
                        } else {
                            timer.lock().unwrap().take().unwrap();
                            None
                        }
                    } else {
                        // completed
                        if let Some(observer) = { observer.lock().unwrap().take() } {
                            observer.on_termination(Termination::Completed);
                        }
                        timer.lock().unwrap().take().unwrap();
                        None
                    }
                } else {
                    unreachable!()
                }
            },
            Some(self.delay),
        ));
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.values
            .lock()
            .unwrap()
            .push_back((Instant::now(), Some(value)));
        self.setup_emit_timer_if_needed();
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.values
                    .lock()
                    .unwrap()
                    .push_back((Instant::now(), None));
                self.setup_emit_timer_if_needed();
            }
            Termination::Error(_) => {
                if let Some(observer) = { self.observer.lock().unwrap().take() } {
                    observer.on_termination(termination);
                }
            }
        }
    }
}
