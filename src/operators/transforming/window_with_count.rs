use crate::utils::safe_lock::SafeLock;
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
};
use educe::Educe;
use std::{cmp::Ordering, num::NonZeroUsize};

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
    T: Clone + NecessarySend + 'or,
    E: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>
        + NecessarySend
        + 'or,
    ) -> Subscription<'sub> {
        let subject = PublishSubject::default();
        observer.on_next(SubjectObservable::new(subject.clone()));

        let observer = WindowWithCountObserver {
            observer,
            subject: Shared::new(Mutable::new(subject)),
            count: self.count,
            sent_count: 0,
        };
        self.source.subscribe(observer)
    }
}

struct WindowWithCountObserver<'or, T, E, OR> {
    observer: OR,
    subject: Shared<Mutable<PublishSubject<'or, T, E>>>,
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
                self.subject.safe_lock_on_next(value);
                self.sent_count += 1;
            }
            Ordering::Equal => {
                let new_subject = PublishSubject::default();
                let old_subject = std::mem::replace(
                    &mut self.subject,
                    Shared::new(Mutable::new(new_subject.clone())),
                );
                old_subject.safe_lock_on_next(value);
                old_subject
                    .safe_lock_clone()
                    .on_termination(Termination::Completed);
                self.observer.on_next(SubjectObservable::new(new_subject));
                self.sent_count = 0;
            }
            Ordering::Greater => unreachable!(),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject
            .safe_lock_clone()
            .on_termination(termination.clone());
        self.observer.on_termination(termination);
    }
}
