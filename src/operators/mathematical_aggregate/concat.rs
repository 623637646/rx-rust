use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    subscription::{Subscription, disposable::CallbackDisposal},
    utils::{
        instant_lock::{InstantMutLock, InstantRefLock},
        marker::MarkerType,
    },
};
use educe::Educe;
use std::{
    collections::VecDeque,
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Concat<OE, OE2> {
    source: OE,
    _marker: MarkerType<OE2>,
}

impl<OE, OE2> Concat<OE, OE2> {
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

impl<OE2, I> Concat<FromIter<I>, OE2> {
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

impl<'or, 'sub, T, E, OE, OE2> Observable<'or, 'sub, T, E> for Concat<OE, OE2>
where
    T: 'or,
    E: 'or,
    OE: Observable<'or, 'sub, OE2, E>,
    OE2: Observable<'or, 'sub, T, E> + Send + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let on_going_sub = Arc::new(Mutex::new(None));
        let observer = ConcatObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            pending_observables: Arc::new(Mutex::new(VecDeque::new())),
            on_going_sub: on_going_sub.clone(),
            completed: Arc::new(AtomicBool::new(false)),
            _marker: PhantomData,
        };
        let disposal = CallbackDisposal::new(move || {
            on_going_sub.lock_mut(Option::take);
        });
        self.source.subscribe(observer) + disposal
    }
}


#[derive(Educe)]
#[educe(Debug, Clone)]
struct ConcatObserver<'sub, T, OR, OE2> {
    observer: Arc<Mutex<Option<OR>>>,
    pending_observables: Arc<Mutex<VecDeque<OE2>>>,
    on_going_sub: Arc<Mutex<Option<Subscription<'sub>>>>,
    completed: Arc<AtomicBool>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, OR, OE2> ConcatObserver<'sub, T, OR, OE2> {
    fn subscribe_next<E>(&self)
    where
        T: 'or,
        E: 'or,
        OR: Observer<T, E> + Send + 'or,
        OE2: Observable<'or, 'sub, T, E> + Send + 'or,
        'sub: 'or,
    {
        if let Some(observable) = self.pending_observables.lock_mut(VecDeque::pop_front) {
            let this = self.clone();
            let terminated = Arc::new(AtomicBool::new(false));
            let terminated_cloned = terminated.clone();
            let observer = ConcatInnerObserver {
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
                self.on_going_sub.lock_mut(|v| v.replace(sub));
            }
        } else if self.completed.load(Ordering::SeqCst) {
            if let Some(observer) = self.observer.lock_mut(Option::take) {
                observer.on_termination(Termination::Completed);
            }
        } else {
            self.on_going_sub.lock_mut(Option::take);
        }
    }
}

impl<'or, 'sub, T, E, OR, OE2> Observer<OE2, E> for ConcatObserver<'sub, T, OR, OE2>
where
    T: 'or,
    E: 'or,
    OR: Observer<T, E> + Send + 'or,
    OE2: Observable<'or, 'sub, T, E> + Send + 'or,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE2) {
        self.pending_observables.lock_mut(|v| v.push_back(value));
        if self.on_going_sub.lock_ref(Option::is_none) {
            self.subscribe_next();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.completed.store(true, Ordering::SeqCst);
                if self.on_going_sub.lock_ref(Option::is_none)
                    && self.pending_observables.lock_ref(VecDeque::is_empty)
                {
                    if let Some(observer) = self.observer.lock_mut(Option::take) {
                        observer.on_termination(Termination::Completed);
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

struct ConcatInnerObserver<OR, F> {
    observer: Arc<Mutex<Option<OR>>>,
    termination_callback: F,
}

impl<T, E, OR, F> Observer<T, E> for ConcatInnerObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) {
        self.observer.lock_mut(|v| {
            if let Some(observer) = v {
                observer.on_next(value);
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.termination_callback)(termination);
    }
}
