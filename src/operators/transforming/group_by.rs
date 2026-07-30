use crate::{
    disposable::{Disposable, DisposableExt, option_disposal::OptionDisposal},
    observable::{Observable, Subscription},
    observer::{BoxedObserverExt, Observer, Termination, boxed_observer::BoxedObserver},
    utils::{
        subscribe_with_context::{
            self, EventBatch, ModelState, ModelUpdate, SubscriptionContext,
            subscribe_with_context_bound_subscription_retain_state_on_stop,
        },
        types::{MarkerType, MaybeSend, MutableBool, MutableBoolHelper, Shared},
    },
};
use educe::Educe;
use slotmap::{DefaultKey, SecondaryMap, SlotMap};
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::Hash,
    marker::PhantomData,
};

/// Divides an Observable into a set of Observables, each of which emits a different group of items from the original Observable, organized by key.
///
/// A group is emitted before its first item is delivered, so each group can be subscribed to
/// before its items arrive. Items emitted while a group has no subscriber are buffered and
/// replayed to a later subscriber; once the group is closed, a late subscriber observes the
/// buffered items followed by the termination.
///
/// A group ends when the source terminates, when its subscription is disposed, or when the group
/// Observable is dropped without being subscribed to. Later items mapping to the key of an ended
/// group are discarded, just like the items of a window whose subscriber is gone.
/// See <https://reactivex.io/documentation/operators/groupby.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::group_by::GroupBy,
///     },
/// };
/// use std::sync::{Arc, Mutex};
///
/// let groups = Arc::new(Mutex::new(Vec::<Vec<i32>>::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::new()));
/// let groups_observer = Arc::clone(&groups);
/// let terminations_observer = Arc::clone(&terminations);
/// let inner_subscriptions_observer = Arc::clone(&inner_subscriptions);
///
/// let subscription = GroupBy::new(FromIter::new(vec![1, 2, 3, 4]), |value| value % 2)
///     .subscribe_with_callback(
///         move |group| {
///             let index = {
///                 let mut groups = groups_observer.lock().unwrap();
///                 groups.push(Vec::new());
///                 groups.len() - 1
///             };
///             let groups_for_values = Arc::clone(&groups_observer);
///             let sub = group.subscribe_with_callback(
///                 move |value| {
///                     groups_for_values.lock().unwrap()[index].push(value);
///                 },
///                 |_| {},
///             );
///             inner_subscriptions_observer.lock().unwrap().push(sub);
///         },
///         move |termination| terminations_observer
///             .lock()
///             .unwrap()
///             .push(termination),
///     );
///
/// drop(subscription);
/// inner_subscriptions
///     .lock()
///     .unwrap()
///     .drain(..)
///     .for_each(drop);
///
/// let mut grouped = groups.lock().unwrap().clone();
/// grouped.iter_mut().for_each(|values| values.sort());
/// grouped.sort();
/// assert_eq!(grouped, vec![vec![1, 3], vec![2, 4]]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct GroupBy<OE, F, K> {
    source: OE,
    key_selector: F,
    _marker: MarkerType<K>,
}

impl<OE, F, K> GroupBy<OE, F, K> {
    pub fn new<'or, T, E>(source: OE, key_selector: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE, F, K> Observable<'or, InnerObservable<'or, T, E, OE::D>, E>
    for GroupBy<OE, F, K>
where
    T: MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    F: FnMut(&T) -> K + MaybeSend + 'or,
    K: Eq + Hash + MaybeSend + 'or,
{
    type D = subscribe_with_context::BoundSubscriptionDisposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<InnerObservable<'or, T, E, OE::D>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = DelegateObserver {
            outer_observer: observer.into_boxed(),
            inner_observers: SecondaryMap::new(),
        };
        subscribe_with_context_bound_subscription_retain_state_on_stop(
            observer,
            Groups::new(),
            |context| {
                self.source.subscribe(SourceObserver {
                    context,
                    key_selector: self.key_selector,
                    group_slots: HashMap::new(),
                })
            },
            std::mem::take,
        )
    }
}

struct GroupBuffer<T, E> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
}

impl<T, E> GroupBuffer<T, E> {
    fn open_with(value: T) -> Self {
        Self {
            values: vec![value],
            termination: None,
        }
    }
}

enum Group<T, E> {
    /// The group was emitted, but no inner observer has subscribed yet.
    Pending(GroupBuffer<T, E>),
    /// The inner observer is attached, so source values can be forwarded directly.
    Subscribed,
}

/// The live groups. A group is removed once it can no longer receive values, which is also what
/// the model retains after the subscription stops, so that late subscribers still observe their
/// buffered events.
type Groups<T, E> = SlotMap<DefaultKey, Group<T, E>>;

type GroupByContext<'or, T, E, D> = SubscriptionContext<
    DelegateAction<'or, T, E, D>,
    E,
    DelegateObserver<'or, T, E, D>,
    Groups<T, E>,
    D,
    Groups<T, E>,
>;

/// Opens a group holding `value` and gives back the action emitting it downstream.
fn open_group<'or, T, E, D>(
    groups: &mut Groups<T, E>,
    context: &GroupByContext<'or, T, E, D>,
    value: T,
) -> (DefaultKey, DelegateAction<'or, T, E, D>)
where
    E: Clone,
    D: Disposable,
{
    let key = groups.insert(Group::Pending(GroupBuffer::open_with(value)));
    let action = DelegateAction::EmitGroup(InnerObservable {
        context: Some(context.clone()),
        key,
    });
    (key, action)
}

struct SourceObserver<'or, T, E, D, F, K>
where
    E: Clone,
    D: Disposable,
{
    context: GroupByContext<'or, T, E, D>,
    key_selector: F,
    /// Maps a group key to its slot. An entry may outlive its group; the next value for that key
    /// then opens a new group.
    group_slots: HashMap<K, DefaultKey>,
}

impl<'or, T, E, D, F, K> Observer<T, E> for SourceObserver<'or, T, E, D, F, K>
where
    E: Clone,
    D: Disposable,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) {
        // The key selector runs outside the context lock.
        let key = (self.key_selector)(&value);
        let context = &self.context;
        let group_slots = &mut self.group_slots;
        let _ = context.try_update_model(|groups| match group_slots.entry(key) {
            Entry::Occupied(entry) => {
                let key = *entry.get();
                match groups.get_mut(key) {
                    Some(Group::Subscribed) => ModelUpdate::empty()
                        .with_next_event(DelegateAction::ForwardValue(key, value))
                        .without_drop_outside(),
                    Some(Group::Pending(buffer)) => {
                        buffer.values.push(value);
                        ModelUpdate::empty().without_events().without_drop_outside()
                    }
                    // The group of this key has ended, so its values have nowhere to go.
                    None => ModelUpdate::empty()
                        .without_events()
                        .with_drop_outside(value),
                }
            }
            Entry::Vacant(entry) => {
                let (key, action) = open_group(groups, context, value);
                entry.insert(key);
                ModelUpdate::empty()
                    .with_next_event(action)
                    .without_drop_outside()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.context.try_update_model(|groups| {
            let mut has_subscribed_group = false;
            groups.retain(|_, group| match group {
                // Pending groups keep their buffer so that a late subscriber
                // observes the buffered values and this termination.
                Group::Pending(buffer) => {
                    buffer.termination = Some(termination.clone());
                    true
                }
                // Subscribed groups are terminated through the delegate observer.
                Group::Subscribed => {
                    has_subscribed_group = true;
                    false
                }
            });
            if has_subscribed_group {
                ModelUpdate::empty().with_next_and_termination_events(
                    DelegateAction::TerminateAllInners(termination.clone()),
                    termination,
                )
            } else {
                ModelUpdate::empty().with_termination_event(termination)
            }
        });
    }
}

pub struct InnerObservable<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    context: Option<GroupByContext<'or, T, E, D>>,
    key: DefaultKey,
}

impl<T, E, D> Drop for InnerObservable<'_, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    fn drop(&mut self) {
        let Some(context) = self.context.take() else {
            return;
        };
        let key = self.key;
        context.update_model_or_retained_state(|model| {
            let groups = match model {
                ModelState::Active(groups) | ModelState::Stopped(groups) => groups,
            };
            ModelUpdate::empty().with_drop_outside(groups.remove(key))
        });
    }
}

impl<'or, T, E, D> Observable<'or, T, E> for InnerObservable<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    type D = OptionDisposal<InnerDisposal<'or, T, E, D>>;

    fn subscribe(
        mut self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let context = self
            .context
            .take()
            .expect("inner observable context must exist");
        let key = self.key;
        let is_disposed = Shared::new(MutableBool::new(false));
        let result = context.update_model_or_retained_state(|model| {
            // The group still accepts values only while the model is active and no
            // termination has been buffered for it.
            let is_open = match &model {
                ModelState::Active(groups) => {
                    matches!(groups.get(key), Some(Group::Pending(buffer)) if buffer.termination.is_none())
                }
                ModelState::Stopped(_) => false,
            };
            let groups = match model {
                ModelState::Active(groups) | ModelState::Stopped(groups) => groups,
            };
            let group = if is_open {
                std::mem::replace(&mut groups[key], Group::Subscribed)
            } else {
                groups.remove(key).expect("group must exist")
            };
            let Group::Pending(buffer) = group else {
                unreachable!("a group observable can only be subscribed once")
            };
            if !is_open {
                return ModelUpdate::new(Some((observer, buffer))).without_events();
            }
            let attach_inner_observer =
                DelegateAction::AttachInnerObserver(key, observer.into_boxed(), is_disposed.clone());
            if buffer.values.is_empty() {
                ModelUpdate::new(None).with_next_event(attach_inner_observer)
            } else {
                let mut actions = Vec::with_capacity(buffer.values.len() + 1);
                actions.push(attach_inner_observer);
                actions.extend(
                    buffer
                        .values
                        .into_iter()
                        .map(|value| DelegateAction::ForwardValue(key, value)),
                );
                ModelUpdate::new(None).with_events(EventBatch::NextBatch(actions))
            }
        });
        if let Some((mut observer, buffer)) = result {
            for value in buffer.values {
                observer.on_next(value);
            }
            if let Some(termination) = buffer.termination {
                observer.on_termination(termination);
            }
            OptionDisposal::none().into_subscription()
        } else {
            InnerDisposal {
                context,
                key,
                is_disposed,
            }
            .into_option()
            .into_subscription()
        }
    }
}

pub struct InnerDisposal<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    context: GroupByContext<'or, T, E, D>,
    key: DefaultKey,
    is_disposed: Shared<MutableBool>,
}

impl<'or, T, E, D> Disposable for InnerDisposal<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    fn dispose(self) {
        self.is_disposed.write(true);
        let _ = self.context.try_update_model(|groups| {
            if matches!(groups.get(self.key), Some(Group::Subscribed)) {
                // The group ends here: a later value of the same key opens a new group.
                groups.remove(self.key);
                ModelUpdate::empty().with_next_event(DelegateAction::DetachInnerObserver(self.key))
            } else {
                ModelUpdate::empty().without_events()
            }
        });
    }
}

enum DelegateAction<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    ForwardValue(DefaultKey, T),
    EmitGroup(InnerObservable<'or, T, E, D>),
    AttachInnerObserver(DefaultKey, BoxedObserver<'or, T, E>, Shared<MutableBool>),
    /// Terminates every attached inner observer. The source termination ends all groups at
    /// once, so this is deliberately a single event: unlike the events of an
    /// `EventBatch::NextBatch`, the remaining groups are still terminated when one of the
    /// inner observers disposes the outer subscription.
    TerminateAllInners(Termination<E>),
    DetachInnerObserver(DefaultKey),
}

struct AttachedInnerObserver<'or, T, E> {
    observer: BoxedObserver<'or, T, E>,
    is_disposed: Shared<MutableBool>,
}

struct DelegateObserver<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    outer_observer: BoxedObserver<'or, InnerObservable<'or, T, E, D>, E>,
    inner_observers: SecondaryMap<DefaultKey, AttachedInnerObserver<'or, T, E>>,
}

impl<'or, T, E, D> Observer<DelegateAction<'or, T, E, D>, E> for DelegateObserver<'or, T, E, D>
where
    E: Clone,
    D: Disposable,
{
    fn on_next(&mut self, action: DelegateAction<'or, T, E, D>) {
        match action {
            DelegateAction::ForwardValue(key, value) => {
                let inner_observer = self
                    .inner_observers
                    .get_mut(key)
                    .expect("subscribed group must have an inner observer");
                if !inner_observer.is_disposed.read() {
                    inner_observer.observer.on_next(value);
                }
            }
            DelegateAction::EmitGroup(inner_observable) => {
                self.outer_observer.on_next(inner_observable);
            }
            DelegateAction::AttachInnerObserver(key, observer, is_disposed) => {
                let previous = self.inner_observers.insert(
                    key,
                    AttachedInnerObserver {
                        observer,
                        is_disposed,
                    },
                );
                debug_assert!(previous.is_none());
            }
            DelegateAction::TerminateAllInners(termination) => {
                for (_, inner_observer) in self.inner_observers.drain() {
                    if !inner_observer.is_disposed.read() {
                        inner_observer.observer.on_termination(termination.clone());
                    }
                }
            }
            DelegateAction::DetachInnerObserver(key) => {
                let inner_observer = self
                    .inner_observers
                    .remove(key)
                    .expect("subscribed group must have an inner observer");
                drop(inner_observer);
            }
        };
    }

    fn on_termination(self, termination: Termination<E>) {
        debug_assert!(self.inner_observers.is_empty());
        self.outer_observer.on_termination(termination);
    }
}
