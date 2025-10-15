use crate::utils::types::NecessarySend;
use crate::utils::unsub_after_termination::subscribe_unsub_after_termination;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct TakeWhile<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> TakeWhile<OE, F> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(&T) -> bool,
    {
        Self { source, callback }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, T, E> for TakeWhile<OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(&T) -> bool + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = TakeWhileObserver {
                observer: Some(observer),
                callback: self.callback,
            };
            self.source.subscribe(observer)
        })
    }
}

struct TakeWhileObserver<OR, F> {
    observer: Option<OR>,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for TakeWhileObserver<OR, F>
where
    OR: Observer<T, E>,
    F: FnMut(&T) -> bool,
{
    fn on_next(&mut self, value: T) {
        let r#continue = (self.callback)(&value);
        if r#continue {
            if let Some(observer) = self.observer.as_mut() {
                observer.on_next(value);
            }
        } else if let Some(observer) = self.observer.take() {
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = self.observer {
            observer.on_termination(termination);
        }
    }
}
