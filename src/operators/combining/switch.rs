//! The [`Switch`] operator, behind
//! [`ObservableExt::switch`](crate::observable::ObservableExt::switch).

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    operators::{creating::from_iter::FromIter, others::with_error_type::WithErrorType},
    thread_mode::ThreadMode,
    utils::{
        MarkerType,
        id_generator::{Id, IdGenerator},
        reservation_slot::ReservationSlot,
        serialized_delivery::{DeliveryStopped, UpdateOutcome},
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;
use std::marker::PhantomData;

/// Flattens an observable of observables by emitting the items of the most recently emitted inner
/// observable only.
/// See <https://reactivex.io/documentation/operators/switch.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::switch::Switch,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
///
/// let inner_1 = FromIter::new(vec![1, 2]);
/// let inner_2 = FromIter::new(vec![3, 4]);
/// let observable = Switch::new_from_iter([inner_1, inner_2]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 3, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Switch<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Switch<OE, OE1> {
    /// Creates a [`Switch`] over `source`;
    /// [`ObservableExt::switch`](crate::observable::ObservableExt::switch) is the fluent form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> Switch<WithErrorType<E, FromIter<I>>, OE1> {
    /// Creates a [`Switch`] over the observables of `into_iterator`.
    pub fn new_from_iter(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, T, E, Model<D1>, D>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for Switch<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Switch<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            SwitchObserver<
                JoinedModeOf<OE, OE1>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::Disposal,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = OE1,
            Error = E,
        >,
    OE1: Observable<
            SwitchInnerObserver<
                JoinedModeOf<OE, OE1>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::Disposal,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            slot: ReservationSlot::Idle,
            is_source_completed: false,
            inner_ids: IdGenerator::default(),
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(SwitchObserver(context))
        })
        .map_inner_into()
    }
}

struct Model<D: Disposable> {
    slot: ReservationSlot<DisposeOnDrop<D>>,
    is_source_completed: bool,
    /// The current inner subscription is always the one subscribed last, so the id it was handed
    /// is [`IdGenerator::latest`]. An inner observer whose id is no longer the latest was
    /// superseded: its subscription is disposed outside the lock, so it can still deliver an event
    /// after being replaced, which is dropped, and a value is answered [`Flow::Stop`].
    inner_ids: IdGenerator,
}

#[doc(hidden)]
pub struct SwitchObserver<M: ThreadMode, T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<ID>, SD>,
);

impl<M: ThreadMode, T, E, OR, OE1, SD> Observer<OE1, E>
    for SwitchObserver<M, T, E, OR, OE1::Disposal, SD>
where
    OR: Observer<T, E>,
    OE1: Observable<
            SwitchInnerObserver<M, T, E, OR, <OE1 as ObservableTypes>::Disposal, SD>,
            Item = T,
            Error = E,
        >,
    SD: Disposable,
{
    fn on_next(&mut self, value: OE1) -> Flow {
        let Ok(id) = self.0.update(|model| {
            UpdateOutcome::new(model.inner_ids.next_id())
                .with_drop_outside(model.slot.reserve_replacing())
        }) else {
            return Flow::Stop;
        };
        let subscription = value.subscribe(SwitchInnerObserver(self.0.clone(), id));
        // `fill` gives the subscription back when the slot was released while it was being built,
        // which means the operator already terminated.
        self.0.update_flow(|model| {
            UpdateOutcome::empty().with_drop_outside(model.slot.fill(subscription))
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|model| {
                    model.is_source_completed = true;
                    if model.slot.is_idle() {
                        UpdateOutcome::empty().with_termination_event(completion)
                    } else {
                        UpdateOutcome::empty().without_events()
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.0.send_termination(error);
            }
        }
    }
}

#[doc(hidden)]
pub struct SwitchInnerObserver<M: ThreadMode, T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<ID>, SD>,
    Id,
);

impl<M: ThreadMode, T, E, OR, ID, SD> Observer<T, E> for SwitchInnerObserver<M, T, E, OR, ID, SD>
where
    OR: Observer<T, E>,
    ID: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        let outcome = self.0.update_with_flow(|model| {
            if model.inner_ids.latest() != Some(self.1) {
                // A value of a superseded inner observable is dropped outside the lock: dropping
                // it can run arbitrary code.
                return UpdateOutcome::new(true)
                    .with_drop_outside(value)
                    .without_events();
            }
            UpdateOutcome::new(false)
                .without_drop_outside()
                .with_next_event(value)
        });
        match outcome {
            Ok((false, flow)) => flow,
            // Superseded, so nothing it sends is wanted anymore, as if downstream had stopped: the
            // disposal on its way does the same, but a source that is not done with this value
            // yet stops here already.
            Ok((true, _)) | Err(DeliveryStopped) => Flow::Stop,
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.update(|model| {
            if model.inner_ids.latest() != Some(self.1) {
                // An ignored error can re-enter this context when dropped.
                return UpdateOutcome::empty()
                    .without_events()
                    .with_drop_outside((Some(termination), None));
            }
            match termination {
                completion @ Termination::Completed => {
                    if model.is_source_completed {
                        UpdateOutcome::empty()
                            .with_termination_event(completion)
                            .with_drop_outside((None, None))
                    } else {
                        // The terminating inner is the current one, so its subscription is still
                        // held or being built: `release` itself rejects an idle slot, and does so
                        // without the in-lock assertion that would poison the lock on the way out.
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside((None, model.slot.release()))
                    }
                }
                error @ Termination::Error(_) => UpdateOutcome::empty()
                    .with_termination_event(error)
                    .with_drop_outside((None, None)),
            }
        });
    }
}
