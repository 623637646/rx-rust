use super::{Subject, publish_subject::PublishSubject};
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::safe_lock;
use crate::subject::publish_subject;
use crate::utils::types::{MaybeSend, Mutable, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Keeps the latest value and emits it immediately to new subscribers.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BehaviorSubject<'or, T, E> {
    value: Shared<Mutable<T>>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> BehaviorSubject<'_, T, E> {
    pub fn new(value: T) -> Self {
        Self {
            value: Shared::new(Mutable::new(value)),
            publish_subject: PublishSubject::default(),
        }
    }

    pub fn value(&self) -> T
    where
        T: Clone,
    {
        safe_lock!(clone: self.value)
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<Subscription<publish_subject::Disposal<'or, T, E>>>,
    where T: Clone, E: Clone
);

impl<'or, T, E> Observable<'or, T, E> for BehaviorSubject<'or, T, E>
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
            observer.on_termination(terminated);
            OptionDisposal::none().into_subscription()
        } else {
            observer.on_next(safe_lock!(clone: self.value));
            self.publish_subject
                .subscribe(observer)
                .into_option()
                .into_subscription()
        }
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            safe_lock!(set: self.value, value.clone());
            self.publish_subject.on_next(value);
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, T, E> Subject<'or, T, E> for BehaviorSubject<'or, T, E>
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
