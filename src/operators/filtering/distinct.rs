use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::{collections::HashSet, hash::Hash};

/// Emits all items from the source Observable that are distinct by comparison from previous items.
/// See <https://reactivex.io/documentation/operators/distinct.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::distinct::Distinct,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Distinct::new(FromIter::new(vec![1, 1, 2, 2, 1, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Distinct<OE, F> {
    source: OE,
    key_selector: F,
}

impl<OE, F> Distinct<OE, F> {
    pub fn new_with_key_selector<'or, 'sub, T, E, K>(source: OE, key_selector: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
        }
    }
}

impl<T, OE> Distinct<OE, fn(&T) -> T> {
    pub fn new<'or, 'sub, E>(source: OE) -> Self
    where
        T: Clone,
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            key_selector: |x| x.clone(),
        }
    }
}

impl<'or, 'sub, T, E, OE, F, K> Observable<'or, 'sub, T, E> for Distinct<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> K + NecessarySendSync + 'or,
    K: Eq + Hash + NecessarySendSync + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        let observer = DistinctObserver {
            observer,
            key_selector: self.key_selector,
            emitted_keys: HashSet::new(),
        };
        self.source.subscribe(observer)
    }
}

struct DistinctObserver<OR, F, K> {
    observer: OR,
    key_selector: F,
    emitted_keys: HashSet<K>,
}

impl<T, E, OR, F, K> Observer<T, E> for DistinctObserver<OR, F, K>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) {
        let key = (self.key_selector)(&value);
        if self.emitted_keys.contains(&key) {
            return;
        }
        self.emitted_keys.insert(key);
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
