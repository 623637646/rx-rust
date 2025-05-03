use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscription::{
        Subscription,
        disposable::{BoxedDisposal, CallbackDisposal, Disposable},
    },
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
            if let Some(timer) = observer_cloned.timer.lock().unwrap().take() {
                timer.dispose();
            }
            observer_cloned.observer.lock().unwrap().take();
        });
        self.source.subscribe(observer) + disposal
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithTimeObserver<T, OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
    count: usize,
    time_pan: Duration,
    scheduler: S,
    #[educe(Debug(ignore))]
    timer: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<T, OR, S> BufferWithTimeObserver<T, OR, S> {
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
                let mut observer_lock = self_cloned.observer.lock().unwrap();
                if let Some(observer) = observer_lock.as_mut() {
                    observer.on_next(std::mem::take(&mut self_cloned.values.lock().unwrap()));
                    drop(observer_lock);
                    self_cloned.setup_emit_timer(Some(self_cloned.time_pan));
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

impl<T, E, OR, S> Observer<T, E> for BufferWithTimeObserver<T, OR, S>
where
    T: Send + 'static,
    OR: Observer<Vec<T>, E> + Send + 'static,
    S: Scheduler + Clone + Send + 'static,
{
    fn on_next(&mut self, value: T) {
        let mut values = self.values.lock().unwrap();
        values.push(value);
        if values.len() >= self.count {
            let mut observer_lock = self.observer.lock().unwrap();
            if let Some(observer) = observer_lock.as_mut() {
                observer.on_next(std::mem::take(&mut values));
                drop(observer_lock);
                self.setup_emit_timer(Some(self.time_pan));
            }
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        if let Some(timer) = self.timer.lock().unwrap().take() {
            timer.dispose();
        }
        if let Some(mut observer) = self.observer.lock().unwrap().take() {
            match terminal {
                Terminal::Completed => {
                    let mut values = self.values.lock().unwrap();
                    if !values.is_empty() {
                        observer.on_next(std::mem::take(&mut values));
                    }
                    observer.on_terminal(Terminal::Completed);
                }
                Terminal::Error(error) => {
                    observer.on_terminal(Terminal::Error(error));
                }
            }
        }
    }
}
