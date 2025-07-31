use crate::utils::safe_lock::SafeLockOption;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

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
    T: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let last_value = Shared::new(Mutable::new(None));
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
    observer: Shared<Mutable<Option<OR>>>,
    last_value: Shared<Mutable<Option<T>>>,
}

impl<T, E, OR> Observer<T, E> for SampleObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.last_value.safe_lock_replace(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.safe_lock_on_termination_if_some(termination);
    }
}

struct SamplerObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    last_value: Shared<Mutable<Option<T>>>,
}

impl<T, E, OR> Observer<(), E> for SamplerObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        self.observer
            .safe_lock_on_next_with_builder(|| self.last_value.safe_lock_take());
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.safe_lock_on_termination_if_some(termination);
    }
}
