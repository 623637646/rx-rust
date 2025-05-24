use crate::{
    observable::Observable,
    observer::{Event, Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

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
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        self.0.subscribe(DematerializeObserver(Some(observer)))
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
