use crate::delegate_disposal;
use crate::utils::subscribe_with_shared_model::{
    Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::utils::{subscribe_unsub_after_termination, subscribe_with_shared_model};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
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

delegate_disposal!(
    Disposal<'or>,
    subscribe_unsub_after_termination::Disposal<subscribe_with_shared_model::Disposal<'or>>
);

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
    type D = Disposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                latest_1: None,
                latest_2: None,
                should_completed: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                let sub_1 = self.source_1.subscribe(ObserverImpl1(context.clone()));
                let sub_2 = self.source_2.subscribe(ObserverImpl2(context));
                sub_1.preceded_by_bound(sub_2)
            })
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
        struct $name<T1, T2, E, OR>(Context<(T1, T2), E, OR, Model<T1, T2>>);

        impl<T1, T2, E, OR> Observer<$t_self, E> for $name<T1, T2, E, OR>
        where
            T1: Clone,
            T2: Clone,
            OR: Observer<(T1, T2), E>,
        {
            fn on_next(&mut self, val: $t_self) {
                let _ = self.0.modify_model(|model| {
                    if let Some(other) = &model.$field_other {
                        model.$field_self = Some(val.clone());
                        ModificationResult::new_send_next($combine(val, other.clone()))
                    } else {
                        model.$field_self = Some(val);
                        ModificationResult::new_without_result()
                    }
                });
            }

            fn on_termination(self, termination: Termination<E>) {
                let _ = self.0.modify_model(|model| match termination {
                    Termination::Completed => {
                        if model.should_completed || model.$field_self.is_none() {
                            ModificationResult::new_send_termination(termination)
                        } else {
                            model.should_completed = true;
                            ModificationResult::new_without_result()
                        }
                    }
                    Termination::Error(_) => ModificationResult::new_send_termination(termination),
                });
            }
        }
    };
}

impl_observer!(ObserverImpl1, T1, latest_1, latest_2, |v, o| (v, o));
impl_observer!(ObserverImpl2, T2, latest_2, latest_1, |v, o| (o, v));
