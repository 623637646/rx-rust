use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subject::{
        Subject,
        publish_subject::{PublishObservable, PublishSubject},
    },
    subscription::Subscription,
    utils::marker::MarkerType,
};
use educe::Educe;
use std::{collections::HashMap, hash::Hash, marker::PhantomData};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct GroupBy<OE, F, K> {
    source: OE,
    callback: F,
    _marker: MarkerType<K>,
}

impl<OE, F, K> GroupBy<OE, F, K> {
    pub fn new<'or, 'sub, T, E>(source: OE, callback: F) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        F: FnMut(T) -> K,
    {
        Self {
            source,
            callback,
            _marker: PhantomData,
        }
    }
}

impl<'or, 'sub, T, E, OE, F, K> Observable<'or, 'sub, PublishObservable<'or, T, E>, E>
    for GroupBy<OE, F, K>
where
    T: Clone + 'or,
    E: Clone + Send + Sync + 'or,
    OE: Observable<'or, 'sub, T, E>,
    F: FnMut(T) -> K + Send + 'or,
    K: Eq + Hash + Send + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<PublishObservable<'or, T, E>, E> + Send + 'or,
    ) -> Subscription<'sub> {
        let observer = GroupByObserver {
            observer,
            callback: self.callback,
            subjects: HashMap::default(),
        };
        self.source.subscribe(observer)
    }
}

struct GroupByObserver<'or, T, E, OR, F, K> {
    observer: OR,
    callback: F,
    subjects: HashMap<K, PublishSubject<'or, T, E>>,
}

impl<'or, T, E, OR, F, K> Observer<T, E> for GroupByObserver<'or, T, E, OR, F, K>
where
    T: Clone,
    E: Clone + Send,
    OR: Observer<PublishObservable<'or, T, E>, E>,
    F: FnMut(T) -> K,
    K: Eq + Hash,
{
    fn on_next(&mut self, value: T) {
        let key = (self.callback)(value.clone());
        let mut subject = self
            .subjects
            .entry(key)
            .or_insert_with(|| {
                let subject = PublishSubject::new();
                self.observer.on_next(subject.clone().into_observable());
                subject
            })
            .clone();
        subject.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subjects
            .into_values()
            .for_each(|subject| subject.on_termination(termination.clone()));
        self.observer.on_termination(termination);
    }
}
