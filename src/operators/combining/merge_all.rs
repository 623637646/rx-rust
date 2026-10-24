//! The [`MergeAll`] operator, behind
//! [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all).

use crate::{
    delegate_disposal,
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    operators::{creating::from_iter::FromIter, others::with_error_type::WithErrorType},
    thread_mode::ThreadMode,
    utils::{
        MarkerType,
        id_generator::{Id, IdGenerator},
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
    },
};
use educe::Educe;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Flattens an observable of observables by emitting the values of every inner observable as they
/// arrive.
/// See <https://reactivex.io/documentation/operators/merge.html>
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
///
/// let observable = MergeAll::new_from_iter([
///     FromIter::new(vec![1, 3]),
///     FromIter::new(vec![2, 4]),
/// ]);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 3, 2, 4]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MergeAll<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> MergeAll<OE, OE1> {
    /// Creates a [`MergeAll`] over `source`;
    /// [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all) is the fluent
    /// form.
    pub fn new<E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> MergeAll<WithErrorType<E, FromIter<I>>, OE1> {
    /// Creates a [`MergeAll`] over the observables of `into_iterator`.
    pub fn new_from_iter(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: ObservableTypes<Error = E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, T, E, Model<D1>, D>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for MergeAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for MergeAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            MergeAllObserver<
                JoinedModeOf<OE, OE1>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::Disposal,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = OE1,
            Error = E,
        >,
    OE1: Observable<
            MergeAllInnerObserver<
                JoinedModeOf<OE, OE1>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::Disposal,
                <OE as ObservableTypes>::Disposal,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let model = Model {
            subscriptions: HashMap::new(),
            keys: IdGenerator::default(),
            is_source_completed: false,
        };
        subscribe_with_context(observer, model, |context| {
            self.source.subscribe(MergeAllObserver(context))
        })
        .map_inner_into()
    }
}

struct Model<D: Disposable> {
    /// Keys are never reused, so a late inner observer can never remove another
    /// inner observer's subscription.
    ///
    /// They are disposed in no particular order when the context stops, which rejects their events
    /// first: the order among them cannot be observed downstream.
    subscriptions: HashMap<Id, Option<DisposeOnDrop<D>>>,
    keys: IdGenerator,
    is_source_completed: bool,
}

impl<D: Disposable> Model<D> {
    /// Inserts a placeholder subscription and returns its key.
    fn insert_placeholder(&mut self) -> Id {
        let key = self.keys.next_id();
        self.subscriptions.insert(key, None);
        key
    }
}

#[doc(hidden)]
pub struct MergeAllObserver<M: ThreadMode, T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<ID>, SD>,
);

impl<M: ThreadMode, T, E, OR, OE1, SD> Observer<OE1, E>
    for MergeAllObserver<M, T, E, OR, OE1::Disposal, SD>
where
    OR: Observer<T, E>,
    OE1: Observable<
            MergeAllInnerObserver<M, T, E, OR, <OE1 as ObservableTypes>::Disposal, SD>,
            Item = T,
            Error = E,
        >,
    SD: Disposable,
{
    fn on_next(&mut self, value: OE1) -> Flow {
        let Ok(key) = self
            .0
            .update(|model| UpdateOutcome::new(model.insert_placeholder()))
        else {
            return Flow::Stop;
        };
        let observer = MergeAllInnerObserver {
            context: self.0.clone(),
            key,
        };
        let subscription = value.subscribe(observer);

        self.0.update_flow(|model| {
            if let Some(slot) = model.subscriptions.get_mut(&key) {
                *slot = Some(subscription);
                UpdateOutcome::empty().without_drop_outside()
            } else {
                // The inner observable completed while it was being subscribed, and removed its
                // own entry.
                UpdateOutcome::empty().with_drop_outside(subscription)
            }
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.0.update(|model| {
                    if model.subscriptions.is_empty() {
                        UpdateOutcome::empty().with_termination_event(completion)
                    } else {
                        model.is_source_completed = true;
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

#[doc(hidden)]
pub struct MergeAllInnerObserver<M: ThreadMode, T, E, OR, ID: Disposable, SD: Disposable> {
    context: SubscriptionContext<M, T, E, OR, Model<ID>, SD>,
    key: Id,
}

impl<M: ThreadMode, T, E, OR, ID, SD> Observer<T, E> for MergeAllInnerObserver<M, T, E, OR, ID, SD>
where
    OR: Observer<T, E>,
    ID: Disposable,
    SD: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.context.send_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            completion @ Termination::Completed => {
                let _ = self.context.update(|model| {
                    let subscription = model.subscriptions.remove(&self.key);
                    if model.is_source_completed && model.subscriptions.is_empty() {
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
