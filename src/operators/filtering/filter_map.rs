//! The [`FilterMap`] operator, behind
//! [`ObservableExt::filter_map`](crate::observable::ObservableExt::filter_map).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Maps each item through a callback and emits only the results that are `Some`.
/// See <https://reactivex.io/documentation/operators/filter.html>
///
/// It does in one step what a [`Map`](crate::operators::transforming::map::Map) to an `Option`, a
/// [`Filter`](crate::operators::filtering::filter::Filter) on `is_some` and an `unwrap` would, as
/// [`Iterator::filter_map`] does for an iterator.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{creating::from_iter::FromIter, filtering::filter_map::FilterMap},
/// };
///
/// let mut values = Vec::new();
///
/// let observable = FilterMap::new(FromIter::new(["1", "two", "3"]), |value| {
///     value.parse::<i32>().ok()
/// });
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// assert_eq!(values, vec![1, 3]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct FilterMap<T0, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T0>,
}

impl<T0, OE, F> FilterMap<T0, OE, F> {
    /// Creates a [`FilterMap`] over `source`;
    /// [`ObservableExt::filter_map`](crate::observable::ObservableExt::filter_map) is the fluent
    /// form.
    pub fn new<T>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T0>,
        F: FnMut(T0) -> Option<T>,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<T0, T, E, OE, F> ObservableTypes for FilterMap<T0, OE, F>
where
    OE: ObservableTypes<Item = T0, Error = E>,
    F: FnMut(T0) -> Option<T>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T0, T, E, OE, F, OR> Observable<OR> for FilterMap<T0, OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<FilterMapObserver<OR, F>, Item = T0, Error = E>,
    F: FnMut(T0) -> Option<T>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = FilterMapObserver {
            observer,
            callback: self.callback,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct FilterMapObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T0, T, E, OR, F> Observer<T0, E> for FilterMapObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(T0) -> Option<T>,
{
    fn on_next(&mut self, value: T0) -> Flow {
        match (self.callback)(value) {
            Some(value) => self.observer.on_next(value),
            None => Flow::Continue,
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
