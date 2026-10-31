//! The [`Sample`] operator, behind
//! [`ObservableExt::sample`](crate::observable::ObservableExt::sample).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context_two_steps},
    },
};
use educe::Educe;

/// Emits the most recently emitted item from the source observable whenever the sampler observable
/// emits an item.
/// See <https://reactivex.io/documentation/operators/sample.html>
///
/// The values of the sampler are dropped — only their arrival matters — so it can be an
/// observable of any item type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::filtering::sample::Sample,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::local();
/// let mut sampler = PublishSubject::<_, Infallible, _>::local();
///
/// let subscription = Sample::new(source.clone(), sampler.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(1);
/// let _ = sampler.on_next(());
/// let _ = source.on_next(2);
/// let _ = source.on_next(3);
/// let _ = sampler.on_next(()); // Only the latest value since the last sample.
/// source.on_termination(Termination::Completed);
///
/// drop((subscription, sampler));
/// assert_eq!(values, [1, 3]);
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
    pub fn new<E>(source: OE, sampler: OE1) -> Self
    where
        OE: ObservableTypes<Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self { source, sampler }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, T, E, Model<T>, ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for Sample<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Sample<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SampleObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            SamplerObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model { last_value: None };
        subscribe_with_context_two_steps(
            observer,
            model,
            |context| Some(self.sampler.subscribe(SamplerObserver(context.clone()))),
            |context| Some(self.source.subscribe(SampleObserver(context.clone()))),
        )
        .map_inner_into()
    }
}

struct Model<T> {
    last_value: Option<T>,
}

#[doc(hidden)]
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

#[doc(hidden)]
pub struct SamplerObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<T>, D>,
);

impl<M: ThreadMode, T, T1, E, OR, D> Observer<T1, E> for SamplerObserver<M, T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T1) -> Flow {
        let flow = self.0.update_flow(|model| {
            if let Some(last_value) = model.last_value.take() {
                UpdateOutcome::empty().with_next_event(last_value)
            } else {
                UpdateOutcome::empty().without_events()
            }
        });
        // Only the arrival of a sample matters, never its value, which is dropped here, after the
        // value it emitted and outside the lock, since dropping it can run arbitrary code.
        drop(value);
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}
