use crate::utils::types::MaybeSend;
use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    utils::types::MarkerType,
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
    pub fn new(source: OE) -> Self {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for WithErrorType<E, OE>
where
    E: 'or,
    OE: Observable<'or, T, Infallible>,
{
    type D = OE::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        let observer = WithErrorTypeObserver {
            observer,
            _marker: PhantomData,
        };
        self.source.subscribe(observer)
    }
}

struct WithErrorTypeObserver<E, OR> {
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
