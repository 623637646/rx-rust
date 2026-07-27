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

impl<'or, T, E, OE1, OE2> Observable<'or, T, E> for Merge<OE1, OE2>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    OE2: Observable<'or, T, E>,
    OE2::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            one_is_completed: false,
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            let subscription_1 = self.source_1.subscribe(MergeObserver(context.clone()));
            let subscription_2 = self.source_2.subscribe(MergeObserver(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

struct Model {
    one_is_completed: bool,
}

struct MergeObserver<T, E, OR, D: Disposable>(SubscriptionContext<T, E, OR, Model, D>);

impl<T, E, OR, D> Observer<T, E> for MergeObserver<T, E, OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        self.0.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _ = self.0.try_update_model(|model| {
                    if model.one_is_completed {
                        ModelUpdate::empty().with_termination_event(termination)
                    } else {
                        model.one_is_completed = true;
                        ModelUpdate::empty().without_events()
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        }
    }
}
