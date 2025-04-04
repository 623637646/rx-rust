use super::{Observable, Observer};
use crate::subscription::Subscription;

/// TODO: doc
/// https://stackoverflow.com/a/56447952/9315497
pub struct BoxedObservable<'a, 'b, OR>(Box<dyn FnOnce(OR) -> Subscription<'a> + 'b>);

impl<'a, 'b, OR> BoxedObservable<'a, 'b, OR> {
    pub fn new<T, E>(observable: impl Observable<'a, T, E, OR> + Send + 'b) -> Self
    where
        OR: Observer<T, E>,
    {
        BoxedObservable(Box::new(|observer| observable.subscribe(observer)))
    }
}

impl<'a, T, E, OR> Observable<'a, T, E, OR> for BoxedObservable<'a, '_, OR>
where
    OR: Observer<T, E>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        self.0(observer)
    }
}

// TODO: Unit Tests
