use crate::utils::subscribe_with_shared_model::{
    self, Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::{
    delegate_disposal,
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::{self, subscribe_with_auto_dispose_on_termination},
};
use educe::Educe;

/// Emits the most recently emitted item from the source Observable whenever the sampler Observable emits an item.
/// See <https://reactivex.io/documentation/operators/sample.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE, sampler: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, sampler }
    }
}

delegate_disposal!(
    Disposal<'or>,
    subscribe_with_auto_dispose_on_termination::Disposal<subscribe_with_shared_model::Disposal<'or>>
);

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for Sample<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = Disposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let model = Model { last_value: None };
            subscribe_with_shared_model(observer, model, |context| {
                let sample_observer = SampleObserver(context.clone());
                let sampler_observer = SamplerObserver(context);
                let subscription_1 = self.sampler.subscribe(sampler_observer);
                let subscription_2 = self.source.subscribe(sample_observer);
                subscription_1.preceded_by_bound(subscription_2)
            })
        })
        .map_into()
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
