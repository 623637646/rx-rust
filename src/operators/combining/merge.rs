use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    subscription::{Subscription, disposable::CallbackDisposal},
    utils::{instant_lock::InstantMutLock, marker::MarkerType},
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
pub struct Merge<OE, OE2> {
    source: OE,
    _marker: MarkerType<OE2>,
}

impl<OE, OE2> Merge<OE, OE2> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE2, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<OE2, I> Merge<FromIter<I>, OE2> {
    pub fn new_from_iter<'or, 'sub, T, E>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE2>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE2> Observable<'or, 'sub, T, E> for Merge<OE, OE2>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE2, E>,
    OE2: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let subscriptions = Arc::new(Mutex::new(Vec::new()));
        let observer = MergeObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            subscriptions: subscriptions.clone(),
            pending_termination_count: Arc::new(AtomicUsize::new(1)),
            _marker: PhantomData,
        };
        let disposal = CallbackDisposal::new(move || subscriptions.lock_mut(Vec::clear));
        self.source.subscribe(observer) + disposal
    }
}

struct MergeObserver<'sub, T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    subscriptions: Arc<Mutex<Vec<Subscription<'sub>>>>,
    pending_termination_count: Arc<AtomicUsize>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE2> Observer<OE2, E> for MergeObserver<'sub, T, OR>
where
    OR: Observer<T, E> + Send + 'or,
    OE2: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: OE2) {
        let observer = MergeInnerObserver {
            observer: self.observer.clone(),
            pending_termination_count: self.pending_termination_count.clone(),
        };
        self.pending_termination_count
            .fetch_add(1, Ordering::SeqCst);
        let sub = value.subscribe(observer);
        self.subscriptions.lock_mut(|v| v.push(sub)); // TODO: self.subscriptions never reduce. 
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = self.observer.lock_mut(Option::take) {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = self.observer.lock_mut(Option::take) {
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
        self.observer.lock_mut(|v| {
            if let Some(observer) = v {
                observer.on_next(value);
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = self.observer.lock_mut(Option::take) {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = self.observer.lock_mut(Option::take) {
                    observer.on_termination(termination);
                }
            }
        }
    }
}
