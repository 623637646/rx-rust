//! The [`CombineLatest`] operator, behind
//! [`ObservableExt::combine_latest`](crate::observable::ObservableExt::combine_latest).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, subscribe_with_context_owning_source,
};
use crate::{
    disposable::Disposable,
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits the latest values of two Observables as a pair whenever either of them emits, once both
/// have emitted.
/// See <https://reactivex.io/documentation/operators/combinelatest.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::combining::combine_latest::CombineLatest,
///     subject::behavior_subject::BehaviorSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let mut subject_1 = BehaviorSubject::<'_, i32, Infallible, rx_rust::thread_mode::Local>::local(0);
/// let mut subject_2 = BehaviorSubject::<'_, i32, Infallible, rx_rust::thread_mode::Local>::local(10);
///
/// let subscription =
///     CombineLatest::new(subject_1.clone(), subject_2.clone()).subscribe_with_callback(
///         |value| values.push(value),
///         |termination| terminations.push(termination),
///     );
///
/// subject_1.on_next(1);
/// subject_2.on_next(11);
/// subject_1.on_termination(Termination::Completed);
/// subject_2.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(values, vec![(0, 10), (1, 10), (1, 11)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct CombineLatest<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> CombineLatest<OE1, OE2> {
    /// Creates a [`CombineLatest`] over `source_1` and `source_2`;
    /// [`ObservableExt::combine_latest`](crate::observable::ObservableExt::combine_latest) is the
    /// fluent form.
    pub fn new<T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: ObservableTypes<Item = T1, Error = E>,
        OE2: ObservableTypes<Item = T2, Error = E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<M, T1, T2, E, D1, D2>,
    subscribe_with_context::ContextDisposal<M, (T1, T2), E, Model<T1, T2>, ChainDisposal<D2, D1>>,
    where M: ThreadMode, D1: Disposable, D2: Disposable
);

impl<T1, T2, E, OE1, OE2> ObservableTypes for CombineLatest<OE1, OE2>
where
    T1: Clone,
    T2: Clone,
    OE1: ObservableTypes<Item = T1, Error = E>,
    OE2: ObservableTypes<Item = T2, Error = E>,
{
    type Item = (T1, T2);
    type Error = E;
    type Mode = Joined<OE1::Mode, OE2::Mode>;
    type D = Disposal<Joined<OE1::Mode, OE2::Mode>, T1, T2, E, OE1::D, OE2::D>;
}

impl<T1, T2, E, OE1, OE2, OR> Observable<OR> for CombineLatest<OE1, OE2>
where
    OR: Observer<(T1, T2), E>,
    T1: Clone,
    T2: Clone,
    OE1: Observable<
            ObserverImpl1<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T1,
                T2,
                E,
                OR,
                ChainDisposal<<OE2 as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = T1,
            Error = E,
        >,
    OE2: Observable<
            ObserverImpl2<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T1,
                T2,
                E,
                OR,
                ChainDisposal<<OE2 as ObservableTypes>::D, <OE1 as ObservableTypes>::D>,
            >,
            Item = T2,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model {
            latest_1: None,
            latest_2: None,
            should_completed: false,
        };
        subscribe_with_context_owning_source(observer, model, |context| {
            let sub_1 = self.source_1.subscribe(ObserverImpl1(context.clone()));
            let sub_2 = self.source_2.subscribe(ObserverImpl2(context));
            sub_1.preceded_by_bound(sub_2)
        })
        .map_into()
    }
}

struct Model<T1, T2> {
    latest_1: Option<T1>,
    latest_2: Option<T2>,
    should_completed: bool,
}

macro_rules! impl_observer {
    ($name:ident, $t_self:ident, $field_self:ident, $field_other:ident, $combine:expr) => {
        pub struct $name<M: ThreadMode, T1, T2, E, OR, D: Disposable>(
            SubscriptionContext<M, (T1, T2), E, OR, Model<T1, T2>, D>,
        );

        impl<M: ThreadMode, T1, T2, E, OR, D> Observer<$t_self, E> for $name<M, T1, T2, E, OR, D>
        where
            T1: Clone,
            T2: Clone,
            OR: Observer<(T1, T2), E>,
            D: Disposable,
        {
            fn on_next(&mut self, val: $t_self) -> Flow {
                self.0.update_flow(|model| {
                    // The latest value this one replaces is handed back, so that the `Drop` of
                    // the user's value runs outside the lock. Building the pair still clones
                    // under it: whether there is a pair to build at all is only known here.
                    if let Some(other) = &model.$field_other {
                        let pair = $combine(val.clone(), other.clone());
                        let replaced = model.$field_self.replace(val);
                        UpdateOutcome::empty()
                            .with_drop_outside(replaced)
                            .with_next_event(pair)
                    } else {
                        let replaced = model.$field_self.replace(val);
                        UpdateOutcome::empty()
                            .with_drop_outside(replaced)
                            .without_events()
                    }
                })
            }

            fn on_termination(self, termination: Termination<E>) {
                let _ = self.0.update(|model| match termination {
                    completion @ Termination::Completed => {
                        if model.should_completed || model.$field_self.is_none() {
                            UpdateOutcome::empty().with_termination_event(completion)
                        } else {
                            model.should_completed = true;
                            UpdateOutcome::empty().without_events()
                        }
                    }
                    error @ Termination::Error(_) => {
                        UpdateOutcome::empty().with_termination_event(error)
                    }
                });
            }
        }
    };
}

impl_observer!(ObserverImpl1, T1, latest_1, latest_2, |v, o| (v, o));
impl_observer!(ObserverImpl2, T2, latest_2, latest_1, |v, o| (o, v));
