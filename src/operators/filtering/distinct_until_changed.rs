//! Emits the items that differ from the one before.
//!
//! The [`DistinctUntilChanged`] operator, behind
//! [`ObservableExt::distinct_until_changed`](crate::observable::ObservableExt::distinct_until_changed),
//! [`ObservableExt::distinct_until_changed_with_key_selector`](crate::observable::ObservableExt::distinct_until_changed_with_key_selector).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
};
use educe::Educe;

/// Emits an item from the source observable only if it differs from the immediately preceding
/// item.
/// See <https://reactivex.io/documentation/operators/distinctuntilchanged.html>
///
/// Items and keys are compared with [`PartialEq`], so one that is not equal to itself, such as
/// `f64::NAN`, never counts as a repeat: every item with such a key is emitted.
///
/// # `new` or `new_with_key_selector`
///
/// [`new`](DistinctUntilChanged::new) compares the items themselves and clones one only when it
/// differs from the one before, so a run of repeats pays for its comparisons and nothing else.
/// [`new_with_key_selector`](DistinctUntilChanged::new_with_key_selector) computes a key for every
/// item, repeat or not, and compares those; the items themselves need no [`PartialEq`].
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
///
/// let observable = DistinctUntilChanged::new(FromIter::new([1, 1, 2, 2, 1, 3]));
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 2, 1, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DistinctUntilChanged<OE, F = ()> {
    source: OE,
    /// Nothing, the default `()`, where the items are compared themselves.
    key_selector: F,
}

impl<OE> DistinctUntilChanged<OE> {
    /// Creates a [`DistinctUntilChanged`] over `source` that compares the items themselves;
    /// [`ObservableExt::distinct_until_changed`](crate::observable::ObservableExt::distinct_until_changed)
    /// is the fluent form.
    pub fn new(source: OE) -> Self {
        Self {
            source,
            key_selector: (),
        }
    }
}

impl<OE, F> DistinctUntilChanged<OE, F> {
    /// Creates a [`DistinctUntilChanged`] over `source` that compares the keys `key_selector`
    /// computes;
    /// [`ObservableExt::distinct_until_changed_with_key_selector`](crate::observable::ObservableExt::distinct_until_changed_with_key_selector)
    /// is the fluent form.
    pub fn new_with_key_selector<T, K>(source: OE, key_selector: F) -> Self
    where
        OE: ObservableTypes<Item = T>,
        F: FnMut(&T) -> K,
    {
        Self {
            source,
            key_selector,
        }
    }
}

impl<T, E, OE, F> ObservableTypes for DistinctUntilChanged<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for DistinctUntilChanged<OE>
where
    T: Clone + PartialEq,
    OR: Observer<T, E>,
    OE: Observable<DistinctUntilChangedObserver<OR, T>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = DistinctUntilChangedObserver {
            observer,
            previous: None,
        };
        self.source.subscribe(observer)
    }
}

impl<T, E, OE, F, K, OR> Observable<OR> for DistinctUntilChanged<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DistinctUntilChangedByKeyObserver<OR, F, K>, Item = T, Error = E>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = DistinctUntilChangedByKeyObserver {
            observer,
            key_selector: self.key_selector,
            previous_key: None,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct DistinctUntilChangedObserver<OR, T> {
    observer: OR,
    previous: Option<T>,
}

impl<T, E, OR> Observer<T, E> for DistinctUntilChangedObserver<OR, T>
where
    T: Clone + PartialEq,
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        if self.previous.as_ref() == Some(&value) {
            return Flow::Continue(());
        }
        // Only what is emitted is kept, so a run of repeats clones nothing.
        self.previous = Some(value.clone());
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}

#[doc(hidden)]
pub struct DistinctUntilChangedByKeyObserver<OR, F, K> {
    observer: OR,
    key_selector: F,
    previous_key: Option<K>,
}

impl<T, E, OR, F, K> Observer<T, E> for DistinctUntilChangedByKeyObserver<OR, F, K>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> K,
    K: PartialEq,
{
    fn on_next(&mut self, value: T) -> Flow {
        let key = (self.key_selector)(&value);
        if self.previous_key.as_ref() == Some(&key) {
            return Flow::Continue(());
        }
        self.previous_key = Some(key);
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
