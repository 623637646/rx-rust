use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, SubscriptionContext, subscribe_with_context_bound_subscription,
};
use crate::utils::types::MaybeSend;
use crate::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Combines the emissions of multiple Observables together via a specified function and emits single items for each combination based on the sequence of their emissions.
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
    pub fn new<'or, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, T1, E>,
        OE2: Observable<'or, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, T1, T2, E, OE1, OE2> Observable<'or, (T1, T2), E> for Zip<OE1, OE2>
where
    T1: MaybeSend + 'or,
    T2: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE1: Observable<'or, T1, E>,
    OE1::D: MaybeSend + 'or,
    OE2: Observable<'or, T2, E>,
    OE2::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let model = Model {
            first: (VecDeque::new(), false),
            second: (VecDeque::new(), false),
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            let subscription_1 = self.source_1.subscribe(ZipObserver1(context.clone()));
            let subscription_2 = self.source_2.subscribe(ZipObserver2(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

struct Model<T1, T2> {
    first: (VecDeque<T1>, bool),  // bool means completed
    second: (VecDeque<T2>, bool), // bool means completed
}

macro_rules! impl_zip_observer {
    ($name:ident, $input_t:ty, $this_field:ident, $other_field:ident, $make_pair:expr) => {
        struct $name<T1, T2, E, OR, D: Disposable>(
            SubscriptionContext<(T1, T2), E, OR, Model<T1, T2>, D>,
        );

        impl<T1, T2, E, OR, D> Observer<$input_t, E> for $name<T1, T2, E, OR, D>
        where
            OR: Observer<(T1, T2), E>,
            D: Disposable,
        {
            fn on_next(&mut self, value: $input_t) {
                let _ = self.0.update_model_and_send(|model| {
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
                });
            }

            fn on_termination(self, termination: Termination<E>) {
                match termination {
                    completion @ Termination::Completed => {
                        let _ = self.0.update_model_and_send(|model| {
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
