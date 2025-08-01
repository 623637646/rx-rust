use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::{SafeLock, SafeLockOption};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{types::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use educe::Educe;
use slotmap::{DefaultKey, SlotMap};
use std::marker::PhantomData;

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
            let context = Shared::new(Mutable::new(MergeAllContext {
                subscriptions: SlotMap::new(),
                terminated: false,
            }));
            let observer = MergeAllObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                context: context.clone(),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + context
        })
    }
}

struct MergeAllContext<'sub> {
    subscriptions: SlotMap<DefaultKey, Subscription<'sub>>,
    terminated: bool,
}

impl Disposable for Shared<Mutable<MergeAllContext<'_>>> {
    fn dispose(self) {
        self.safe_lock_mut(|e| e.subscriptions.clear());
    }
}

struct MergeAllObserver<'sub, T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<MergeAllContext<'sub>>>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for MergeAllObserver<'sub, T, OR>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        // Insert a placeholder subscription.
        let key = self
            .context
            .safe_lock_mut(|e| e.subscriptions.insert(Subscription::default()));

        let observer = MergeAllInnerObserver {
            observer: self.observer.clone(),
            context: self.context.clone(),
            key,
        };
        let sub = value.subscribe(observer);

        let mut lock = self.context.lock_mut();
        if lock.subscriptions.contains_key(key) {
            lock.subscriptions[key] = sub;
        } else {
            // already terminated
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let mut lock = self.context.lock_mut();
                if lock.subscriptions.is_empty() {
                    drop(lock);
                    self.observer.safe_lock_on_termination_if_some(termination);
                } else {
                    lock.terminated = true;
                }
            }
            Termination::Error(_) => {
                self.observer.safe_lock_on_termination_if_some(termination);
            }
        }
    }
}

struct MergeAllInnerObserver<'sub, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<MergeAllContext<'sub>>>,
    key: DefaultKey,
}

impl<T, E, OR> Observer<T, E> for MergeAllInnerObserver<'_, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.safe_lock_on_next_if_some(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut lock = self.context.lock_mut();
        lock.subscriptions.remove(self.key);
        match termination {
            Termination::Completed => {
                if lock.terminated && lock.subscriptions.is_empty() {
                    drop(lock);
                    self.observer.safe_lock_on_termination_if_some(termination);
                }
            }
            Termination::Error(_) => {
                drop(lock);
                self.observer.safe_lock_on_termination_if_some(termination);
            }
        }
    }
}
