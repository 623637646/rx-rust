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
use std::{
    collections::VecDeque,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ConcatAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> ConcatAll<OE, OE1> {
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

impl<OE1, I> ConcatAll<FromIter<I>, OE1> {
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

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for ConcatAll<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'sub,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let context = Shared::new(Mutable::new(ConcatAllContext {
                pending_observables: VecDeque::new(),
                on_going_sub: None,
                completed: false,
            }));
            let observer = ConcatAllObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                context: context.clone(),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + context
        })
    }
}

struct ConcatAllContext<'sub, OE1> {
    pending_observables: VecDeque<OE1>,
    on_going_sub: Option<Subscription<'sub>>,
    completed: bool,
}

impl<'sub, OE1> Disposable for Shared<Mutable<ConcatAllContext<'sub, OE1>>> {
    fn dispose(self) {
        if let Some(sub) = self.safe_lock_mut(|e| e.on_going_sub.take()) {
            sub.dispose();
        }
    }
}

struct ConcatAllObserver<'sub, T, OR, OE1> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<ConcatAllContext<'sub, OE1>>>,
    _marker: MarkerType<T>,
}

fn subscribe_next<'or, 'sub, T, E, OR, OE1>(
    context: Shared<Mutable<ConcatAllContext<'sub, OE1>>>,
    observer: Shared<Mutable<Option<OR>>>,
    observable: Option<OE1>,
) where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    'sub: 'or,
{
    let mut lock = context.lock_mut();
    if let Some(observable) = observable {
        lock.pending_observables.push_back(observable);
        if lock.on_going_sub.is_some() {
            return;
        }
    }
    if let Some(observable) = lock.pending_observables.pop_front() {
        drop(lock);
        let terminated = Shared::new(AtomicBool::new(false));
        let terminated_cloned = terminated.clone();
        let context_cloned = context.clone();
        let observer = ConcatAllInnerObserver {
            observer: observer.clone(),
            termination_callback: move |termination| {
                terminated_cloned.store(true, Ordering::SeqCst);
                match termination {
                    Termination::Completed => subscribe_next(context_cloned, observer, None),
                    Termination::Error(_) => {
                        observer.safe_lock_on_termination_if_some(termination);
                    }
                }
            },
        };
        let sub = observable.subscribe(observer);
        if !terminated.load(Ordering::SeqCst) {
            context.safe_lock_mut(|e| e.on_going_sub = Some(sub));
        }
    } else if lock.completed {
        drop(lock);
        observer.safe_lock_on_termination_if_some(Termination::Completed);
    } else {
        lock.on_going_sub.take();
    }
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for ConcatAllObserver<'sub, T, OR, OE1>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        subscribe_next(self.context.clone(), self.observer.clone(), Some(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let mut lock = self.context.lock_mut();
                lock.completed = true;
                if lock.on_going_sub.is_none() && lock.pending_observables.is_empty() {
                    drop(lock);
                    self.observer.safe_lock_on_termination_if_some(termination);
                }
            }
            Termination::Error(_) => {
                self.observer.safe_lock_on_termination_if_some(termination);
            }
        }
    }
}

struct ConcatAllInnerObserver<OR, F> {
    observer: Shared<Mutable<Option<OR>>>,
    termination_callback: F,
}

impl<T, E, OR, F> Observer<T, E> for ConcatAllInnerObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.safe_lock_on_next_if_some(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.termination_callback)(termination);
    }
}
