use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Discards items emitted by an Observable until a specified condition becomes false.
/// See <https://reactivex.io/documentation/operators/skipwhile.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipWhile<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> SkipWhile<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for SkipWhile<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> bool + NecessarySend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = SkipWhileObserver {
            observer,
            callback: self.callback,
            skip: true,
        };
        self.source.subscribe(observer)
    }
}

struct SkipWhileObserver<OR, F> {
    observer: OR,
    callback: F,
    skip: bool,
}

impl<T, E, OR, F> Observer<T, E> for SkipWhileObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) {
        if !self.skip {
            self.observer.on_next(value);
        } else {
            self.skip = (self.callback)(&value);
            if !self.skip {
                self.observer.on_next(value);
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
