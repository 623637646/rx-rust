use crate::disposable::Disposable;
use crate::disposable::boxed_disposal::BoxedDisposal;
use crate::disposable::subscription::Subscription;
use crate::scheduler::RecursionAction;
use crate::utils::types::{Mutable, MutableHelper, NecessarySend, Shared};
use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    scheduler::Scheduler,
};
use crate::{safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer};
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
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct DelayObserver<T, OR, S> {
    delay: Duration,
    scheduler: S,
    context: Shared<Mutable<DelayContext<T>>>,
    observer: Shared<Mutable<Option<OR>>>, // None means terminated or disposed
}

impl<T, OR, S> DelayObserver<T, OR, S> {
    fn emit_value_and_setup_timer_if_needed<E>(&self, value: Option<T>)
    where
        T: NecessarySend + 'static,
        OR: Observer<T, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        self.context.lock_mut(|mut lock| {
            lock.values.push_back((Instant::now() + self.delay, value));
            if lock.timer.is_some() {
                return;
            }
            let context = self.context.clone();
            let observer = self.observer.clone();
            lock.timer = Some(BoxedDisposal::new(self.scheduler.schedule_recursively(
                move |_| {
                    // Get values that should be sent
                    let values_should_be_sent = context.lock_mut(|mut lock| {
                        let mut values_should_be_sent = Vec::new();
                        let now = Instant::now();
                        while let Some((instant, _)) = lock.values.front() {
                            if now < *instant {
                                break;
                            }
                            values_should_be_sent.push(lock.values.pop_front().unwrap().1);
                        }
                        assert!(!values_should_be_sent.is_empty());
                        values_should_be_sent
                    });

                    let completed = observer.lock_mut(|mut lock| {
                        // Send values
                        for value in values_should_be_sent {
                            if let Some(value) = value {
                                //  Next
                                lock.as_mut().unwrap().on_next(value);
                            } else {
                                // Completed
                                let observer = lock.take().unwrap();
                                drop(lock);
                                observer.on_termination(Termination::Completed);
                                return true;
                            }
                        }
                        false
                    });
                    if completed {
                        return RecursionAction::Stop;
                    }

                    context.lock_mut(|mut lock| {
                        if let Some((next_instant, _)) = lock.values.front() {
                            // Continue
                            RecursionAction::ContinueAt(*next_instant)
                        } else {
                            // No more values. Stop timer. Set timer to None.
                            if let Some(timer) = lock.timer.take() {
                                drop(lock);
                                timer.dispose();
                            }
                            RecursionAction::Stop
                        }
                    })
                },
                Some(self.delay),
            )));
        });
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.emit_value_and_setup_timer_if_needed(Some(value));
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.emit_value_and_setup_timer_if_needed(None);
            }
            Termination::Error(_) => {
                self.context.dispose();
                safe_lock_option_observer!(on_termination: self.observer, termination);
            }
        }
    }
}
