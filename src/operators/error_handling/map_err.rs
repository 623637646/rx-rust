//! The [`MapErr`] operator, behind
//! [`ObservableExt::map_err`](crate::observable::ObservableExt::map_err).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;

/// Transforms an Observable's error while leaving its items unchanged.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::throw::Throw,
/// };
///
/// let mut terminations = Vec::new();
///
/// Throw::new("boom")
///     .with_item_type::<i32>()
///     .map_err(|error| error.len())
///     .subscribe_with_callback(
///         |_| -> () { unreachable!() },
///         |termination| terminations.push(termination),
///     );
///
/// assert_eq!(terminations, vec![Termination::Error(4)]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct MapErr<E, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<E>,
}

impl<E, OE, F> MapErr<E, OE, F> {
    /// Creates a [`MapErr`] over `source`;
    /// [`ObservableExt::map_err`](crate::observable::ObservableExt::map_err) is the fluent form.
    pub fn new<T, E1>(source: OE, callback: F) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        F: FnOnce(E) -> E1,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<T, E, E1, OE, F> ObservableTypes for MapErr<E, OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(E) -> E1,
{
    type Item = T;
    type Error = E1;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, E1, OE, F, OR> Observable<OR> for MapErr<E, OE, F>
where
    OR: Observer<T, E1>,
    OE: Observable<MapErrObserver<OR, F>, Item = T, Error = E>,
    F: FnOnce(E) -> E1,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::Disposal> {
        self.source.subscribe(MapErrObserver {
            observer,
            callback: self.callback,
        })
    }
}

pub struct MapErrObserver<OR, F> {
    observer: OR,
    callback: F,
}

impl<T, E, E1, OR, F> Observer<T, E> for MapErrObserver<OR, F>
where
    OR: Observer<T, E1>,
    F: FnOnce(E) -> E1,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => self
                .observer
                .on_termination(Termination::Error((self.callback)(error))),
        }
    }
}
