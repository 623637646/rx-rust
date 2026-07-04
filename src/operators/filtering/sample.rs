use crate::utils::subscribe_with_shared_model::{
    Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
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
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model { last_value: None };
            subscribe_with_shared_model(observer, model, |context| {
                let sample_observer = SampleObserver(context.clone());
                let sampler_observer = SamplerObserver(context);
                let subscription_1 = self.sampler.subscribe(sampler_observer);
                let subscription_2 = self.source.subscribe(sample_observer);
                subscription_1 + subscription_2
            })
        })
    }
}

struct Model<T> {
    last_value: Option<T>,
}

struct SampleObserver<T, E, OR>(Context<T, E, OR, Model<T>>);

impl<T, E, OR> Observer<T, E> for SampleObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.modify_model(|model| {
            ModificationResult::new_without_result().drop_outside(model.last_value.replace(value))
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

struct SamplerObserver<T, E, OR>(Context<T, E, OR, Model<T>>);

impl<T, E, OR> Observer<(), E> for SamplerObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        let _ = self.0.modify_model(|model| {
            if let Some(value) = model.last_value.take() {
                ModificationResult::new_without_result()
                    .send_next(value)
                    .ignore_drop_outside()
            } else {
                ModificationResult::new_without_result()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}
