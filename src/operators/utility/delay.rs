use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    scheduler::Scheduler,
    subscription::{Subscription, disposable::CallbackDisposal},
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// An observable that delays the next value and completed events from the source observable by a duration.
/// The error will be emitted immediately.
#[derive(Clone)]
pub struct Delay<OE, S> {
    source_observable: OE,
    delay: Duration,
    scheduler: S,
}

impl<OE, S> Delay<OE, S> {
    /// Creates a new `Delay` observable.
    ///
    /// # Arguments
    ///
    /// * `source` - The source observable to delay.
    /// * `delay` - The duration to delay each emission.
    /// * `scheduler` - The scheduler to use for timing the delay.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Delay<OE, S> {
        Delay {
            source_observable: source,
            delay,
            scheduler,
        }
    }
}

impl<'a, T, E, OE, OR, S> Observable<'a, T, E, OR> for Delay<OE, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    OE: Observable<'a, T, E, DelayObserver<T, E, S>>,
    S: Scheduler,
{
    // TODO: Do we need to use macro to generate this?
    // ```text
    // use std::sync::{Arc, Mutex};
    // pub(crate) type ChainedSubscribeOR<OR> = Arc<Mutex<Option<OR>>>;
    // #[macro_export]
    // macro_rules! define_chained_subscribe {
    //     ($builder:expr) => {
    //         fn subscribe(self, observer: OR) -> $crate::subscription::Subscription {
    //             let source_observer = std::sync::Arc::new(std::sync::Mutex::new(Some(observer)));
    //             let subscription = $builder(self, source_observer.clone());
    //             let disposal = $crate::subscription::disposable::CallbackDisposal::new(move || {
    //                 let mut source_observer = source_observer.lock().unwrap();
    //                 source_observer.take();
    //             });
    //             subscription + disposal
    //         }
    //     };
    // }
    // ```
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let source_observer = Arc::new(Mutex::new(Some(BoxedObserver::new(observer))));
        let delay_observer = DelayObserver {
            source_observer: source_observer.clone(),
            delay: self.delay,
            scheduler: self.scheduler,
        };
        let disposal = CallbackDisposal::new(move || {
            let mut source_observer = source_observer.lock().unwrap();
            source_observer.take();
        });
        let subscription = self.source_observable.subscribe(delay_observer);
        subscription + disposal
    }
}

pub struct DelayObserver<T, E, S> {
    source_observer: Arc<Mutex<Option<BoxedObserver<'static, T, E>>>>,
    delay: Duration,
    scheduler: S,
}

impl<T, E, S> Observer<T, E> for DelayObserver<T, E, S>
where
    T: Send + 'static,
    E: Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let observer = self.source_observer.clone();
        self.scheduler.schedule(
            move || {
                let mut observer = observer.lock().unwrap();
                if let Some(observer) = &mut *observer {
                    observer.on_next(value)
                }
            },
            Some(self.delay),
        );
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match &terminal {
            Terminal::Completed => {
                self.scheduler.schedule(
                    move || {
                        let observer = self.source_observer.lock().unwrap().take();
                        if let Some(observer) = observer {
                            observer.on_terminal(terminal);
                        }
                    },
                    Some(self.delay),
                );
            }
            Terminal::Error(_) => {
                let observer = self.source_observer.lock().unwrap().take();
                if let Some(observer) = observer {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}

/// Extension trait to add the `delay` method to observables.
pub trait DelayableObservable<T, E, S>: Sized {
    /// Delays the next value and completed events from the source observable by a duration.
    /// The error will be emitted immediately.
    ///
    /// # Arguments
    ///
    /// * `delay` - The duration to delay each emission.
    /// * `scheduler` - The scheduler to use for timing the delay.
    ///
    /// # Returns
    ///
    /// A new observable that delays emissions from the source observable.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rx_rust::operators::creating::just::Just;
    /// use rx_rust::operators::utility::delay::DelayableObservable;
    /// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    /// use rx_rust::scheduler::tokio_scheduler::TokioScheduler;
    /// use std::time::Duration;
    /// #[tokio::main]
    /// async fn main() {
    ///     let observable = Just::new(333);
    ///     let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
    ///     observable.subscribe_on(
    ///         |value| {
    ///             println!("Next value: {}", value);
    ///         },
    ///         |terminal| {
    ///             println!("Terminal event: {:?}", terminal);
    ///         }
    ///     );
    /// }
    /// ```
    fn delay(self, delay: Duration, scheduler: S) -> Delay<Self, S>;
}

impl<'a, T, E, S, OE> DelayableObservable<T, E, S> for OE
where
    T: Send + 'static,
    E: Send + 'static,
    OE: Observable<'a, T, E, DelayObserver<T, E, S>>,
    S: Scheduler,
{
    fn delay(self, delay: Duration, scheduler: S) -> Delay<Self, S> {
        Delay::new(self, delay, scheduler)
    }
}

#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        operators::creating::create::Create,
        scheduler::tokio_scheduler::TokioScheduler,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };

    #[tokio::test]
    async fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333, 444]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();
        let checker_3 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        let subscription_3 = observable_3.subscribe(checker_3.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222]));
        assert!(checker_3.is_unterminated());

        subscription_2.unsubscribe();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222, 333]));
        assert!(checker_3.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(checker_3.is_values_matched(&[111, 222, 333]));
        assert!(checker_3.is_error("error"));

        _ = subscription_3; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), TokioScheduler);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .delay(Duration::from_millis(50), TokioScheduler)
            .delay(Duration::from_millis(50), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Delay::new(observable, Duration::from_millis(100), TokioScheduler);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        subject.on_next(333);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(checker.is_values_matched(&[111, 222, 333]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_lifetime() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = Delay::new(observable, Duration::from_millis(100), TokioScheduler);

            let checker = CheckingObserver::new();
            checker.is_values_matched(&[1]);
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }
}
