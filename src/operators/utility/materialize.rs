use crate::utils::types::NecessarySend;
use crate::{
    observable::Observable,
    observer::{Event, Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;
use std::convert::Infallible;

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
