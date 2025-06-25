use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::sync::{Arc, Mutex};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Sample<OE, OE1> {
    source: OE,
    sampler: OE1,
}

impl<OE, OE1> Sample<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, sampler: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, sampler }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for Sample<OE, OE1>
where
    T: Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Arc::new(Mutex::new(Some(observer)));
            let last_value = Arc::new(Mutex::new(None));
            let sample_observer = SampleObserver {
                observer: observer.clone(),
                last_value: last_value.clone(),
            };
            let sampler_observer = SamplerObserver {
                observer,
                last_value,
            };

            let subscription_1 = self.sampler.subscribe(sampler_observer);
            let subscription_2 = self.source.subscribe(sample_observer);
            subscription_1 + subscription_2
        })
    }
}

struct SampleObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    last_value: Arc<Mutex<Option<T>>>,
}

impl<T, E, OR> Observer<T, E> for SampleObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.last_value.lock().unwrap().replace(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}

struct SamplerObserver<T, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    last_value: Arc<Mutex<Option<T>>>,
}

impl<T, E, OR> Observer<(), E> for SamplerObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            if let Some(last_value) = { self.last_value.lock().unwrap().take() } {
                observer.on_next(last_value);
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}
