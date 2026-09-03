use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, SubscriptionContext, subscribe_with_context_bound_subscription,
};
use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::types::MarkerType,
};
use educe::Educe;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Merges an Observable of Observables into a single Observable that emits all of their emissions.
/// See <https://reactivex.io/documentation/operators/merge.html> (referencing merge operator for general concept)
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    pub fn new<'or, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, OE1, E>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> MergeAll<WithErrorType<E, FromIter<I>>, OE1> {
    pub fn new_from_iter<'or, T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, T, E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, T, E> for MergeAll<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, OE1, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let model = Model {
            subscriptions: HashMap::new(),
            next_key: 0,
            is_source_terminated: false,
        };
        subscribe_with_context_bound_subscription(observer, model, |context| {
            self.source.subscribe(MergeAllObserver(context))
        })
    }
}

struct Model<D: Disposable> {
    /// Keys are never reused, so a late inner observer can never remove another
    /// inner observer's subscription.
    subscriptions: HashMap<u64, Option<Subscription<D>>>,
    next_key: u64,
    is_source_terminated: bool,
}

impl<D: Disposable> Model<D> {
    /// Inserts a placeholder subscription and returns its key.
    fn insert_placeholder(&mut self) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        self.subscriptions.insert(key, None);
        key
    }
}

struct MergeAllObserver<T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<T, E, OR, Model<ID>, SD>,
);

impl<'or, T, E, OR, OE1, SD> Observer<OE1, E> for MergeAllObserver<T, E, OR, OE1::D, SD>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OR: Observer<T, E> + MaybeSend + 'or,
    OE1: Observable<'or, T, E>,
    OE1::D: MaybeSend + 'or,
    SD: Disposable + MaybeSend + 'or,
{
    fn on_next(&mut self, value: OE1) {
        // Insert a placeholder subscription.
        let result = self
            .0
            .update_model_and_send(|model| UpdateOutcome::new(model.insert_placeholder()));
        let key = match result {
            Ok(key) => key,
            Err(_) => return,
        };
        let observer = MergeAllInnerObserver {
            context: self.0.clone(),
            key,
        };
        let sub = value.subscribe(observer);

        let _ = self.0.update_model_and_send(|model| {
            if let Some(slot) = model.subscriptions.get_mut(&key) {
                *slot = Some(sub);
                UpdateOutcome::empty().without_drop_outside()
            } else {
                // already terminated
                UpdateOutcome::empty().with_drop_outside(sub)
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update_model_and_send(|model| {
                    if model.subscriptions.is_empty() {
                        UpdateOutcome::empty().with_termination_event(completion)
                    } else {
                        model.is_source_terminated = true;
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

struct MergeAllInnerObserver<T, E, OR, ID: Disposable, SD: Disposable> {
    context: SubscriptionContext<T, E, OR, Model<ID>, SD>,
    key: u64,
}

impl<T, E, OR, ID, SD> Observer<T, E> for MergeAllInnerObserver<T, E, OR, ID, SD>
where
    OR: Observer<T, E>,
    ID: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) {
        self.context.send_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update_model_and_send(|model| {
                    let subscription = model.subscriptions.remove(&self.key);
                    if model.is_source_terminated && model.subscriptions.is_empty() {
                        UpdateOutcome::empty()
                            .with_termination_event(completion)
                            .with_drop_outside(subscription)
                    } else {
                        UpdateOutcome::empty()
                            .without_events()
                            .with_drop_outside(subscription)
                    }
                });
            }
            error @ Termination::Error(_) => {
                self.context.send_termination(error);
            }
        }
    }
}
