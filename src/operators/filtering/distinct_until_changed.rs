//! The [`DistinctUntilChanged`] operator, behind
//! [`ObservableExt::distinct_until_changed`](crate::observable::ObservableExt::distinct_until_changed),
//! [`ObservableExt::distinct_until_changed_with_key_selector`](crate::observable::ObservableExt::distinct_until_changed_with_key_selector).

use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits an item from the source Observable only if it differs from the immediately preceding
/// item.
/// See <https://reactivex.io/documentation/operators/distinctuntilchanged.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         filtering::distinct_until_changed::DistinctUntilChanged,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = DistinctUntilChanged::new(FromIter::new(vec![1, 1, 2, 2, 1, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2, 1, 3]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DistinctUntilChanged<OE, F> {
    source: OE,
    key_selector: F,
}

impl<OE, F> DistinctUntilChanged<OE, F> {
    /// Creates a [`DistinctUntilChanged`] over `source` that compares the keys `key_selector`
    /// computes.
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

impl<T, OE> DistinctUntilChanged<OE, fn(&T) -> T> {
    /// Creates a [`DistinctUntilChanged`] over `source`;
    /// [`ObservableExt::distinct_until_changed`](crate::observable::ObservableExt::distinct_until_changed)
    /// is the fluent form.
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

impl<T, E, OE, F, K> ObservableTypes for DistinctUntilChanged<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: Eq,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, F, K, OR> Observable<OR> for DistinctUntilChanged<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DistinctUntilChangedObserver<OR, F, K>, Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: Eq,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        let observer = DistinctUntilChangedObserver {
            observer,
            key_selector: self.key_selector,
            previous_key: None,
        };
        self.source.subscribe(observer)
    }
}

pub struct DistinctUntilChangedObserver<OR, F, K> {
    observer: OR,
    key_selector: F,
    previous_key: Option<K>,
}

impl<T, E, OR, F, K> Observer<T, E> for DistinctUntilChangedObserver<OR, F, K>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> K,
    K: Eq,
{
    fn on_next(&mut self, value: T) -> Flow {
        let key = (self.key_selector)(&value);
        if self.previous_key.as_ref() == Some(&key) {
            return Flow::Continue;
        }
        self.previous_key = Some(key);
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
