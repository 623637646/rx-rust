use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscription::{Subscription, disposable::CallbackDisposal},
};
use educe::Educe;
use std::{
    sync::{Arc, Mutex},
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
        let source_observer = Arc::new(Mutex::new(Some(observer)));
        let delay_observer = DelayObserver {
            source_observer: source_observer.clone(),
            delay: self.delay,
            scheduler: self.scheduler,
        };
        let disposal = CallbackDisposal::new(move || {
            source_observer.lock().unwrap().take();
        });
        let subscription = self.source.subscribe(delay_observer);
        subscription + disposal
    }
}

pub struct DelayObserver<OR, S> {
    source_observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: S,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<OR, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let observer = self.source_observer.clone();
        self.scheduler.schedule(
            move || {
                if let Some(observer) = observer.lock().unwrap().as_mut() {
                    observer.on_next(value)
                }
            },
            Some(self.delay),
        );
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match &terminal {
            Terminal::Completed => {
                self.scheduler.schedule(
                    move || {
                        if let Some(observer) = self.source_observer.lock().unwrap().take() {
                            observer.on_terminal(terminal);
                        }
                    },
                    Some(self.delay),
                );
            }
            Terminal::Error(_) => {
                if let Some(observer) = self.source_observer.lock().unwrap().take() {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}
