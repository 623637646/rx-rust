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
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debounce<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Debounce<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Debounce<OE, S>
where
    T: Send + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler + Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'static) -> Subscription<'sub> {
        let disposal = Arc::new(Mutex::new(None));
        let observer = Arc::new(Mutex::new(Some(observer)));
        self.source.subscribe(DebounceObserver {
            observer: observer.clone(),
            time_span: self.time_span,
            scheduler: self.scheduler,
            current_value: Arc::new(Mutex::new(None)),
            disposal: disposal.clone(),
        }) + DebounceDisposable { observer, disposal }
    }
}

struct DebounceObserver<T, OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    time_span: Duration,
    scheduler: S,
    current_value: Arc<Mutex<Option<T>>>,
    disposal: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<T, E, OR, S> Observer<T, E> for DebounceObserver<T, OR, S>
where
    T: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        if self.observer.lock().unwrap().is_none() {
            return;
        }
        *self.current_value.lock().unwrap() = Some(value);

        let current_value = self.current_value.clone();
        let observer = self.observer.clone();
        let disposal = self.scheduler.schedule(
            move || {
                if let Some(value) = { current_value.lock().unwrap().take() } {
                    if let Some(observer) = observer.lock().unwrap().as_mut() {
                        observer.on_next(value);
                    }
                }
            },
            Some(self.time_span),
        );

        if let Some(disposal) = {
            self.disposal
                .lock()
                .unwrap()
                .replace(BoxedDisposal::new(disposal))
        } {
            disposal.dispose();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(disposal) = { self.disposal.lock().unwrap().take() } {
            disposal.dispose();
        }
        if let Some(mut observer) = { self.observer.lock().unwrap().take() } {
            match termination {
                Termination::Completed => {
                    if let Some(value) = { self.current_value.lock().unwrap().take() } {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}

struct DebounceDisposable<OR> {
    observer: Arc<Mutex<Option<OR>>>,
    disposal: Arc<Mutex<Option<BoxedDisposal<'static>>>>,
}

impl<OR> Disposable for DebounceDisposable<OR> {
    fn dispose(self) {
        if let Some(disposal) = self.disposal.lock().unwrap().take() {
            disposal.dispose();
        }
        self.observer.lock().unwrap().take();
    }
}
