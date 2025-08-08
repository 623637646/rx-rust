use crate::safe_lock_option_observer;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::collections::VecDeque;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Zip<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Zip<OE1, OE2> {
    pub fn new<'or, 'sub, T1, T2, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T1, E>,
        OE2: Observable<'or, 'sub, T2, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T1, T2, E, OE1, OE2> Observable<'or, 'sub, (T1, T2), E> for Zip<OE1, OE2>
where
    T1: NecessarySend + 'or,
    T2: NecessarySend + 'or,
    OE1: Observable<'or, 'sub, T1, E>,
    OE2: Observable<'or, 'sub, T2, E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<(T1, T2), E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let buffer = Shared::new(Mutable::new(ZipObserverBufferState::None));
            let observer_1 = ZipObserver1 {
                observer: observer.clone(),
                buffer: buffer.clone(),
            };
            let observer_2 = ZipObserver2 { observer, buffer };
            let subscription_1 = self.source_1.subscribe(observer_1);
            let subscription_2 = self.source_2.subscribe(observer_2);
            subscription_1 + subscription_2
        })
    }
}

enum ZipObserverBufferState<T1, T2> {
    None,
    One(VecDeque<T1>),
    Two(VecDeque<T2>),
}

struct ZipObserver1<T1, T2, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    buffer: Shared<Mutable<ZipObserverBufferState<T1, T2>>>,
}

impl<T1, T2, E, OR> Observer<T1, E> for ZipObserver1<T1, T2, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T1) {
        self.buffer.lock_mut(|mut lock| match &mut *lock {
            ZipObserverBufferState::None => {
                *lock = ZipObserverBufferState::One(VecDeque::from([value]));
            }
            ZipObserverBufferState::One(items) => {
                items.push_back(value);
            }
            ZipObserverBufferState::Two(items) => {
                let item = items.pop_front().unwrap();
                if items.is_empty() {
                    *lock = ZipObserverBufferState::None;
                }
                drop(lock);
                safe_lock_option_observer!(on_next: self.observer, (value, item));
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}

struct ZipObserver2<T1, T2, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    buffer: Shared<Mutable<ZipObserverBufferState<T1, T2>>>,
}

impl<T1, T2, E, OR> Observer<T2, E> for ZipObserver2<T1, T2, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T2) {
        self.buffer.lock_mut(|mut lock| match &mut *lock {
            ZipObserverBufferState::None => {
                *lock = ZipObserverBufferState::Two(VecDeque::from([value]));
            }
            ZipObserverBufferState::One(items) => {
                let item = items.pop_front().unwrap();
                if items.is_empty() {
                    *lock = ZipObserverBufferState::None;
                }
                drop(lock);
                safe_lock_option_observer!(on_next: self.observer, (item, value));
            }
            ZipObserverBufferState::Two(items) => {
                items.push_back(value);
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}
