use crate::utils::types::{MarkerType, NecessarySend};
use crate::utils::unsub_after_termination::subscribe_unsub_after_termination;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::marker::PhantomData;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct All<T, OE, F> {
    source: OE,
    callback: F,
    _marker: MarkerType<T>,
}

impl<T, OE, F> All<T, OE, F> {
    pub fn new<'or, 'sub, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> bool,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, F> Observable<'or, 'sub, bool, E> for All<T, OE, F>
where
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) -> bool + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<bool, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = AllObserver {
                observer: Some(observer),
                callback: self.callback,
            };
            self.source.subscribe(observer)
        })
    }
}

struct AllObserver<OR, F> {
    observer: Option<OR>,
    callback: F,
}

impl<T, E, OR, F> Observer<T, E> for AllObserver<OR, F>
where
    OR: Observer<bool, E>,
    F: FnMut(T) -> bool,
{
    fn on_next(&mut self, value: T) {
        if !(self.callback)(value)
            && let Some(mut observer) = self.observer.take()
        {
            observer.on_next(false);
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(mut observer) = self.observer.take() {
            match termination {
                Termination::Completed => observer.on_next(true),
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
