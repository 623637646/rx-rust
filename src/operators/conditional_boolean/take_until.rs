use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::{marker::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TakeUntil<T1, OE, OE2> {
    source: OE,
    stop: OE2,
    _marker: MarkerType<T1>,
}

impl<T1, OE, OE2> TakeUntil<T1, OE, OE2> {
    pub fn new<'or, 'sub, T, E>(source: OE, stop: OE2) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T1, E>,
    {
        Self {
            source,
            stop,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, T1, E, OE, OE2> Observable<'or, 'sub, T, E> for TakeUntil<T1, OE, OE2>
where
    T: 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T1, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Arc::new(Mutex::new(Some(observer)));
            let stop_observer = StopObserver {
                observer: observer.clone(),
                _marker: PhantomData,
            };
            let subscription_1 = self.stop.subscribe(stop_observer);
            let observer = TakeUntilObserver(observer.clone());
            let subscription_2 = self.source.subscribe(observer);
            subscription_1 + subscription_2
        })
    }
}

struct TakeUntilObserver<OR>(Arc<Mutex<Option<OR>>>);

impl<T, E, OR> Observer<T, E> for TakeUntilObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if let Some(observer) = self.0.lock().unwrap().as_mut() {
            observer.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = { self.0.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}

struct StopObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    _marker: MarkerType<T>,
}

impl<T, T1, E, OR> Observer<T1, E> for StopObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: T1) {
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {}
            Termination::Error(error) => {
                if let Some(observer) = { self.observer.lock().unwrap().take() } {
                    observer.on_termination(Termination::Error(error));
                }
            }
        }
    }
}
