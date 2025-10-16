use crate::safe_lock_option_observer;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::utils::unsub_after_termination::subscribe_unsub_after_termination;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::types::MarkerType,
};
use educe::Educe;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

/// Discards items emitted by a source Observable until a second Observable emits an item.
/// See <https://reactivex.io/documentation/operators/skipuntil.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipUntil<OE, OE1> {
    source: OE,
    start: OE1,
}

impl<OE, OE1> SkipUntil<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, start: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, start }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for SkipUntil<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let started = Shared::new(AtomicBool::new(false));
            let start_observer = StartObserver {
                observer: observer.clone(),
                started: started.clone(),
                _marker: PhantomData,
            };
            let subscription_1 = self.start.subscribe(start_observer);
            let observer = SkipUntilObserver { observer, started };
            let subscription_2 = self.source.subscribe(observer);
            subscription_1 + subscription_2
        })
    }
}

struct SkipUntilObserver<OR> {
    observer: Shared<Mutable<Option<OR>>>,
    started: Shared<AtomicBool>,
}

impl<T, E, OR> Observer<T, E> for SkipUntilObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        if self.started.load(Ordering::SeqCst) {
            safe_lock_option_observer!(on_next: self.observer, value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}

struct StartObserver<T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    started: Shared<AtomicBool>,
    _marker: MarkerType<T>,
}

impl<T, E, OR> Observer<(), E> for StartObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        self.started.store(true, Ordering::SeqCst);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {}
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
