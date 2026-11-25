//! Gives an error type to an observable that never fails.
//!
//! The [`WithErrorType`] operator, behind
//! [`ObservableExt::with_error_type`](crate::observable::ObservableExt::with_error_type).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Gives an observable whose error type is `Infallible` a concrete error type.
///
/// A source that cannot fail — `Just`, `FromIter`, `Interval`, … — says so with
/// `Error = Infallible`, which is what lets [`into_future`] resolve to the value with no `Result`.
/// The operators that join several observables (`merge`, `concat`, `amb`, `switch_if_empty`,
/// the inner observables of `catch`, `retry` and the `*_map` operators) require their errors to be
/// the same type, though, and Rust does not turn `Infallible` into another type on its own: this
/// operator is that step, done explicitly where the types meet. It does nothing at run time.
///
/// It is the same as `map_err(|error| match error {})`, without the closure. The error type is
/// usually inferred from the observable it is joined to; name it (`with_error_type::<E>()`) only
/// when nothing else does.
///
/// [`with_item_type`](crate::observable::ObservableExt::with_item_type) is the twin for the item
/// type.
///
/// [`into_future`]: crate::observable::ObservableExt::into_future
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::{from_iter::FromIter, from_result::FromResult},
/// };
///
/// let mut values = Vec::new();
///
/// // `FromIter` cannot fail, `FromResult` fails with a `&str`: `concat` needs one error type.
/// FromIter::new([1, 2])
///     .with_error_type()
///     .concat(FromResult::new(Err("boom")))
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| assert_eq!(termination, Termination::Error("boom")),
///     );
///
/// assert_eq!(values, vec![1, 2]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WithErrorType<E, OE> {
    source: OE,
    _marker: MarkerType<E>,
}

impl<E, OE> WithErrorType<E, OE> {
    /// Creates a [`WithErrorType`] over `source`;
    /// [`ObservableExt::with_error_type`](crate::observable::ObservableExt::with_error_type) is the
    /// fluent form.
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE> ObservableTypes for WithErrorType<E, OE>
where
    OE: ObservableTypes<Item = T, Error = Infallible>,
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for WithErrorType<E, OE>
where
    OR: Observer<T, E>,
    OE: Observable<WithErrorTypeObserver<E, OR>, Item = T, Error = Infallible>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let observer = WithErrorTypeObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

#[doc(hidden)]
pub struct WithErrorTypeObserver<E, OR> {
    observer: OR,
    _marker: MarkerType<E>,
}

impl<T, E, OR> Observer<T, Infallible> for WithErrorTypeObserver<E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<Infallible>) {
        match termination {
            Termination::Completed => self.observer.on_termination(Termination::Completed),
            Termination::Error(error) => match error {},
        }
    }
}
