use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    subject::{
        publish_subject::PublishSubject, subject_ext::SubjectExt,
        subject_observable::SubjectObservable,
    },
    utils::types::MarkerType,
};
use educe::Educe;
use std::{collections::HashMap, hash::Hash, marker::PhantomData};

/// Divides an Observable into a set of Observables, each of which emits a different group of items from the original Observable, organized by key.
/// See <https://reactivex.io/documentation/operators/groupby.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::subscription::Subscription,
///     observable::observable_ext::ObservableExt,
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
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::<Subscription>::new()));
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
    callback: F,
    _marker: MarkerType<K>,
}

impl<OE, F, K> GroupBy<OE, F, K> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> K,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, F, K>
    Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E> for GroupBy<OE, F, K>
where
    T: Clone + NecessarySend + 'or,
    E: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) -> K + NecessarySend + 'or,
    K: Eq + Hash + NecessarySend + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        let observer = GroupByObserver {
            observer,
            callback: self.callback,
            subjects: HashMap::default(),
        };
        self.source.subscribe(observer)
    }
}

struct GroupByObserver<'or, T, E, OR, F, K> {
    observer: OR,
    callback: F,
    subjects: HashMap<K, PublishSubject<'or, T, E>>,
}

impl<'or, T, E, OR, F, K> Observer<T, E> for GroupByObserver<'or, T, E, OR, F, K>
where
    T: Clone + NecessarySend,
    E: Clone + NecessarySend,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
    F: FnMut(T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) {
        let key = (self.callback)(value.clone());
        let mut subject = self
            .subjects
            .entry(key)
            .or_insert_with(|| {
                let subject = PublishSubject::new();
                self.observer.on_next(subject.clone().into_observable());
                subject
            })
            .clone();
        subject.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subjects
            .into_values()
            .for_each(|subject| subject.on_termination(termination.clone()));
        self.observer.on_termination(termination);
    }
}
