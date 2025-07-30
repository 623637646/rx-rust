use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
use crate::utils::safe_lock::{SafeLockOption, SafeLockVecDeque};
use crate::utils::types::{Mutable, NecessarySend, Shared};
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
    E: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let on_going_sub = Shared::new(Mutable::new(None));
            let observer = ConcatAllObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                pending_observables: Shared::new(Mutable::new(VecDeque::new())),
                on_going_sub: on_going_sub.clone(),
                completed: Shared::new(AtomicBool::new(false)),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + SharedDisposal::new(on_going_sub)
        })
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
struct ConcatAllObserver<'sub, T, OR, OE1> {
    observer: Shared<Mutable<Option<OR>>>,
    pending_observables: Shared<Mutable<VecDeque<OE1>>>,
    on_going_sub: Shared<Mutable<Option<Subscription<'sub>>>>,
    completed: Shared<AtomicBool>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, OR, OE1> ConcatAllObserver<'sub, T, OR, OE1> {
    fn subscribe_next<E>(&self)
    where
        T: 'or,
        E: 'or,
        OR: Observer<T, E> + NecessarySend + 'or,
        OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
        'sub: 'or,
    {
        if let Some(observable) = self.pending_observables.safe_lock_pop_front() {
            let this = self.clone();
            let terminated = Shared::new(AtomicBool::new(false));
            let terminated_cloned = terminated.clone();
            let observer = ConcatAllInnerObserver {
                observer: this.observer.clone(),
                termination_callback: move |termination| {
                    terminated_cloned.store(true, Ordering::SeqCst);
                    match termination {
                        Termination::Completed => this.subscribe_next(),
                        Termination::Error(_) => {
                            this.on_termination(termination);
                        }
                    }
                },
            };
            let sub = observable.subscribe(observer);
            if !terminated.load(Ordering::SeqCst) {
                self.on_going_sub.safe_lock_replace(sub);
            }
        } else if self.completed.load(Ordering::SeqCst) {
            if let Some(observer) = self.observer.safe_lock_take() {
                observer.on_termination(Termination::Completed);
            }
        } else {
            self.on_going_sub.safe_lock_take();
        }
    }
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for ConcatAllObserver<'sub, T, OR, OE1>
where
    T: 'or,
    E: 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E> + NecessarySend + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        self.pending_observables.safe_lock_push_back(value);
        if self.on_going_sub.safe_lock_is_none() {
            self.subscribe_next();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.completed.store(true, Ordering::SeqCst);
                if self.on_going_sub.safe_lock_is_none()
                    && self.pending_observables.safe_lock_is_empty()
                {
                    if let Some(observer) = self.observer.safe_lock_take() {
                        observer.on_termination(Termination::Completed);
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
