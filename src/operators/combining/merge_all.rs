//! The [`MergeAll`] operator, behind
//! [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all).

use crate::disposable::Disposable;
use crate::operators::others::with_error_type::WithErrorType;
use crate::thread_mode::Joined;
use crate::thread_mode::ThreadMode;
use crate::utils::id_generator::{Id, IdGenerator};
use crate::utils::serialized_delivery::UpdateOutcome;
use crate::utils::subscribe_with_context::{
    self, SubscriptionContext, subscribe_with_context_owning_source,
};
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::MarkerType,
};
use educe::Educe;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Flattens an Observable of Observables by emitting the values of every inner Observable as they
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
    /// Creates a [`MergeAll`] over `source`;
    /// [`ObservableExt::merge_all`](crate::observable::ObservableExt::merge_all) is the fluent
    /// form.
    pub fn new<T, E>(source: OE) -> Self
    where
        OE: ObservableTypes<Item = OE1, Error = E>,
        OE1: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<E, OE1, I> MergeAll<WithErrorType<E, FromIter<I>>, OE1> {
    /// Creates a [`MergeAll`] over the observables of `into_iterator`.
    pub fn new_from_iter<T>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source: WithErrorType::new(FromIter::new(into_iterator)),
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE, OE1> ObservableTypes for MergeAll<OE, OE1>
where
    OE: ObservableTypes<Item = OE1, Error = E>,
    OE1: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = Joined<OE::Mode, OE1::Mode>;
    type D = subscribe_with_context::ContextDisposal<
        Joined<OE::Mode, OE1::Mode>,
        T,
        E,
        Model<OE1::D>,
        OE::D,
    >;
}

impl<T, E, OE, OE1, OR> Observable<OR> for MergeAll<OE, OE1>
where
    OR: Observer<T, E>,
    OE: Observable<
            MergeAllObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::D,
                <OE as ObservableTypes>::D,
            >,
            Item = OE1,
            Error = E,
        >,
    OE1: Observable<
            MergeAllInnerObserver<
                Joined<<OE as ObservableTypes>::Mode, <OE1 as ObservableTypes>::Mode>,
                T,
                E,
                OR,
                <OE1 as ObservableTypes>::D,
                <OE as ObservableTypes>::D,
            >,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let model = Model {
            subscriptions: HashMap::new(),
            keys: IdGenerator::default(),
            is_source_terminated: false,
        };
        subscribe_with_context_owning_source(observer, model, |context| {
            self.source.subscribe(MergeAllObserver(context))
        })
    }
}

pub struct Model<D: Disposable> {
    /// Keys are never reused, so a late inner observer can never remove another
    /// inner observer's subscription.
    subscriptions: HashMap<Id, Option<Subscription<D>>>,
    keys: IdGenerator,
    is_source_terminated: bool,
}

impl<D: Disposable> Model<D> {
    /// Inserts a placeholder subscription and returns its key.
    fn insert_placeholder(&mut self) -> Id {
        let key = self.keys.next_id();
        self.subscriptions.insert(key, None);
        key
    }
}

pub struct MergeAllObserver<M: ThreadMode, T, E, OR, ID: Disposable, SD: Disposable>(
    SubscriptionContext<M, T, E, OR, Model<ID>, SD>,
);

impl<M: ThreadMode, T, E, OR, OE1, SD> Observer<OE1, E>
    for MergeAllObserver<M, T, E, OR, OE1::D, SD>
where
    OR: Observer<T, E>,
    OE1: Observable<
            MergeAllInnerObserver<M, T, E, OR, <OE1 as ObservableTypes>::D, SD>,
            Item = T,
            Error = E,
        >,
    SD: Disposable,
{
    fn on_next(&mut self, value: OE1) -> Flow {
        let result = self
            .0
            .update(|model| UpdateOutcome::new(model.insert_placeholder()));
        let key = match result {
            Ok(key) => key,
            Err(_) => return Flow::Stop,
        };
        let observer = MergeAllInnerObserver {
            context: self.0.clone(),
            key,
        };
        let sub = value.subscribe(observer);

        self.0.update_flow(|model| {
            if let Some(slot) = model.subscriptions.get_mut(&key) {
                *slot = Some(sub);
                UpdateOutcome::empty().without_drop_outside()
            } else {
                // The inner observable completed while it was being subscribed, and removed its
                // own entry.
                UpdateOutcome::empty().with_drop_outside(sub)
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
