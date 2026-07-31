use crate::{
    observable::{Observable, Subscription},
    observer::{Observer, Termination},
    subject::unicast_subject::{UnicastObservable, UnicastSender, unicast_subject},
    utils::types::{MarkerType, MaybeSend},
};
use educe::Educe;
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::Hash,
    marker::PhantomData,
};

/// Divides an Observable into a set of Observables, each of which emits a different group of items from the original Observable, organized by key.
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
/// outer subscription closes the open groups, which drops their observers without notifying them
/// and discards what they had buffered.
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

impl<'or, T, E, OE, F, K> Observable<'or, UnicastObservable<'or, T, E>, E> for GroupBy<OE, F, K>
where
    T: MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    F: FnMut(&T) -> K + MaybeSend + 'or,
    K: Eq + Hash + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(
        self,
        observer: impl Observer<UnicastObservable<'or, T, E>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        // The groups own their buffered items and the source is the only upstream, so this
        // observer can own the sending ends directly: the upstream holds `&mut` to it while it
        // delivers, which is what serializes the groups against each other.
        self.source.subscribe(SourceObserver {
            observer,
            senders: HashMap::new(),
            key_selector: self.key_selector,
        })
    }
}

struct SourceObserver<'or, T, E, OR, F, K> {
    observer: OR,
    /// The sending end of every group that has been opened. An ended group keeps its entry,
    /// because the values of an ended group are discarded rather than opening a new one.
    senders: HashMap<K, UnicastSender<'or, T, E>>,
    key_selector: F,
}

impl<'or, T, E, OR, F, K> Observer<T, E> for SourceObserver<'or, T, E, OR, F, K>
where
    E: Clone,
    OR: Observer<UnicastObservable<'or, T, E>, E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) {
        let key = (self.key_selector)(&value);
        match self.senders.entry(key) {
            // A group whose consumer is gone drops the value instead of buffering it.
            Entry::Occupied(entry) => entry.into_mut().on_next(value),
            Entry::Vacant(entry) => {
                let (sender, group) = unicast_subject();
                let sender = entry.insert(sender);
                // The group is emitted before its first value, so it can be subscribed to before
                // that value arrives. A value sent to a group that nobody subscribed to yet waits
                // in the group itself.
                self.observer.on_next(group);
                sender.on_next(value);
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
