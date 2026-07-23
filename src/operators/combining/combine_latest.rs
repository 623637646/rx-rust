use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, ModelUpdate, SubscriptionContext,
    subscribe_with_context_bound_subscription,
};
use crate::utils::types::MaybeSend;
use crate::{
    disposable::Disposable,
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
};
use educe::Educe;

/// Combines multiple Observables to create an Observable whose values are calculated from the latest values of each of its input Observables.
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
/// let mut subject_1 = BehaviorSubject::<'_, i32, Infallible>::new(0);
/// let mut subject_2 = BehaviorSubject::<'_, i32, Infallible>::new(10);
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
    pub fn new<'or, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, T1, E>,
        OE2: Observable<'or, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, T1, T2, E, OE1, OE2> Observable<'or, (T1, T2), E> for CombineLatest<OE1, OE2>
where
    T1: Clone + MaybeSend + 'or,
    T2: Clone + MaybeSend + 'or,
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
            latest_1: None,
            latest_2: None,
            should_completed: false,
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            let sub_1 = self.source_1.subscribe(ObserverImpl1(context.clone()));
            let sub_2 = self.source_2.subscribe(ObserverImpl2(context));
            sub_1.preceded_by_bound(sub_2)
        })
    }
}

struct Model<T1, T2> {
    latest_1: Option<T1>,
    latest_2: Option<T2>,
    should_completed: bool,
}

macro_rules! impl_observer {
    ($name:ident, $t_self:ident, $field_self:ident, $field_other:ident, $combine:expr) => {
        struct $name<T1, T2, E, OR, D: Disposable>(
            SubscriptionContext<(T1, T2), E, OR, Model<T1, T2>, D>,
        );

        impl<T1, T2, E, OR, D> Observer<$t_self, E> for $name<T1, T2, E, OR, D>
        where
            T1: Clone,
            T2: Clone,
            OR: Observer<(T1, T2), E>,
            D: Disposable,
        {
            fn on_next(&mut self, val: $t_self) {
                let _ = self.0.try_update_model(|model| {
                    if let Some(other) = &model.$field_other {
                        model.$field_self = Some(val.clone());
                        ModelUpdate::new_send_next($combine(val, other.clone()))
                    } else {
                        model.$field_self = Some(val);
                        ModelUpdate::new_without_result()
                    }
                });
            }

            fn on_termination(self, termination: Termination<E>) {
                let _ = self.0.try_update_model(|model| match termination {
                    Termination::Completed => {
                        if model.should_completed || model.$field_self.is_none() {
                            ModelUpdate::new_send_termination(termination)
                        } else {
                            model.should_completed = true;
                            ModelUpdate::new_without_result()
                        }
                    }
                    Termination::Error(_) => ModelUpdate::new_send_termination(termination),
                });
            }
        }
    };
}

impl_observer!(ObserverImpl1, T1, latest_1, latest_2, |v, o| (v, o));
impl_observer!(ObserverImpl2, T2, latest_2, latest_1, |v, o| (o, v));
