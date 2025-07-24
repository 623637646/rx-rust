use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::{SafeLockOption, SafeLockOptionObserver, SafeLockVec};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{types::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MergeAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> MergeAll<OE, OE1> {
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

impl<OE1, I> MergeAll<FromIter<I>, OE1> {
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

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for MergeAll<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let subscriptions = Shared::new(Mutable::new(Vec::new()));
            let observer = MergeAllObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                subscriptions: subscriptions.clone(),
                pending_termination_count: Shared::new(AtomicUsize::new(1)),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + MergeAllDisposal { subscriptions }
        })
    }
}

type SubscriptionsType<'sub> = Shared<Mutable<Vec<(Subscription<'sub>, Shared<AtomicBool>)>>>;

struct MergeAllObserver<'sub, T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    subscriptions: SubscriptionsType<'sub>,
    pending_termination_count: Shared<AtomicUsize>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for MergeAllObserver<'sub, T, OR>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: OE1) {
        let terminated = Shared::new(AtomicBool::new(false));
        let observer = MergeAllInnerObserver {
            observer: self.observer.clone(),
            pending_termination_count: self.pending_termination_count.clone(),
            terminated: terminated.clone(),
        };
        self.pending_termination_count
            .fetch_add(1, Ordering::SeqCst);
        let sub = value.subscribe(observer);

        let mut lock = self.subscriptions.lock_mut();
        // clean up terminated subscriptions
        lock.retain(|(_, terminated)| !terminated.load(Ordering::SeqCst));
        // add new subscription
        lock.push((sub, terminated));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = self.observer.safe_lock_take() {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = self.observer.safe_lock_take() {
                    observer.on_termination(termination);
                }
            }
        }
    }
}

struct MergeAllInnerObserver<OR> {
    observer: Shared<Mutable<Option<OR>>>,
    pending_termination_count: Shared<AtomicUsize>,
    terminated: Shared<AtomicBool>,
}

impl<T, E, OR> Observer<T, E> for MergeAllInnerObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.safe_lock_on_next_if_some(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.terminated.store(true, Ordering::SeqCst);
        match termination {
            Termination::Completed => {
                self.pending_termination_count
                    .fetch_sub(1, Ordering::SeqCst);
                if self.pending_termination_count.load(Ordering::SeqCst) == 0 {
                    if let Some(observer) = self.observer.safe_lock_take() {
                        observer.on_termination(termination);
                    }
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = self.observer.safe_lock_take() {
                    observer.on_termination(termination);
                }
            }
        }
    }
}

struct MergeAllDisposal<'sub> {
    subscriptions: SubscriptionsType<'sub>,
}

impl Disposable for MergeAllDisposal<'_> {
    fn dispose(self) {
        self.subscriptions.safe_lock_clear();
    }
}
