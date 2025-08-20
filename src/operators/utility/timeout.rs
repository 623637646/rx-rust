use crate::{
    disposable::{Disposable, boxed_disposal::BoxedDisposal, subscription::Subscription},
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock_option, safe_lock_option_disposable, safe_lock_option_observer,
    scheduler::Scheduler,
    utils::{
        types::{MutGuard, Mutable, MutableHelper, NecessarySend, Shared},
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
            let context = Shared::new(Mutable::new(TimeoutContext {
                timer: None,
                version: 0,
            }));
            let observer = TimeoutObserver {
                observer: Shared::new(Mutable::new(Some(observer))),
                duration: self.duration,
                scheduler: self.scheduler,
                context: context.clone(),
            };
            observer.schedule_timer(None);
            self.source.subscribe(observer) + context
        })
    }
}

struct TimeoutContext {
    timer: Option<BoxedDisposal<'static>>, // None means disposed
    version: usize,
}

impl Disposable for Shared<Mutable<TimeoutContext>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, timer);
    }
}

struct TimeoutObserver<OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    duration: Duration,
    scheduler: S,
    context: Shared<Mutable<TimeoutContext>>,
}

impl<OR, S> TimeoutObserver<OR, S> {
    fn schedule_timer<T, E>(&self, lock: Option<MutGuard<'_, TimeoutContext>>)
    where
        OR: Observer<T, Error<E>> + NecessarySend + 'static,
        S: Scheduler,
    {
        let implementation = |mut lock: MutGuard<'_, TimeoutContext>| {
            let observer = self.observer.clone();
            let context = self.context.clone();
            let version = lock.version;
            let timer = BoxedDisposal::new(self.scheduler.schedule(
                move || {
                    context.lock_mut(|mut lock| {
                        if lock.version == version {
                            // Same version, should do timeout.
                            let current_timer = lock.timer.take(); // Take the current timer to mark it as disposed.
                            drop(lock);
                            safe_lock_option_observer!(on_termination: observer, Termination::Error(Error::Timeout));
                            if let Some(current_timer) = current_timer {
                                current_timer.dispose(); // Dispose the old timer as soon as possible to make the `EntryExitChecker` correct.
                            }
                        } else {
                            // New version, should ignore.
                        }
                    });
                },
                Some(self.duration),
            ));
            if let Some(old_timer) = lock.timer.replace(timer) {
                drop(lock);
                old_timer.dispose();
            }
        };
        if let Some(lock) = lock {
            implementation(lock);
        } else {
            self.context.lock_mut(implementation);
        }
    }
}

impl<T, E, OR, S> Observer<T, E> for TimeoutObserver<OR, S>
where
    OR: Observer<T, Error<E>> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let do_on_next = self.context.lock_mut(|mut lock| {
            if lock.timer.is_none() {
                // Already disposed
                false
            } else {
                lock.version += 1;
                self.schedule_timer(Some(lock));
                true
            }
        });
        if do_on_next {
            safe_lock_option_observer!(on_next: self.observer, value);
        }
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
