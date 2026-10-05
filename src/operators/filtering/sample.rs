//! The [`Sample`] operator, behind
//! [`ObservableExt::sample`](crate::observable::ObservableExt::sample).

use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, subscribe_with_context_owning_source,
};
use crate::{
    disposable::Disposable,
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits the most recently emitted item from the source Observable whenever the sampler Observable
/// emits an item.
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
/// let mut source: PublishSubject<'_, i32, Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
/// let mut sampler: PublishSubject<'_, (), Infallible, rx_rust::thread_mode::Local> = PublishSubject::local();
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
    /// Creates a [`Sample`] over `source`;
    /// [`ObservableExt::sample`](crate::observable::ObservableExt::sample) is the fluent form.
    pub fn new<T, E>(source: OE, sampler: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = (), Error = E>,
    {
        Self { source, sampler }
    }
}

impl<T, E, OE, OE1> ObservableTypes for Sample<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = (), Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type D = subscribe_with_context::ContextDisposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        Model<T>,
        ChainDisposal<OE::D, OE1::D>,
    >;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Sample<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SampleObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<<OE as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            SamplerObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                ChainDisposal<<OE as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = (),
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model { last_value: None };
        subscribe_with_context_owning_source(observer, model, |context| {
            let sample_observer = SampleObserver(context.clone());
            let sampler_observer = SamplerObserver(context);
            let subscription_1 = self.sampler.subscribe(sampler_observer);
            let subscription_2 = self.source.subscribe(sample_observer);
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

pub struct Model<T> {
    last_value: Option<T>,
}

pub struct SampleObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<T>, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<T, E> for SampleObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.update_flow(|model| {
            UpdateOutcome::empty().with_drop_outside(model.last_value.replace(value))
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

pub struct SamplerObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<T>, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<(), E> for SamplerObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) -> Flow {
        self.0.update_flow(|model| {
            if let Some(value) = model.last_value.take() {
                UpdateOutcome::empty().with_next_event(value)
            } else {
                UpdateOutcome::empty().without_events()
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}
