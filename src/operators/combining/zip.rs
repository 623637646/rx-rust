//! The [`Zip`] operator, behind [`ObservableExt::zip`](crate::observable::ObservableExt::zip).

use crate::delegate_disposal;
use crate::disposable::chain_disposal::ChainDisposal;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{self, SubscriptionContext, subscribe_with_context};
use crate::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Pairs the items of two Observables in order: the first of each, then the second of each, and so
/// on. It completes once a source has completed and every item it emitted was paired.
/// See <https://reactivex.io/documentation/operators/zip.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::zip::Zip,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Zip::new(
///     FromIter::new(vec![1, 2]),
///     FromIter::new(vec![10, 20]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![(1, 10), (2, 20)]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Zip<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Zip<OE1, OE2> {
    /// Creates a [`Zip`] over `source_1` and `source_2`;
    /// [`ObservableExt::zip`](crate::observable::ObservableExt::zip) is the fluent form.
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
    subscribe_with_context::Disposal<M, (T1, T2), E, Model<T1, T2>, ChainDisposal<D2, D1>>,
    where M: ThreadMode, D1: Disposable, D2: Disposable
);

impl<T1, T2, E, OE1, OE2> ObservableTypes for Zip<OE1, OE2>
where
    OE1: ObservableTypes<Item = T1, Error = E>,
    OE2: ObservableTypes<Item = T2, Error = E>,
{
    type Item = (T1, T2);
    type Error = E;
    type Mode = Joined<OE1::Mode, OE2::Mode>;
    type Disposal = Disposal<Joined<OE1::Mode, OE2::Mode>, T1, T2, E, OE1::Disposal, OE2::Disposal>;
}

impl<T1, T2, E, OE1, OE2, OR> Observable<OR> for Zip<OE1, OE2>
where
    OR: Observer<(T1, T2), E>,
    OE1: Observable<
            ZipObserver1<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T1,
                T2,
                E,
                OR,
                ChainDisposal<
                    <OE2 as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T1,
            Error = E,
        >,
    OE2: Observable<
            ZipObserver2<
                Joined<<OE1 as ObservableTypes>::Mode, <OE2 as ObservableTypes>::Mode>,
                T1,
                T2,
                E,
                OR,
                ChainDisposal<
                    <OE2 as ObservableTypes>::Disposal,
                    <OE1 as ObservableTypes>::Disposal,
                >,
            >,
            Item = T2,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            first: (VecDeque::new(), false),
            second: (VecDeque::new(), false),
        };
        subscribe_with_context(observer, model, |context| {
            let subscription_1 = self.source_1.subscribe(ZipObserver1(context.clone()));
            let subscription_2 = self.source_2.subscribe(ZipObserver2(context));
            subscription_1.preceded_by_wrapped(subscription_2)
        })
        .map_into()
    }
}

struct Model<T1, T2> {
    /// The values of the first source waiting for a partner, and whether that source completed.
    first: (VecDeque<T1>, bool),
    /// The same for the second source.
    second: (VecDeque<T2>, bool),
}

macro_rules! impl_zip_observer {
    ($name:ident, $input_t:ty, $this_field:ident, $other_field:ident, $make_pair:expr) => {
        pub struct $name<M: ThreadMode, T1, T2, E, OR, D: Disposable>(
            SubscriptionContext<M, (T1, T2), E, OR, Model<T1, T2>, D>,
        );

        impl<M: ThreadMode, T1, T2, E, OR, D> Observer<$input_t, E> for $name<M, T1, T2, E, OR, D>
        where
            OR: Observer<(T1, T2), E>,
            D: Disposable,
        {
            fn on_next(&mut self, value: $input_t) -> Flow {
                self.0.update_flow(|model| {
                    if let Some(other) = model.$other_field.0.pop_front() {
                        if model.$other_field.1 && model.$other_field.0.is_empty() {
                            UpdateOutcome::empty().with_next_and_termination_events(
                                $make_pair(value, other),
                                Termination::Completed,
                            )
                        } else {
                            UpdateOutcome::empty().with_next_event($make_pair(value, other))
                        }
                    } else {
                        model.$this_field.0.push_back(value);
                        UpdateOutcome::empty().without_events()
                    }
                })
            }

            fn on_termination(self, termination: Termination<E>) {
                match termination {
                    completion @ Termination::Completed => {
                        let _ = self.0.update(|model| {
                            model.$this_field.1 = true;
                            if model.$this_field.0.is_empty() {
                                UpdateOutcome::empty().with_termination_event(completion)
                            } else {
                                UpdateOutcome::empty().without_events()
                            }
                        });
                    }
                    error @ Termination::Error(_) => {
                        self.0.send_termination(error);
                    }
                };
            }
        }
    };
}

impl_zip_observer!(ZipObserver1, T1, first, second, |this, other| (this, other));
impl_zip_observer!(ZipObserver2, T2, second, first, |this, other| (other, this));
