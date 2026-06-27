use crate::disposable::subscription::Subscription;
use crate::observable::observable_ext::ObservableExt;
use crate::operators::others::map_infallible_to_error::MapInfallibleToError;
use crate::utils::subscribe_with_shared_model::{
    Action, ActionAndResult, Context, subscribe_with_shared_model,
};
use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{
        subscribe_unsub_after_termination::subscribe_unsub_after_termination, types::MarkerType,
    },
};
use educe::Educe;
use slotmap::{DefaultKey, SlotMap};
use std::marker::PhantomData;

/// Merges an Observable of Observables into a single Observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html> (referencing merge operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         combining::merge_all::MergeAll,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = MergeAll::new_from_iter([
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// ]);
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
pub struct MergeAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> MergeAll<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> MergeAll<MapInfallibleToError<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, 'sub, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator).map_infallible_to_error(),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for MergeAll<OE, OE1>
where
    'sub: 'or,
    'or: 'sub,
    T: NecessarySend + 'sub,
    E: NecessarySend + 'sub,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model {
                subscriptions: SlotMap::new(),
                terminated: false,
            };
            subscribe_with_shared_model(observer, model, |context| {
                self.source.subscribe(MergeAllObserver(context))
            })
        })
    }
}

struct Model<'sub> {
    subscriptions: SlotMap<DefaultKey, Subscription<'sub>>,
    terminated: bool,
}

struct MergeAllObserver<'sub, T, E, OR>(Context<T, E, OR, Model<'sub>>);

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for MergeAllObserver<'sub, T, E, OR>
where
    'sub: 'or,
    T: NecessarySend + 'or,
    E: NecessarySend + 'or,
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
{
    fn on_next(&mut self, value: OE1) {
        // Insert a placeholder subscription.
        let key = self
            .0
            .modify_model(|model| Some(model?.subscriptions.insert(Subscription::default())));
        let Some(key) = key else {
            return;
        };

        let observer = MergeAllInnerObserver {
            context: self.0.clone(),
            key,
        };
        let sub = value.subscribe(observer);

        self.0.modify_model(|model| {
            let Some(model) = model else {
                return;
            };
            if model.subscriptions.contains_key(key) {
                model.subscriptions[key] = sub;
            } else {
                // already terminated
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.0.modify_model_with_action(|model| {
                    let Some(model) = model else {
                        return Action::None;
                    };
                    if model.subscriptions.is_empty() {
                        Action::SendTermination(termination)
                    } else {
                        model.terminated = true;
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

struct MergeAllInnerObserver<'sub, T, E, OR> {
    context: Context<T, E, OR, Model<'sub>>,
    key: DefaultKey,
}

impl<T, E, OR> Observer<T, E> for MergeAllInnerObserver<'_, T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.context.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                let _subscription = self.context.modify_model_with_action_and_result(|model| {
                    let Some(model) = model else {
                        return ActionAndResult::default();
                    };
                    let subscription = model.subscriptions.remove(self.key);
                    if model.terminated && model.subscriptions.is_empty() {
                        ActionAndResult {
                            action: Action::SendTermination(termination),
                            result: Some(subscription), // Drop subscription outside the lock to avoid potential deadlock
                        }
                    } else {
                        ActionAndResult::default()
                    }
                });
            }
            Termination::Error(_) => {
                self.context.send_termination(termination);
            }
        }
    }
}
