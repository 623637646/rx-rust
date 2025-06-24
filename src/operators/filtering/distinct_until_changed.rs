use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DistinctUntilChanged<OE, F1, F2> {
    source: OE,
    key_selector: F1,
    equals: F2,
}

impl<OE, F1, F2> DistinctUntilChanged<OE, F1, F2> {
    pub fn new_with_key_selector_and_equals<'or, 'sub, T, E, K>(
        source: OE,
        key_selector: F1,
        equals: F2,
    ) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F1: FnMut(&T) -> K,
        F2: FnMut(&K, &K) -> bool,
    {
        Self {
            source,
            key_selector,
            equals,
        }
    }
}

pub type DistinctUntilChangedConvenientType<T, OE> =
    DistinctUntilChanged<OE, fn(&T) -> T, fn(&T, &T) -> bool>;

impl<T, OE> DistinctUntilChangedConvenientType<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE) -> Self
    where
        T: Clone + Eq,
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            key_selector: |x| x.clone(),
            equals: |x, y| x == y,
        }
    }
}

impl<'or, 'sub, T, E, OE, F1, F2, K> Observable<'or, 'sub, T, E>
    for DistinctUntilChanged<OE, F1, F2>
where
    OE: Observable<'or, 'sub, T, E>,
    F1: FnMut(&T) -> K + Send + 'or,
    F2: FnMut(&K, &K) -> bool + Send + 'or,
    K: Send + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        let observer = DistinctUntilChangedObserver {
            observer,
            key_selector: self.key_selector,
            equals: self.equals,
            previous_key: None,
        };
        self.source.subscribe(observer)
    }
}

struct DistinctUntilChangedObserver<OR, F1, F2, K> {
    observer: OR,
    key_selector: F1,
    equals: F2,
    previous_key: Option<K>,
}

impl<T, E, OR, F1, F2, K> Observer<T, E> for DistinctUntilChangedObserver<OR, F1, F2, K>
where
    OR: Observer<T, E>,
    F1: FnMut(&T) -> K,
    F2: FnMut(&K, &K) -> bool,
{
    fn on_next(&mut self, value: T) {
        let key = (self.key_selector)(&value);
        if let Some(previous_key) = self.previous_key.as_ref() {
            if (self.equals)(previous_key, &key) {
                return;
            }
        }
        self.previous_key = Some(key);
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination)
    }
}
