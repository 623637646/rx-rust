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
            observer: Some(observer),
            values: VecDeque::new(),
            timer: None,
        }));
        let delay_observer = DelayObserver {
            delay: self.delay,
            scheduler: self.scheduler,
            context: context.clone(),
        };
        self.source.subscribe(delay_observer) + context
    }
}

struct DelayContext<T, OR> {
    observer: Option<OR>,                   // None means terminated or disposed
    values: VecDeque<(Instant, Option<T>)>, // None means completed
    timer: Option<BoxedDisposal<'static>>,
}

impl<T, OR> Disposable for Shared<Mutable<DelayContext<T, OR>>> {
    fn dispose(self) {
        let mut lock = self.lock_mut();
        lock.observer.take();
        if let Some(timer) = lock.timer.take() {
            timer.dispose();
        }
    }
}

struct DelayObserver<T, OR, S> {
    delay: Duration,
    scheduler: S,
    context: Shared<Mutable<DelayContext<T, OR>>>,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let mut lock = self.context.lock_mut();
        lock.values.push_back((Instant::now(), Some(value)));
        if lock.timer.is_none() {
            setup_emit_timer(
                &mut lock.timer,
                self.delay,
                self.scheduler.clone(),
                self.context.clone(),
            );
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        let mut lock = self.context.lock_mut();
        match termination {
            Termination::Completed => {
                lock.values.push_back((Instant::now(), None));
                if lock.timer.is_none() {
                    setup_emit_timer(
                        &mut lock.timer,
                        self.delay,
                        self.scheduler.clone(),
                        self.context.clone(),
                    );
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = lock.observer.take() {
                    observer.on_termination(termination);
                }
                if let Some(timer) = lock.timer.take() {
                    timer.dispose();
                }
            }
        }
    }
}

fn setup_emit_timer<T, E, OR>(
    timer: &mut Option<BoxedDisposal<'static>>,
    delay: Duration,
    scheduler: impl Scheduler,
    context: Shared<Mutable<DelayContext<T, OR>>>,
) where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
{
    *timer = Some(BoxedDisposal::new(scheduler.schedule_recursively(
        move |_| {
            let mut lock = context.lock_mut();
            if let Some((instant, value)) = lock.values.pop_front() {
                if let Some(value) = value {
                    //  Next
                    if let Some(observer) = lock.observer.as_mut() {
                        observer.on_next(value);
                        if let Some((next_instant, _)) = lock.values.front() {
                            // Continue
                            let delay = next_instant.duration_since(instant);
                            RecursionAction::ContinueAfterRevisedDelay(delay)
                        } else {
                            // No more values. Stop timer. Set timer to None.
                            lock.timer.take().unwrap().dispose();
                            RecursionAction::Stop
                        }
                    } else {
                        // Already terminated
                        RecursionAction::Stop
                    }
                } else {
                    // Completed
                    if let Some(observer) = lock.observer.take() {
                        drop(lock);
                        observer.on_termination(Termination::Completed);
                    }
                    RecursionAction::Stop
                }
            } else {
                unreachable!()
            }
        },
        Some(delay),
    )));
}
