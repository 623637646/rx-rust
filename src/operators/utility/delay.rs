use crate::disposable::auto_disposal::AutoDisposal;
use crate::disposable::shared_disposal::SharedDisposal;
use crate::disposable::subscription::Subscription;
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
    S: Scheduler + NecessarySend + 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let timer = Shared::new(Mutable::new(None));
        let delay_observer = DelayObserver {
            observer: Shared::new(Mutable::new(Some(observer))),
            delay: self.delay,
            scheduler: self.scheduler,
            values: Shared::new(Mutable::new(VecDeque::new())),
            timer: timer.clone(),
        };
        self.source.subscribe(delay_observer) + SharedDisposal::new(timer)
    }
}

type DelayObserverValues<T> = Shared<Mutable<VecDeque<(Instant, Option<T>)>>>; // None means completed

struct DelayObserver<T, OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    delay: Duration,
    scheduler: S,
    values: DelayObserverValues<T>,
    timer: Shared<Mutable<Option<AutoDisposal<'static>>>>,
}

impl<T, OR, S> DelayObserver<T, OR, S> {
    fn setup_emit_timer_if_needed<E>(&mut self)
    where
        T: NecessarySend + 'static,
        OR: Observer<T, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        if self.timer.lock_ref().is_some() {
            return;
        }
        let values = self.values.clone();
        let observer = self.observer.clone();
        let timer = self.timer.clone();
        *self.timer.lock_mut() = Some(self.scheduler.schedule_recursive(
            move |_| {
                if let Some((instant, value)) = { values.lock_mut().pop_front() } {
                    if let Some(value) = value {
                        //  next
                        if let Some(observer) = observer.lock_mut().as_mut() {
                            observer.on_next(value);
                            if let Some((next_instant, _)) = values.lock_ref().front() {
                                let delay = next_instant.duration_since(instant);
                                Some(delay)
                            } else {
                                timer.lock_mut().take().unwrap();
                                None
                            }
                        } else {
                            timer.lock_mut().take().unwrap();
                            None
                        }
                    } else {
                        // completed
                        if let Some(observer) = { observer.lock_mut().take() } {
                            observer.on_termination(Termination::Completed);
                        }
                        timer.lock_mut().take().unwrap();
                        None
                    }
                } else {
                    unreachable!()
                }
            },
            Some(self.delay),
        ));
    }
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<T, OR, S>
where
    T: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.values
            .lock_mut()
            .push_back((Instant::now(), Some(value)));
        self.setup_emit_timer_if_needed();
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                self.values.lock_mut().push_back((Instant::now(), None));
                self.setup_emit_timer_if_needed();
            }
            Termination::Error(_) => {
                if let Some(observer) = { self.observer.lock_mut().take() } {
                    observer.on_termination(termination);
                }
            }
        }
    }
}
