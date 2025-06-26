use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

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
    T1: Send + 'or,
    T2: Send + 'or,
    OE1: Observable<'or, 'sub, T1, E>,
    OE2: Observable<'or, 'sub, T2, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<(T1, T2), E> + Send + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Arc::new(Mutex::new(Some(observer)));
            let buffer = Arc::new(Mutex::new(ZipObserverBufferState::None));
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
    observer: Arc<Mutex<Option<OR>>>,
    buffer: Arc<Mutex<ZipObserverBufferState<T1, T2>>>,
}

impl<T1, T2, E, OR> Observer<T1, E> for ZipObserver1<T1, T2, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T1) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            let mut lock = self.buffer.lock().unwrap();
            match &mut *lock {
                ZipObserverBufferState::None => {
                    *lock = ZipObserverBufferState::One(VecDeque::from([value]))
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
                    observer.on_next((value, item))
                }
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}

struct ZipObserver2<T1, T2, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    buffer: Arc<Mutex<ZipObserverBufferState<T1, T2>>>,
}

impl<T1, T2, E, OR> Observer<T2, E> for ZipObserver2<T1, T2, OR>
where
    OR: Observer<(T1, T2), E>,
{
    fn on_next(&mut self, value: T2) {
        if let Some(observer) = self.observer.lock().unwrap().as_mut() {
            let mut lock = self.buffer.lock().unwrap();
            match &mut *lock {
                ZipObserverBufferState::None => {
                    *lock = ZipObserverBufferState::Two(VecDeque::from([value]))
                }
                ZipObserverBufferState::One(items) => {
                    let item = items.pop_front().unwrap();
                    if items.is_empty() {
                        *lock = ZipObserverBufferState::None;
                    }
                    drop(lock);
                    observer.on_next((item, value))
                }
                ZipObserverBufferState::Two(items) => {
                    items.push_back(value);
                }
            }
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(observer) = { self.observer.lock().unwrap().take() } {
            observer.on_termination(termination);
        }
    }
}
