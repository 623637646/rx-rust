use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
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
        let context = Shared::new(Mutable::new(DebounceContext {
            current_value: None,
            timer: None,
        }));
        self.source.subscribe(DebounceObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            context: context.clone(),
            time_span: self.time_span,
            scheduler: self.scheduler,
        }) + context
    }
}

struct DebounceContext<T> {
    current_value: Option<T>,
    timer: Option<BoxedDisposal<'static>>,
}

impl<T> Disposable for Shared<Mutable<DebounceContext<T>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct DebounceObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    context: Shared<Mutable<DebounceContext<T>>>,
    time_span: Duration,
    scheduler: S,
}

impl<T, E, OR, S> Observer<T, E> for DebounceObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            let context = self.context.clone();
            let observer = self.observer.clone();
            let disposal = self.scheduler.schedule(
                move || {
                    if let Some(current_value) = safe_lock_option!(take: context, current_value) {
                        safe_lock_option_observer!(on_next: observer, current_value);
                    }
                },
                Some(self.time_span),
            );

            lock.current_value = Some(value);
            if let Some(disposal) = lock.timer.replace(BoxedDisposal::new(disposal)) {
                drop(lock);
                disposal.dispose();
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(value) = safe_lock_option!(take: self.context, current_value) {
                    safe_lock_option_observer!(on_next_and_termination: self.observer, value, termination);
                } else {
                    safe_lock_option_observer!(on_termination: self.observer, termination);
                }
            }
            Termination::Error(_) => {
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
