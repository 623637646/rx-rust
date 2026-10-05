//! The [`WithErrorType`] operator, behind
//! [`ObservableExt::with_error_type`](crate::observable::ObservableExt::with_error_type).

use crate::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::{Flow, Observer, Termination},
    utils::MarkerType,
};
use educe::Educe;
use std::{convert::Infallible, marker::PhantomData};

/// Gives an Observable whose error type is `Infallible` a concrete error type.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::creating::from_iter::FromIter,
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// FromIter::new(vec![1, 2])
///     .with_error_type::<String>()
///     .subscribe_with_callback(
///         |value| values.push(value),
///         |termination| terminations.push(termination),
///     );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
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
    type D = OE::D;
}

impl<T, E, OE, OR> Observable<OR> for WithErrorType<E, OE>
where
    OR: Observer<T, E>,
    OE: Observable<WithErrorTypeObserver<E, OR>, Item = T, Error = Infallible>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let observer = WithErrorTypeObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

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
