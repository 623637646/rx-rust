use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
    subscription::Subscription,
};
use educe::Educe;
use std::{
    cmp::Ordering,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> WindowWithCount<OE> {
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E>
    for WindowWithCount<OE>
where
    T: Clone + 'or,
    E: Clone + Send + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E> + Send + 'or,
    ) -> Subscription<'sub> {
        let subject = PublishSubject::default();
        observer.on_next(SubjectObservable::new(subject.clone()));

        let observer = WindowWithCountObserver {
            observer,
            subject: Arc::new(Mutex::new(subject)),
            count: self.count,
            sent_count: 0,
        };
        self.source.subscribe(observer)
    }
}

struct WindowWithCountObserver<'or, T, E, OR> {
    observer: OR,
    subject: Arc<Mutex<PublishSubject<'or, T, E>>>,
    count: NonZeroUsize,
    sent_count: usize,
}

impl<'or, T, E, OR> Observer<T, E> for WindowWithCountObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, value: T) {
        match (self.sent_count + 1).cmp(&self.count.get()) {
            Ordering::Less => {
                self.subject.lock().unwrap().on_next(value);
                self.sent_count += 1;
            }
            Ordering::Equal => {
                let new_subject = PublishSubject::default();
                let old_subject =
                    std::mem::replace(&mut self.subject, Arc::new(Mutex::new(new_subject.clone())));
                old_subject.lock().unwrap().on_next(value);
                old_subject
                    .lock()
                    .unwrap()
                    .clone()
                    .on_termination(Termination::Completed);
                self.observer.on_next(SubjectObservable::new(new_subject));
                self.sent_count = 0;
            }
            Ordering::Greater => unreachable!(),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject
            .lock()
            .unwrap()
            .clone()
            .on_termination(termination.clone());
        self.observer.on_termination(termination);
    }
}
