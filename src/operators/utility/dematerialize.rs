use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Event, Flow, Observer, Termination},
    utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an Observable that emits `Event` objects into a "live" Observable that emits the items and notifications embedded in those `Event` objects.
/// See <https://reactivex.io/documentation/operators/materialize-dematerialize.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         utility::{
///             dematerialize::Dematerialize,
///             materialize::Materialize,
///         },
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// Dematerialize::new(Materialize::new(FromIter::new(vec![1, 2])))
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
pub struct Dematerialize<OE>(OE);

impl<OE> Dematerialize<OE> {
    pub fn new(source: OE) -> Self {
        Self(source)
    }
}

impl<'or, T, E, OE> Observable<'or, T, E> for Dematerialize<OE>
where
    OE: Observable<'or, Event<T, E>, Infallible>,
    OE::D: MaybeSend + 'or,
{
    type D = crate::utils::subscribe_with_auto_dispose_on_termination::Disposal<OE::D>;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        subscribe_with_auto_dispose_on_termination(observer, |observer| {
            self.0.subscribe(DematerializeObserver(Some(observer)))
        })
    }
}

struct DematerializeObserver<OR>(Option<OR>);

impl<T, E, OR> Observer<Event<T, E>, Infallible> for DematerializeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: Event<T, E>) -> Flow {
        match value {
            Event::Next(value) => match self.0.as_mut() {
                Some(observer) => {
                    let flow = observer.on_next(value);
                    if flow.is_stop() {
                        drop(self.0.take());
                    }
                    flow
                }
                // The materialized termination already ended the stream downstream.
                None => Flow::Stop,
            },
            Event::Termination(termination) => {
                if let Some(observer) = self.0.take() {
                    observer.on_termination(termination);
                }
                // The stream ends with the value that carried the termination, so whatever the
                // source has left is of no use anymore.
                Flow::Stop
            }
        }
    }

    fn on_termination(mut self, termination: Termination<Infallible>) {
        match termination {
            Termination::Completed => {
                if let Some(observer) = self.0.take() {
                    observer.on_termination(Termination::Completed);
                }
            }
            // `Infallible` is uninhabited, so the compiler proves this arm unreachable.
            Termination::Error(error) => match error {},
        }
    }
}
