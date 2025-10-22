use crate::{
    disposable::{
        Disposable, boxed_disposal::BoxedDisposal, callback_disposal::CallbackDisposal,
        subscription::Subscription,
    },
    observable::Observable,
    observer::{Observer, Termination},
    safe_lock, safe_lock_option_observer,
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

/// Mirrors the source Observable, but issues an error if a specified duration elapses between emissions.
/// See <https://reactivex.io/documentation/operators/timeout.html>
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {
/// #     panic!("Use tokio-scheduler feature to run tests.");
/// # }
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::observable_ext::ObservableExt,
///         observer::{Observer, Termination},
///         operators::utility::timeout::{Error, Timeout},
///         subject::publish_subject::PublishSubject,
///     };
///     use std::{convert::Infallible, sync::{Arc, Mutex}};
///     use tokio::time::{sleep, Duration};
///
///     let handle = tokio::runtime::Handle::current();
///     let values = Arc::new(Mutex::new(Vec::new()));
///     let terminations = Arc::new(Mutex::new(Vec::new()));
///     let values_observer = Arc::clone(&values);
///     let terminations_observer = Arc::clone(&terminations);
///     let mut subject: PublishSubject<'static, i32, Infallible> = PublishSubject::default();
///
///     let subscription = Timeout::new(subject.clone(), Duration::from_millis(5), handle.clone())
///         .subscribe_with_callback(
///             move |value| values_observer.lock().unwrap().push(value),
///             move |termination| terminations_observer
///                 .lock()
///                 .unwrap()
///                 .push(termination),
///         );
///
///     subject.on_next(1);
///     sleep(Duration::from_millis(10)).await;
///     drop(subscription);
///
///     assert_eq!(&*values.lock().unwrap(), &[1]);
///     assert_eq!(
///         &*terminations.lock().unwrap(),
///         &[Termination::Error(Error::Timeout)]
///     );
/// }
/// ```
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
                timer_state: TimerState::Initialized,
                version: 0,
            }));
            let observer = Shared::new(Mutable::new(Some(observer)));
            let timeout_observer = TimeoutObserver {
                observer: observer.clone(),
                duration: self.duration,
                scheduler: self.scheduler.clone(),
                context: context.clone(),
            };

            let sub = self.source.subscribe(timeout_observer);
            let timer = create_timer(
                0,
                observer.clone(),
                self.duration,
                self.scheduler.clone(),
                context.clone(),
            );
            let timer_state =
                safe_lock!(mem_replace: context, timer_state, TimerState::Scheduled(timer));
            match timer_state {
                TimerState::Initialized => {} // Normal case.
                TimerState::Scheduled(_) => unreachable!(),
                TimerState::DidTimeout => {} // Scheduled task is too fast.
                TimerState::Disposed => unreachable!(),
            }
            sub + context
        })
    }
}

enum TimerState {
    Initialized,
    Scheduled(BoxedDisposal<'static>),
    DidTimeout,
    Disposed,
}

struct TimeoutContext {
    timer_state: TimerState,
    version: usize,
}

impl Disposable for Shared<Mutable<TimeoutContext>> {
    fn dispose(self) {
        let timer_state = safe_lock!(mem_replace: self, timer_state, TimerState::Disposed);
        match timer_state {
            TimerState::Initialized => unreachable!(),
            TimerState::Scheduled(disposal) => disposal.dispose(), // Not timeout yet.
            TimerState::DidTimeout => {}                           // Timeout
            TimerState::Disposed => unreachable!(),
        }
    }
}

struct TimeoutObserver<OR, S> {
    observer: Shared<Mutable<Option<OR>>>,
    duration: Duration,
    scheduler: S,
    context: Shared<Mutable<TimeoutContext>>,
}

impl<T, E, OR, S> Observer<T, E> for TimeoutObserver<OR, S>
where
    OR: Observer<T, Error<E>> + NecessarySend + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        self.context
            .lock_mut(|mut lock| match &mut lock.timer_state {
                TimerState::Initialized => {
                    drop(lock);
                    safe_lock_option_observer!(on_next: self.observer, value);
                }
                TimerState::Scheduled(disposal) => {
                    // dispose old timer
                    let disposal = std::mem::replace(
                        disposal,
                        BoxedDisposal::new(CallbackDisposal::new(|| {})), // Plcaceholder
                    );
                    disposal.dispose();

                    // schedule new timer
                    lock.version += 1;
                    let timer = create_timer(
                        lock.version,
                        self.observer.clone(),
                        self.duration,
                        self.scheduler.clone(),
                        self.context.clone(),
                    );
                    lock.timer_state = TimerState::Scheduled(timer);

                    // emit
                    drop(lock);
                    safe_lock_option_observer!(on_next: self.observer, value);
                }
                TimerState::DidTimeout => {}
                TimerState::Disposed => {}
            });
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

fn create_timer<T, E, OR, S>(
    version: usize,
    observer: Shared<Mutable<Option<OR>>>,
    duration: Duration,
    scheduler: S,
    context: Shared<Mutable<TimeoutContext>>,
) -> BoxedDisposal<'static>
where
    OR: Observer<T, Error<E>> + NecessarySend + 'static,
    S: Scheduler,
{
    BoxedDisposal::new(scheduler.schedule(
        move || {
            context.lock_mut(|mut lock| {
                if lock.version != version {
                    // New version, should ignore.
                    return;
                }
                // Same version, should do timeout.
                let timer_state = std::mem::replace(&mut lock.timer_state, TimerState::DidTimeout);
                drop(lock);
                safe_lock_option_observer!(on_termination: observer, Termination::Error(Error::Timeout));
                match timer_state {
                    TimerState::Initialized => {} //  Scheduled task is too fast.
                    TimerState::Scheduled(disposal) => disposal.dispose(), // Dispose the old timer as soon as possible to make the `EntryExitChecker` correct.
                    TimerState::DidTimeout => unreachable!(),
                    TimerState::Disposed => {},
                }
            });
        },
        Some(duration),
    ))
}
