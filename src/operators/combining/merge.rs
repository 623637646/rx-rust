use crate::utils::subscribe_with_shared_model::{Action, Context, subscribe_with_shared_model};
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;

/// Combines multiple Observables into a single Observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
    pub fn new<'or, 'sub, T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T, E, OE1, OE2> Observable<'or, 'sub, T, E> for Merge<OE1, OE2>
where
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'sub,
    E: NecessarySend + 'sub,
    OE1: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                one_is_completed: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                let subscription_1 = self.source_1.subscribe(MergeObserver(context.clone()));
                let subscription_2 = self.source_2.subscribe(MergeObserver(context));
                subscription_1 + subscription_2
            })
        })
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
                self.0.modify_model_with_action(|model| {
                    let Some(model) = model else {
                        return Action::None;
                    };
                    if model.one_is_completed {
                        Action::SendTermination(termination)
                    } else {
                        model.one_is_completed = true;
                        Action::None
                    }
                });
            }
            Termination::Error(_) => {
                self.0.send_termination(termination);
            }
        }
    }
}
