use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Event, Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::convert::Infallible;

/// Converts an Observable that emits `Event` objects into a "live" Observable that emits the items and notifications embedded in those `Event` objects.
/// See <https://reactivex.io/documentation/operators/materialize-dematerialize.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
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

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for Dematerialize<OE>
where
    OE: Observable<'or, 'sub, Event<T, E>, Infallible>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySendSync + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            self.0.subscribe(DematerializeObserver(Some(observer)))
        })
    }
}

struct DematerializeObserver<OR>(Option<OR>);

impl<T, E, OR> Observer<Event<T, E>, Infallible> for DematerializeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: Event<T, E>) {
        match value {
            Event::Next(value) => {
                if let Some(observer) = self.0.as_mut() {
                    observer.on_next(value);
                }
            }
            Event::Termination(termination) => {
                if let Some(observer) = self.0.take() {
                    observer.on_termination(termination);
                }
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
            Termination::Error(_) => unreachable!(),
        }
    }
}
