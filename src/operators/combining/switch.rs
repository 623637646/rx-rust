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
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Switch<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Switch<OE, OE1> {
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

impl<OE1, I> Switch<FromIter<I>, OE1> {
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

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for Switch<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let on_going_sub = Arc::new(Mutex::new(None));
            let observer = SwitchObserver {
                observer: Arc::new(Mutex::new(Some(observer))),
                on_going_sub: on_going_sub.clone(),
                completed: Arc::new(AtomicBool::new(false)),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + SwitchDisposal { on_going_sub }
        })
    }
}

struct SwitchObserver<'sub, T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    on_going_sub: Arc<Mutex<Option<Subscription<'sub>>>>,
    completed: Arc<AtomicBool>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for SwitchObserver<'sub, T, OR>
where
    OR: Observer<T, E> + Send + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        let observer = self.observer.clone();
        let completed = self.completed.clone();
        let on_going_sub = self.on_going_sub.clone();
        let terminated = Arc::new(AtomicBool::new(false));
        let terminated_cloned = terminated.clone();

        let observer = SwitchInnerObserver {
            observer: observer.clone(),
            termination_callback: move |termination| {
                terminated_cloned.store(true, Ordering::SeqCst);
                match termination {
                    Termination::Completed => {
                        if completed.load(Ordering::SeqCst) {
                            if let Some(observer) = { observer.lock().unwrap().take() } {
                                observer.on_termination(Termination::Completed);
                            }
                        } else {
                            on_going_sub.lock().unwrap().take();
                        }
                    }
                    Termination::Error(_) => {
                        if let Some(observer) = { observer.lock().unwrap().take() } {
                            observer.on_termination(termination);
                        }
                    }
                }
            },
        };
        let sub = value.subscribe(observer);
        if !terminated.load(Ordering::SeqCst) {
            if let Some(sub) = { self.on_going_sub.lock().unwrap().replace(sub) } {
                sub.dispose();
            }
        } else if let Some(sub) = { self.on_going_sub.lock().unwrap().take() } {
            sub.dispose();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.completed.store(true, Ordering::SeqCst);
                if self.on_going_sub.lock().unwrap().is_none() {
                    if let Some(observer) = { self.observer.lock().unwrap().take() } {
                        observer.on_termination(Termination::Completed);
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

struct SwitchInnerObserver<OR, F> {
    observer: Arc<Mutex<Option<OR>>>,
    termination_callback: F,
}

impl<T, E, OR, F> Observer<T, E> for SwitchInnerObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(Termination<E>),
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.termination_callback)(termination);
    }
}

struct SwitchDisposal<'sub> {
    on_going_sub: Arc<Mutex<Option<Subscription<'sub>>>>,
}

impl Disposable for SwitchDisposal<'_> {
    fn dispose(self) {
        self.on_going_sub.lock().unwrap().take();
    }
}
