//! The [`WithItemType`] operator, behind
//! [`ObservableExt::with_item_type`](crate::observable::ObservableExt::with_item_type).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Gives an observable whose item type is `Infallible` a concrete item type.
///
/// A source that never emits an item — `Empty`, `Never`, `Throw` — says so with
/// `Item = Infallible`. The operators that join several observables (`merge`, `concat`, `amb`,
/// `switch_if_empty`, the inner observables of `catch`, `retry` and the `*_map` operators) require
/// their items to be the same type, though, and Rust does not turn `Infallible` into another type
/// on its own: this operator is that step, done explicitly where the types meet. It does nothing
/// at run time.
///
/// It is the same as `map(|item| match item {})`, without the closure. The item type is usually
/// inferred from the observable it is joined to; name it (`with_item_type::<T>()`) only when
/// nothing else does.
///
/// [`with_error_type`](crate::observable::ObservableExt::with_error_type) is the twin for the
/// error type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::{from_result::FromResult, throw::Throw},
/// };
///
/// let mut values = Vec::new();
///
/// // `Throw` never emits an item, `FromResult` emits an `i32`: `concat` needs one item type.
/// FromResult::new(Ok(1))
///     .concat(Throw::new("boom").with_item_type())
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Error("boom")),
///     );
///
/// assert_eq!(values, vec![1]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WithItemType<T, OE> {
    source: OE,
    _marker: MarkerType<T>,
}

impl<T, OE> WithItemType<T, OE> {
    /// Creates a [`WithItemType`] over `source`;
    /// [`ObservableExt::with_item_type`](crate::observable::ObservableExt::with_item_type) is the
    /// fluent form.
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE> ObservableTypes for WithItemType<T, OE>
where
    OE: ObservableTypes<Item = Infallible, Error = E>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for WithItemType<T, OE>
where
    OR: Observer<T, E>,
    OE: Observable<WithItemTypeObserver<T, OR>, Item = Infallible, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = WithItemTypeObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct WithItemTypeObserver<T, OR> {
    observer: OR,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<Infallible, E> for WithItemTypeObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: Infallible) -> Flow {
        match value {}
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
    }
}
