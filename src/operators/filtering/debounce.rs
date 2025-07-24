use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::shared_disposal::SharedDisposal;
use crate::utils::safe_lock::{SafeLock, SafeLockOption, SafeLockOptionObserver};
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::time::Duration;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debounce<OE, S> {
    source: OE,
    time_span: Duration,
    scheduler: S,
}

impl<OE, S> Debounce<OE, S> {
    pub fn new(source: OE, time_span: Duration, scheduler: S) -> Self {
        Self {
            source,
            time_span,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Debounce<OE, S>
where
    T: NecessarySend + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let disposal = Shared::new(Mutable::new(None));
        self.source.subscribe(DebounceObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            time_span: self.time_span,
            scheduler: self.scheduler,
            current_value: Shared::new(Mutable::new(None)),
            disposal: disposal.clone(),
        }) + SharedDisposal::new(disposal)
    }
}

struct DebounceObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    time_span: Duration,
    scheduler: S,
    current_value: Shared<Mutable<Option<T>>>,
    disposal: Shared<Mutable<Option<BoxedDisposal<'static>>>>,
}

impl<T, E, OR, S> Observer<T, E> for DebounceObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        if self.observer.safe_lock_is_none() {
            return;
        }
        self.current_value.safe_lock_set(Some(value));

        let current_value = self.current_value.clone();
        let observer = self.observer.clone();
        let disposal = self.scheduler.clone().schedule(
            move || {
                if let Some(value) = current_value.safe_lock_take() {
                    observer.safe_lock_on_next_if_some(value);
                }
            },
            Some(self.time_span),
        );

        if let Some(disposal) = self
            .disposal
            .safe_lock_replace(BoxedDisposal::new(disposal))
        {
            disposal.dispose();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(disposal) = self.disposal.safe_lock_take() {
            disposal.dispose();
        }
        if let Some(mut observer) = self.observer.safe_lock_take() {
            match termination {
                Termination::Completed => {
                    if let Some(value) = self.current_value.safe_lock_take() {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
