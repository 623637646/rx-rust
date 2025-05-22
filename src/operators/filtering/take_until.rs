use crate::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::{instant_lock::InstantMutLock, marker::MarkerType},
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TakeUntil<T2, OE, OE2> {
    source: OE,
    stop: OE2,
    _marker: MarkerType<T2>,
}

impl<T2, OE, OE2> TakeUntil<T2, OE, OE2> {
    pub fn new<'or, 'sub, T, E>(source: OE, stop: OE2) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T2, E>,
    {
        Self {
            source,
            stop,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, T2, E, OE, OE2> Observable<'or, 'sub, T, E> for TakeUntil<T2, OE, OE2>
where
    T: 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T2, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = Arc::new(Mutex::new(Some(observer)));
        let stop_observer = StopObserver {
            observer: observer.clone(),
            _marker: PhantomData,
        };
        let subscription_1 = self.stop.subscribe(stop_observer);
        let observer = TakeUntilObserver(observer.clone());
        let subscription_2 = self.source.subscribe(observer);
        subscription_1 + subscription_2
    }
}

impl<T2, OE, OE2> ObservableExt for TakeUntil<T2, OE, OE2> {}

struct TakeUntilObserver<OR>(Arc<Mutex<Option<OR>>>);

impl<T, E, OR> Observer<T, E> for TakeUntilObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.lock_mut(|v| {
            if let Some(observer) = v {
                observer.on_next(value);
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = self.0.lock_mut(Option::take) {
            observer.on_termination(termination);
        }
    }
}

struct StopObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    _marker: MarkerType<T>,
}

impl<T, T2, E, OR> Observer<T2, E> for StopObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: T2) {
        if let Some(observer) = self.observer.lock_mut(Option::take) {
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {}
            Termination::Error(error) => {
                if let Some(observer) = self.observer.lock_mut(Option::take) {
                    observer.on_termination(Termination::Error(error));
                }
            }
        }
    }
}
