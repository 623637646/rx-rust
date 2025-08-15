use crate::{
    disposable::{Disposable, boxed_disposal::BoxedDisposal, subscription::Subscription},
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock_option_disposable, safe_lock_option_observer,
    scheduler::{RecursionAction, Scheduler},
    utils::types::{MutGuard, Mutable, MutableHelper, NecessarySend, Shared},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct ObserveOn<OE, S> {
    source: OE,
    scheduler: S,
}

impl<OE, S> ObserveOn<OE, S> {
    pub fn new(source: OE, scheduler: S) -> Self {
        Self { source, scheduler }
    }
}

impl<'or, 'sub, T, E, OE, S> Observable<'static, 'sub, T, E> for ObserveOn<OE, S>
where
    T: NecessarySend + 'static,
    E: NecessarySend + 'static,
    OE: Observable<'or, 'sub, T, E>,
    S: Scheduler,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySend + 'static,
    ) -> Subscription<'sub> {
        let context = Shared::new(Mutable::new(ObserveOnContext {
            values: Vec::new(),
            termination: None,
            disposal: None,
        }));
        let observer = ObserveOnObserver {
            context: context.clone(),
            observer: Shared::new(Mutable::new(Some(observer))),
            scheduler: self.scheduler,
        };
        self.source.subscribe(observer) + context
    }
}

struct ObserveOnContext<T, E> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
    disposal: Option<BoxedDisposal<'static>>,
}

impl<T, E> Disposable for Shared<Mutable<ObserveOnContext<T, E>>> {
    fn dispose(self) {
        safe_lock_option_disposable!(dispose: self, disposal);
    }
}

struct ObserveOnObserver<T, E, OR, S> {
    context: Shared<Mutable<ObserveOnContext<T, E>>>,
    observer: Shared<Mutable<Option<OR>>>,
    scheduler: S,
}

impl<T, E, OR, S> ObserveOnObserver<T, E, OR, S> {
    fn setup_scheduler_if_needed(&self, mut lock: MutGuard<'_, ObserveOnContext<T, E>>)
    where
        T: NecessarySend + 'static,
        E: NecessarySend + 'static,
        OR: Observer<T, E> + NecessarySend + 'static,
        S: Scheduler,
    {
        if lock.disposal.is_some() {
            return;
        }
        let context = self.context.clone();
        let observer = self.observer.clone();
        lock.disposal
            .replace(BoxedDisposal::new(self.scheduler.schedule_recursively(
                move |_| {
                    context.lock_mut(|mut lock| {
                        let termination = lock.termination.take();
                        let values = std::mem::take(&mut lock.values);

                        match (termination, values.is_empty()){
                            (None, true) => {
                                lock.disposal = None; // No more values. Stop scheduler. Set disposal to None.
                                RecursionAction::Stop
                            },
                            (None, false) => {
                                drop(lock);
                                safe_lock_option_observer!(on_next: observer, values: values);
                                RecursionAction::ContinueImmediately
                            },
                            (Some(termination), true) => {
                                drop(lock);
                                safe_lock_option_observer!(on_termination: observer, termination);
                                RecursionAction::Stop
                            },
                            (Some(termination), false) => {
                                drop(lock);
                                match termination {
                                    Termination::Completed => {
                                        safe_lock_option_observer!(on_next_and_termination: observer, values: values, Termination::Completed);
                                    }
                                    Termination::Error(_) => {
                                        safe_lock_option_observer!(on_termination: observer, termination);
                                    }
                                }
                                RecursionAction::Stop
                            },
                        }
                    })
                },
                None,
            )));
    }
}

impl<T, E, OR, S> Observer<T, E> for ObserveOnObserver<T, E, OR, S>
where
    T: NecessarySend + 'static,
    E: NecessarySend + 'static,
    OR: Observer<T, E> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.context.lock_mut(|mut lock| {
            lock.values.push(value);
            self.setup_scheduler_if_needed(lock);
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.context.lock_mut(|mut lock| {
            lock.termination.replace(termination);
            self.setup_scheduler_if_needed(lock);
        });
    }
}
