use super::{Subject, publish_subject::PublishSubject};
use crate::delegate_disposal;
use crate::disposable::DisposableExt;
use crate::disposable::option_disposal::OptionDisposal;
use crate::observable::Subscription;
use crate::subject::publish_subject;
use crate::utils::types::{MaybeSend, Mutable, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
};
use crate::{safe_lock, safe_lock_option};
use educe::Educe;

/// Remembers only the last emission and replays it on completion.
#[derive(Educe)]
#[educe(Debug, Clone, Default)]
pub struct AsyncSubject<'or, T, E> {
    value: Shared<Mutable<Option<T>>>,
    publish_subject: PublishSubject<'or, T, E>,
}

impl<T, E> AsyncSubject<'_, T, E> {
    pub fn new() -> Self {
        Self {
            value: Shared::new(Mutable::new(None)),
            publish_subject: PublishSubject::default(),
        }
    }
}

delegate_disposal!(
    Disposal<'or, T, E>,
    OptionDisposal<Subscription<publish_subject::Disposal<'or, T, E>>>
);

impl<'or, T, E> Observable<'or> for AsyncSubject<'or, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    type T = T;
    type E = E;
    type D = Disposal<'or, T, E>;

    fn subscribe(
        self,
        mut observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        if let Some(terminated) = self.terminated() {
            match &terminated {
                Termination::Completed => {
                    if let Some(value) = safe_lock!(clone: self.value) {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(terminated);
            OptionDisposal::none().into()
        } else {
            self.publish_subject
                .subscribe(observer)
                .into_option()
                .into()
        }
    }
}

impl<T, E> Observer<T, E> for AsyncSubject<'_, T, E>
where
    T: Clone + MaybeSend,
    E: Clone + MaybeSend,
{
    fn on_next(&mut self, value: T) {
        if self.terminated().is_none() {
            safe_lock_option!(replace: self.value, value);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match &termination {
            Termination::Completed => {
                if let Some(value) = safe_lock!(clone: self.value) {
                    self.publish_subject.on_next(value);
                }
            }
            Termination::Error(_) => {}
        }
        self.publish_subject.on_termination(termination);
    }
}

impl<'or, T, E> Subject<'or> for AsyncSubject<'or, T, E>
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
