use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
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
    callback: F,
    _marker: MarkerType<K>,
}

impl<OE, F, K> GroupBy<OE, F, K> {
    pub fn new<'or, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, T, E>,
        F: FnMut(T) -> K,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE, F, K> Observable<'or, SubjectObservable<PublishSubject<'or, T, E>>, E>
    for GroupBy<OE, F, K>
where
    T: Clone + MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    F: FnMut(T) -> K + MaybeSend + 'or,
    K: Eq + Hash + MaybeSend + 'or,
{
    type D = OE::D;

    fn subscribe(
        self,
        observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
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
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
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
                self.observer
                    .on_next(SubjectObservable::new(subject.clone()));
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
