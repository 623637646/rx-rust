use crate::{
    disposable::{Disposable, boxed_disposal::BoxedDisposal, subscription::Subscription},
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock_option, safe_lock_option_observer,
    scheduler::Scheduler,
    utils::{
        types::{Mutable, MutableHelper, NecessarySend, Shared},
        unsub_after_termination::subscribe_unsub_after_termination,
    },
};
use educe::Educe;
use std::time::Duration;

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
pub enum Error<E> {
    Timeout,
    SourceError(E),
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Timeout<OE, S> {
    source: OE,
    duration: Duration,
    scheduler: S,
}

impl<OE, S> Timeout<OE, S> {
    pub fn new(source: OE, duration: Duration, scheduler: S) -> Self {
        Self {
            source,
            duration,
            scheduler,
        }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, Error<E>> for Timeout<OE, S>
where
    OE: Observable<'or, 'static, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, Error<E>> + NecessarySend + 'static,
    ) -> Subscription<'static> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let timer = Shared::new(Mutable::new(Some(schedule_timer(
                &self.scheduler,
                observer.clone(),
                self.duration,
            ))));
            let observer = TimeoutObserver {
                observer,
                duration: self.duration,
                scheduler: self.scheduler,
                timer: timer.clone(),
            };
            self.source.subscribe(observer) + timer
        })
    }
}

fn schedule_timer<T, E, OR, S>(
    scheduler: &S,
    observer: Shared<Mutable<Option<OR>>>,
    duration: Duration,
) -> BoxedDisposal<'static>
where
    OR: Observer<T, Error<E>> + NecessarySend + 'static,
    S: Scheduler,
{
    BoxedDisposal::new(scheduler.schedule(
        move || {
            safe_lock_option_observer!(on_termination: observer, Termination::Error(Error::Timeout));
        },
        Some(duration),
    ))
}

struct TimeoutObserver<OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    duration: Duration,
    scheduler: S,
    timer: Shared<Mutable<Option<BoxedDisposal<'static>>>>, // None means disposed
}

impl<T, E, OR, S> Observer<T, E> for TimeoutObserver<OR, S>
where
    OR: Observer<T, Error<E>> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        // TODO: think about this lock.
        self.timer.lock_mut(|mut lock| {
            if lock.is_none() {
                // Already disposed
                return;
            }
            let timer = schedule_timer(&self.scheduler, self.observer.clone(), self.duration);
            let old_timer = lock.replace(timer).unwrap();
            drop(lock);
            old_timer.dispose();
        });
        safe_lock_option_observer!(on_next: self.observer, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                safe_lock_option_observer!(on_termination: self.observer, Termination::Completed);
            }
            Termination::Error(error) => {
                safe_lock_option_observer!(on_termination: self.observer, Termination::Error(Error::SourceError(error)));
            }
        }
    }
}
