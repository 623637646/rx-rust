//! The [`GroupBy`] operator, behind
//! [`ObservableExt::group_by`](crate::observable::ObservableExt::group_by).

use crate::observer::boxed_observer::ObserverMode;
use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
    utils::MarkerType,
};
use educe::Educe;
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::Hash,
    marker::PhantomData,
};

/// Divides an Observable into a set of Observables, one per key, each of which emits the items of
/// the original Observable that map to its key.
/// See <https://reactivex.io/documentation/operators/groupby.html>
///
/// A group is emitted before its first item is delivered, so each group can be subscribed to
/// before its items arrive. Items emitted while a group has no subscriber are buffered and
/// replayed to a later subscriber; once the group is terminated, a late subscriber observes the
/// buffered items followed by the termination.
///
/// A group ends when the source terminates, when its subscription is disposed, or when the group
/// Observable is dropped without being subscribed to. Later items mapping to the key of an ended
/// group are discarded, just like the items of a window whose subscriber is gone.
///
/// Each group is a single-consumer pipe: it can be subscribed to once, and it is serialized on its
/// own rather than together with the outer Observable, so the items of a group keep their order
/// among themselves, but they are not ordered against the emission of another group. Disposing the
/// outer subscription ends the open groups without a termination: what a group had buffered is
/// still delivered to its subscriber, even a later one, which is then dropped without being
/// notified.
///
/// Disposing the subscription of a single group does not necessarily release its observer where it
/// happens: the group releases it on its next item, when it ends, or when the outer subscription
/// is disposed, whichever comes first. See [the unicast subject](crate::subject::unicast_subject)
/// each group is built on.
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
pub struct GroupBy<'a, OE, F, K> {
    source: OE,
    key_selector: F,
    /// The observer of a group may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the group is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the group boxes it.
    _marker: MarkerType<(&'a (), K)>,
}

impl<OE, F, K> GroupBy<'_, OE, F, K> {
    /// Creates a [`GroupBy`] over `source`;
    /// [`ObservableExt::group_by`](crate::observable::ObservableExt::group_by) is the fluent form.
    pub fn new<T, E>(source: OE, key_selector: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, E, OE, F, K> ObservableTypes for GroupBy<'a, OE, F, K>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    E: Clone,
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    type Item = BoxedUnicastObservable<'a, T, E, OE::Mode>;
    type Error = E;
    /// The groups are emitted, and fed, from wherever the source emits.
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<'a, T, E, OE, F, K, OR> Observable<OR> for GroupBy<'a, OE, F, K>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, <OE as ObservableTypes>::Mode>, E>,
    E: Clone,
    OE: Observable<
            SourceObserver<'a, <OE as ObservableTypes>::Mode, T, E, OR, F, K>,
            Item = T,
            Error = E,
        >,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        self.source.subscribe(SourceObserver {
            observer,
            senders: HashMap::new(),
            key_selector: self.key_selector,
        })
    }
}

pub struct SourceObserver<'a, M: ObserverMode, T, E, OR, F, K> {
    observer: OR,
    senders: HashMap<K, BoxedUnicastSender<'a, T, E, M>>,
    key_selector: F,
}

impl<'a, M, T, E, OR, F, K> Observer<T, E> for SourceObserver<'a, M, T, E, OR, F, K>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) -> Flow {
        let key = (self.key_selector)(&value);
        // The consumer of one group stops that group, not the operator: the other groups, and the
        // groups still to come, have their own consumers. Only the observer of the groups
        // themselves can stop the source.
        match self.senders.entry(key) {
            // A group whose consumer is gone drops the value instead of buffering it.
            Entry::Occupied(entry) => {
                let _ = entry.into_mut().on_next(value);
                Flow::Continue
            }
            Entry::Vacant(entry) => {
                let (sender, group) = unicast_subject::new_boxed();
                let sender = entry.insert(sender);
                // The group is emitted before its first value, so it can be subscribed to before
                // that value arrives. A value sent to a group that nobody subscribed to yet waits
                // in the group itself.
                let flow = self.observer.on_next(group);
                let _ = sender.on_next(value);
                flow
            }
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The groups end before the outer Observable does.
        for (_, sender) in self.senders.drain() {
            sender.on_termination(termination.clone());
        }
        self.observer.on_termination(termination);
    }
}
