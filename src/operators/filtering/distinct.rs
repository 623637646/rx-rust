//! The [`Distinct`] operator, behind
//! [`ObservableExt::distinct`](crate::observable::ObservableExt::distinct),
//! [`ObservableExt::distinct_with_key_selector`](crate::observable::ObservableExt::distinct_with_key_selector).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;
use std::{collections::HashSet, hash::Hash};

/// Emits the items of the source Observable that are distinct from every previous one.
/// See <https://reactivex.io/documentation/operators/distinct.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
    /// Creates a [`Distinct`] over `source` that compares the keys `key_selector` computes.
    pub fn new_with_key_selector<T, E, K>(source: OE, key_selector: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
        }
    }
}

impl<T, OE> Distinct<OE, fn(&T) -> T> {
    /// Creates a [`Distinct`] over `source`;
    /// [`ObservableExt::distinct`](crate::observable::ObservableExt::distinct) is the fluent form.
    pub fn new<E>(source: OE) -> Self
    where
        T: Clone,
        OE: ObservableTypes<Item = T, Error = E>,
    {
        Self {
            source,
            key_selector: |x| x.clone(),
        }
    }
}

impl<T, E, OE, F, K> ObservableTypes for Distinct<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, K, OR> Observable<OR> for Distinct<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DistinctObserver<OR, F, K>, Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: Eq + Hash,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = DistinctObserver {
            observer,
            key_selector: self.key_selector,
            emitted_keys: HashSet::new(),
        };
        self.source.subscribe(observer)
    }
}

pub struct DistinctObserver<OR, F, K> {
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
    fn on_next(&mut self, value: T) -> Flow {
        let key = (self.key_selector)(&value);
        if self.emitted_keys.insert(key) {
            self.observer.on_next(value)
        } else {
            Flow::Continue
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
