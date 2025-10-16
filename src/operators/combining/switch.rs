use crate::disposable::Disposable;
use crate::disposable::subscription::Subscription;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    operators::creating::from_iter::FromIter,
    utils::{types::MarkerType, unsub_after_termination::subscribe_unsub_after_termination},
};
use crate::{safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
use educe::Educe;
use std::marker::PhantomData;

/// Converts an Observable that emits Observables into a single Observable that emits the items emitted by the most recently emitted of those Observables.
/// See <https://reactivex.io/documentation/operators/switch.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Switch<OE, OE1> {
    source: OE,
    _marker: MarkerType<OE1>,
}

impl<OE, OE1> Switch<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE) -> Self
    where
        OE: Observable<'or, 'sub, OE1, E>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            _marker: PhantomData,
        }
    }
}

impl<OE1, I> Switch<FromIter<I>, OE1> {
    pub fn new_from_iter<'or, 'sub, T, E>(into_iterator: I) -> Self
    where
        I: IntoIterator<Item = OE1>,
        OE1: Observable<'or, 'sub, T, E>,
    {
        Self {
            source: FromIter::new(into_iterator),
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for Switch<OE, OE1>
where
    T: 'or,
    OE: Observable<'or, 'sub, OE1, E>,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let context = Shared::new(Mutable::new(SwitchContext {
                on_going_sub: None,
                completed: false,
            }));
            let observer = SwitchObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                context: context.clone(),
                _marker: PhantomData,
            };
            self.source.subscribe(observer) + context
        })
    }
}

struct SwitchContext<'sub> {
    on_going_sub: Option<Subscription<'sub>>,
    completed: bool,
}

impl Disposable for Shared<Mutable<SwitchContext<'_>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, on_going_sub);
    }
}

struct SwitchObserver<'sub, T, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<SwitchContext<'sub>>>,
    _marker: MarkerType<T>,
}

impl<'or, 'sub, T, E, OR, OE1> Observer<OE1, E> for SwitchObserver<'sub, T, OR>
where
    OR: Observer<T, E> + NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn on_next(&mut self, value: OE1) {
        // Use a placeholder subscription.
        if let Some(on_going_sub) =
            safe_lock_option!(replace: self.context, on_going_sub, Subscription::default())
        {
            on_going_sub.dispose();
        }
        let observer = SwitchInnerObserver {
            observer: self.observer.clone(),
            context: self.context.clone(),
        };
        let sub = value.subscribe(observer);
        self.context.lock_mut(|mut lock| {
            if lock.on_going_sub.is_some() {
                lock.on_going_sub = Some(sub);
            } else {
                // already terminated
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.context.lock_mut(|mut lock| {
                    lock.completed = true;
                    if lock.on_going_sub.is_none() {
                        drop(lock);
                        safe_lock_option_observer!(on_termination: self.observer, termination);
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}

struct SwitchInnerObserver<'sub, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<SwitchContext<'sub>>>,
}

impl<T, E, OR> Observer<T, E> for SwitchInnerObserver<'_, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_option_observer!(on_next: self.observer, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.context.lock_mut(|mut lock| {
                    if lock.completed {
                        drop(lock);
                        safe_lock_option_observer!(on_termination: self.observer, termination);
                    } else if let Some(on_going_sub) = lock.on_going_sub.take() {
                        drop(lock);
                        on_going_sub.dispose();
                    }
                });
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
