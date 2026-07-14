use crate::utils::subscribe_with_shared_model::{
    self, Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::{delegate_disposal, utils::subscribe_with_auto_dispose_on_termination};
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;

/// Combines multiple Observables into a single Observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge::Merge,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Merge::new(
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// );
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Merge<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Merge<OE1, OE2> {
    pub fn new<'or, T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, T, E>,
        OE2: Observable<'or, T, E>,
    {
        Self { source_1, source_2 }
    }
}

delegate_disposal!(
    Disposal<'or>,
    subscribe_with_auto_dispose_on_termination::Disposal<subscribe_with_shared_model::Disposal<'or>>
);

impl<'or, T, E, OE1, OE2> Observable<'or, T, E> for Merge<OE1, OE2>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    OE2: Observable<'or, T, E>,
    OE2::D: MaybeSend + 'or,
{
    type D = Disposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            let model = Model {
                one_is_completed: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                let subscription_1 = self.source_1.subscribe(MergeObserver(context.clone()));
                let subscription_2 = self.source_2.subscribe(MergeObserver(context));
                subscription_1.preceded_by_bound(subscription_2)
            })
        })
        .map_into()
    }
}

struct Model {
    one_is_completed: bool,
}

struct MergeObserver<T, E, OR>(Context<T, E, OR, Model>);

impl<T, E, OR> Observer<T, E> for MergeObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.modify_model(|model| {
                    if model.one_is_completed {
                        ModificationResult::new_send_termination(termination)
                    } else {
                        model.one_is_completed = true;
                        ModificationResult::new_without_result()
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        }
    }
}
