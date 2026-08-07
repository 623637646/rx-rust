use super::{Subject, publish_subject::PublishSubject};
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::safe_lock;
use crate::subject::publish_subject;
use crate::utils::types::{MaybeSend, Mutable, MutableHelper, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::collections::VecDeque;

/// Buffers emissions and replays them to late subscribers.
///
/// A subscriber that arrives after the subject terminated observes the buffered values followed by
/// the termination, whether the subject completed or errored.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ReplaySubject<'or, T, E> {
    values: Shared<Mutable<VecDeque<T>>>,
    buffer_size: Option<usize>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> ReplaySubject<'_, T, E> {
    pub fn new(buffer_size: Option<usize>) -> Self {
        let vec = match buffer_size {
            Some(size) => VecDeque::with_capacity(size),
            None => VecDeque::new(),
        };
        Self {
            values: Shared::new(Mutable::new(vec)),
            buffer_size,
            publish_subject: PublishSubject::default(),
        }
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<Subscription<publish_subject::Disposal<'or, T, E>>>,
    where T: Clone, E: Clone
);

impl<'or, T, E> Observable<'or, T, E> for ReplaySubject<'or, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    type D = Disposal<'or, T, E>;

    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        if let Some(terminated) = self.terminated() {
            // The buffer is the history of the subject, so it is replayed whichever way the
            // subject terminated: an error does not erase what was emitted before it.
            let values = safe_lock!(clone: self.values);
            for value in values {
                observer.on_next(value);
            }
            observer.on_termination(terminated);
            OptionDisposal::none().into_subscription()
        } else {
            let values = safe_lock!(clone: self.values);
            for value in values {
                observer.on_next(value);
            }
            self.publish_subject
                .subscribe(observer)
                .into_option()
                .into_subscription()
        }
    }
}

impl<T, E> Observer<T, E> for ReplaySubject<'_, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            self.values.lock_mut(|mut lock| {
                if let Some(buffer_size) = self.buffer_size {
                    if lock.len() == buffer_size {
                        if lock.pop_front().is_some() {
                            // only push if the buffer is not 0
                            lock.push_back(value.clone());
                        }
                    } else {
                        lock.push_back(value.clone());
                    }
                } else {
                    lock.push_back(value.clone());
                }
            });
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, T, E> Subject<'or, T, E> for ReplaySubject<'or, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }
}
