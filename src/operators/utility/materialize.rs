//! The [`Materialize`] operator, behind
//! [`ObservableExt::materialize`](crate::observable::ObservableExt::materialize).

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Event, Flow, Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an Observable into one that emits each of its events, the termination included, as an
/// [`Event`] item, then completes.
/// See <https://reactivex.io/documentation/operators/materialize-dematerialize.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Event, Termination},
///     operators::{
///         creating::from_iter::FromIter,
///         utility::materialize::Materialize,
///     },
/// };
///
/// let mut events = Vec::new();
/// let mut terminations = Vec::new();
///
/// Materialize::new(FromIter::new(vec![1, 2])).subscribe_with_callback(
///     |event| events.push(event),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(
///     events,
///     vec![
///         Event::Next(1),
///         Event::Next(2),
///         Event::Termination(Termination::Completed)
///     ]
/// );
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Materialize<OE>(OE);

impl<OE> Materialize<OE> {
    /// Creates a [`Materialize`] over `source`;
    /// [`ObservableExt::materialize`](crate::observable::ObservableExt::materialize) is the fluent
    /// form.
    pub fn new(source: OE) -> Self {
        Self(source)
    }
}

impl<T, E, OE> ObservableTypes for Materialize<OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = Event<T, E>;
    type Error = Infallible;
    type Mode = OE::Mode;
    type Disposal = OE::Disposal;
}

impl<T, E, OE, OR> Observable<OR> for Materialize<OE>
where
    OR: Observer<Event<T, E>, Infallible>,
    OE: Observable<MaterializeObserver<OR>, Item = T, Error = E>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.0.subscribe(MaterializeObserver(observer))
    }
}

pub struct MaterializeObserver<OR>(OR);

impl<T, E, OR> Observer<T, E> for MaterializeObserver<OR>
where
    OR: Observer<Event<T, E>, Infallible>,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.on_next(Event::Next(value))
    }

    fn on_termination(mut self, termination: Termination<E>) {
        // The materialized termination is the last value of the stream, so a downstream that
        // stopped on it is not completed on top of that.
        if self
            .0
            .on_next(Event::Termination(termination))
            .is_continue()
        {
            self.0.on_termination(Termination::Completed);
        }
    }
}
