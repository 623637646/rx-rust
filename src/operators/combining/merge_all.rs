use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::{Subscription, disposable::CallbackDisposal},
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MergeAll<OE, OE2> {
    source: OE,
    _marker: PhantomData<fn(OE2) -> OE2>, // Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn(OE2) -> OE2>`
}

impl<OE, OE2> MergeAll<OE, OE2> {
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE2> Observable<'or, 'sub, T, E> for MergeAll<OE, OE2>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE2, E>,
    OE2: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let subscriptions = Arc::new(Mutex::new(Vec::new()));
        let observer = MergeAllObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            subscriptions: subscriptions.clone(),
            pending_terminals_count: Arc::new(Mutex::new(1)),
            _marker: PhantomData,
        };
        let disposal = CallbackDisposal::new(|| {
            drop(subscriptions);
        });
        self.source.subscribe(observer) + disposal
    }
}

struct MergeAllObserver<'sub, T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    subscriptions: Arc<Mutex<Vec<Subscription<'sub>>>>,
    pending_terminals_count: Arc<Mutex<usize>>,
    _marker: PhantomData<fn(T) -> T>, // Refer to `MapInfallibleToErrorObserver` for the reason of using `PhantomData<fn(T) -> T>`
}

impl<'or, 'sub, T, E, OR, OE2> Observer<OE2, E> for MergeAllObserver<'sub, T, OR>
where
    OR: Observer<T, E> + Send + 'or,
    OE2: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: OE2) {
        let observer = MergeAllInnerObserver {
            observer: self.observer.clone(),
            pending_terminals_count: self.pending_terminals_count.clone(),
        };
        *self.pending_terminals_count.lock().unwrap() += 1;
        let sub = value.subscribe(observer);
        self.subscriptions.lock().unwrap().push(sub);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match terminal {
            Terminal::Completed => {
                let mut count = self.pending_terminals_count.lock().unwrap();
                *count -= 1;
                if *count == 0 {
                    if let Some(observer) = self.observer.lock().unwrap().take() {
                        observer.on_terminal(terminal);
                    }
                }
            }
            Terminal::Error(_) => {
                if let Some(observer) = self.observer.lock().unwrap().take() {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}

struct MergeAllInnerObserver<OR> {
    observer: Arc<Mutex<Option<OR>>>,
    pending_terminals_count: Arc<Mutex<usize>>,
}

impl<T, E, OR> Observer<T, E> for MergeAllInnerObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            observer.on_next(value);
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match terminal {
            Terminal::Completed => {
                let mut count = self.pending_terminals_count.lock().unwrap();
                *count -= 1;
                if *count == 0 {
                    if let Some(observer) = self.observer.lock().unwrap().take() {
                        observer.on_terminal(terminal);
                    }
                }
            }
            Terminal::Error(_) => {
                if let Some(observer) = self.observer.lock().unwrap().take() {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}
