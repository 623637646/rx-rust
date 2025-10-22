use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Event, Observer, Termination},
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an Observable into an Observable that emits `Event` objects, each of which wraps a notification from the source Observable.
/// See <https://reactivex.io/documentation/operators/materialize-dematerialize.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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
    pub fn new(source: OE) -> Self {
        Self(source)
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, Event<T, E>, Infallible> for Materialize<OE>
where
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        observer: impl Observer<Event<T, E>, Infallible> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        self.0.subscribe(MaterializeObserver(observer))
    }
}

struct MaterializeObserver<OR>(OR);

impl<T, E, OR> Observer<T, E> for MaterializeObserver<OR>
where
    OR: Observer<Event<T, E>, Infallible>,
{
    fn on_next(&mut self, value: T) {
        self.0.on_next(Event::Next(value));
    }

    fn on_termination(mut self, termination: Termination<E>) {
        self.0.on_next(Event::Termination(termination));
        self.0.on_termination(Termination::Completed);
    }
}
