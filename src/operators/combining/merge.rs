use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    subscription::{Subscription, disposable::Disposable},
    utils::{marker::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Merge<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Merge<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<OE1, I> Merge<FromIter<I>, OE1> {
    pub fn new_from_iter<'or, 'sub, T, E>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for Merge<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let subscriptions = Arc::new(Mutex::new(Vec::new()));
            let observer = MergeObserver {
                observer: Arc::new(Mutex::new(Some(observer))),
                subscriptions: subscriptions.clone(),
                pending_termination_count: Arc::new(AtomicUsize::new(1)),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + MergeDisposal { subscriptions }
        })
    }
}

struct MergeObserver<'sub, T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    subscriptions: Arc<Mutex<Vec<Subscription<'sub>>>>,
    pending_termination_count: Arc<AtomicUsize>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for MergeObserver<'sub, T, OR>
where
    OR: Observer<T, E> + Send + 'or,
    OE1: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: OE1) {
        let observer = MergeInnerObserver {
            observer: self.observer.clone(),
            pending_termination_count: self.pending_termination_count.clone(),
        };
        self.pending_termination_count
            .fetch_add(1, Ordering::SeqCst);
        let sub = value.subscribe(observer);
        self.subscriptions.lock().unwrap().push(sub); // TODO: self.subscriptions never reduce.
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = { self.observer.lock().unwrap().take() } {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = { self.observer.lock().unwrap().take() } {
                    observer.on_termination(termination);
                }
            }
        }
    }
}

struct MergeInnerObserver<OR> {
    observer: Arc<Mutex<Option<OR>>>,
    pending_termination_count: Arc<AtomicUsize>,
}

impl<T, E, OR> Observer<T, E> for MergeInnerObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = { self.observer.lock().unwrap().take() } {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = { self.observer.lock().unwrap().take() } {
                    observer.on_termination(termination);
                }
            }
        }
    }
}

struct MergeDisposal<'sub> {
    subscriptions: Arc<Mutex<Vec<Subscription<'sub>>>>,
}

impl Disposable for MergeDisposal<'_> {
    fn dispose(self) {
        self.subscriptions.lock().unwrap().clear();
    }
}
