use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
    subscription::Subscription,
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTime<OE, S> {
    source: OE,
    time_pan: Duration,
    scheduler: S,
    delay: Option<Duration>,
}

impl<OE, S> BufferWithTime<OE, S> {
    pub fn new(source: OE, time_pan: Duration, scheduler: S, delay: Option<Duration>) -> Self {
        Self {
            source,
            time_pan,
            scheduler,
            delay,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, Vec<T>, E> for BufferWithTime<OE, S>
where
    T: Send + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(self, observer: impl Observer<Vec<T>, E> + Send + 'static) -> Subscription<'sub> {
        let observer = BufferWithTimeObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            values: Arc::new(Mutex::new(Vec::default())),
        };
        let observer_cloned = observer.clone();
        let disposal = self.scheduler.schedule_period(
            move |_| {
                if let Some(observer) = observer_cloned.observer.lock().unwrap().as_mut() {
                    let values = std::mem::take(&mut *observer_cloned.values.lock().unwrap());
                    observer.on_next(values);
                    false
                } else {
                    true
                }
            },
            self.time_pan,
            self.delay,
        );
        self.source.subscribe(observer) + disposal
    }
}

#[derive(Educe)]
#[educe(Clone)]
struct BufferWithTimeObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
}

impl<T, E, OR> Observer<T, E> for BufferWithTimeObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.lock().unwrap().push(value);
    }

    fn on_termination(self, termination: Termination<E>) {
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
