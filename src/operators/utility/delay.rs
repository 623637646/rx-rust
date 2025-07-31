use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::scheduler::RecursionAction;
use crate::utils::safe_lock::{SafeLock, SafeLockOption};
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use educe::Educe;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Self {
        Self {
            source,
            delay,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for Delay<OE, S>
where
    T: NecessarySend + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let context = Shared::new(Mutable::new(DelayContext {
            values: VecDeque::new(),
            timer: None,
        }));
        let delay_observer = DelayObserver {
            delay: self.delay,
            scheduler: self.scheduler,
            context: context.clone(),
            observer: Shared::new(Mutable::new(Some(observer))),
        };
        self.source.subscribe(delay_observer) + context
    }
}

struct DelayContext<T> {
    values: VecDeque<(Instant, Option<T>)>, // None means completed
    timer: Option<BoxedDisposal<'static>>,
}

impl<T> Disposable for Shared<Mutable<DelayContext<T>>> {
    fn dispose(self) {
        if let Some(timer) = self.safe_lock_mut(|e| e.timer.take()) {
            timer.dispose();
        }
    }
}

struct DelayObserver<T, OR, S> {
    delay: Duration,
    scheduler: S,
    context: Shared<Mutable<DelayContext<T>>>,
    observer: Shared<Mutable<Option<OR>>>, // None means terminated or disposed
}

impl<T, OR, S> DelayObserver<T, OR, S> {
    fn emit_value_and_setup_timer_if_needed<E>(&self, value: (Instant, Option<T>))
    where
        T: NecessarySend + 'static,
        OR: Observer<T, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        let mut lock = self.context.lock_mut();
        lock.values.push_back(value);
        if lock.timer.is_some() {
            return;
        }
        let context = self.context.clone();
        let observer = self.observer.clone();
        lock.timer = Some(BoxedDisposal::new(self.scheduler.schedule_recursively(
            move |_| {
                if let Some((instant, value)) = context.safe_lock_mut(|e| e.values.pop_front()) {
                    if let Some(value) = value {
                        //  Next
                        if observer.safe_lock_on_next_if_some(value) {
                            let mut lock = context.lock_mut();
                            if let Some((next_instant, _)) = lock.values.front() {
                                // Continue
                                let delay = next_instant.duration_since(instant);
                                drop(lock);
                                RecursionAction::ContinueAfterRevisedDelay(delay)
                            } else {
                                // No more values. Stop timer. Set timer to None.
                                if let Some(timer) = lock.timer.take() {
                                    drop(lock);
                                    timer.dispose();
                                }
                                RecursionAction::Stop
                            }
                        } else {
                            // Already terminated
                            RecursionAction::Stop
                        }
                    } else {
                        // Completed
                        observer.safe_lock_on_termination_if_some(Termination::Completed);
                        RecursionAction::Stop
                    }
                } else {
                    unreachable!()
                }
            },
            Some(self.delay),
        )));
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.emit_value_and_setup_timer_if_needed((Instant::now(), Some(value)));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.emit_value_and_setup_timer_if_needed((Instant::now(), None));
            }
            Termination::Error(_) => {
                self.context.dispose();
                self.observer.safe_lock_on_termination_if_some(termination);
            }
        }
    }
}
