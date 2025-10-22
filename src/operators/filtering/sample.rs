use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use crate::{safe_lock_option, safe_lock_option_observer};
use educe::Educe;

/// Emits the most recently emitted item from the source Observable whenever the sampler Observable emits an item.
/// See <https://reactivex.io/documentation/operators/sample.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::filtering::sample::Sample,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut sampler: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = Sample::new(source.clone(), sampler.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// sampler.on_next(());
/// source.on_next(2);
/// source.on_next(3);
/// sampler.on_next(());
/// source.on_termination(Termination::Completed);
///
/// drop(subscription);
/// assert_eq!(&*values.lock().unwrap(), &[1, 3]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
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
        safe_lock_option!(replace: self.last_value, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
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
        if let Some(value) = safe_lock_option!(take: self.last_value) {
            safe_lock_option_observer!(on_next: self.observer, value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}
