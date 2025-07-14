use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::shared_disposal::SharedDisposal;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
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
    S: Scheduler + NecessarySend + 'or,
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
        if self.observer.lock_ref().is_none() {
            return;
        }
        *self.current_value.lock_mut() = Some(value);

        let current_value = self.current_value.clone();
        let observer = self.observer.clone();
        let disposal = self.scheduler.clone().schedule(
            move || {
                if let Some(value) = { current_value.lock_mut().take() } {
                    if let Some(observer) = observer.lock_mut().as_mut() {
                        observer.on_next(value);
                    }
                }
            },
            Some(self.time_span),
        );

        if let Some(disposal) = {
            self.disposal
                .lock_mut()
                .replace(BoxedDisposal::new(disposal))
        } {
            disposal.dispose();
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        if let Some(disposal) = { self.disposal.lock_mut().take() } {
            disposal.dispose();
        }
        if let Some(mut observer) = { self.observer.lock_mut().take() } {
            match termination {
                Termination::Completed => {
                    if let Some(value) = { self.current_value.lock_mut().take() } {
                        observer.on_next(value);
                    }
                }
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
