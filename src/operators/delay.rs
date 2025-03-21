use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscription::{disposable::CallbackDisposal, Subscription},
};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
    time::Duration,
};

/// An observable that delays the next value and completed events from the source observable by a duration.
/// The error will be emitted immediately.
#[derive(Clone)]
pub struct Delay<OE, S, OR> {
    source_observable: OE,
    delay: Duration,
    scheduler: S,
    // Adding this to avoid the compiler error: `type annotations needed. multiple `impl`s satisfying `_: observer::Observer<*, *>` found`
    _marker: PhantomData<OR>,
}

impl<OE, S, OR> Delay<OE, S, OR> {
    /// Creates a new `Delay` observable.
    ///
    /// # Arguments
    ///
    /// * `source` - The source observable to delay.
    /// * `delay` - The duration to delay each emission.
    /// * `scheduler` - The scheduler to use for timing the delay.
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Delay<OE, S, OR> {
        Delay {
            source_observable: source,
            delay,
            scheduler,
            _marker: PhantomData,
        }
    }
}

impl<T, E, OE, OR, S> Observable<T, E, OR> for Delay<OE, S, OR>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    OE: Observable<T, E, DelayObserver<OR, S>>,
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
    //             let mut subscription = $builder(self, source_observer.clone());
    //             let disposal = $crate::subscription::disposable::CallbackDisposal::new(move || {
    //                 let mut source_observer = source_observer.lock().unwrap();
    //                 source_observer.take();
    //             });
    //             subscription.append_disposable(disposal);
    //             subscription
    //         }
    //     };
    // }
    // ```
    fn subscribe(self, observer: OR) -> Subscription {
        let source_observer = Arc::new(Mutex::new(Some(observer)));
        let delay_observer = DelayObserver {
            source_observer: source_observer.clone(),
            delay: self.delay,
            scheduler: self.scheduler,
        };
        let disposal = CallbackDisposal::new(move || {
            let mut source_observer = source_observer.lock().unwrap();
            source_observer.take();
        });
        let mut subscription = self.source_observable.subscribe(delay_observer);
        subscription.append_disposable(disposal);
        subscription
    }
}

pub struct DelayObserver<OR, S> {
    source_observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: S,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<OR, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let observer = self.source_observer.clone();
        _ = self.scheduler.schedule(
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
                _ = self.scheduler.schedule(
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
pub trait DelayableObservable<T, E, OR, S>: Sized {
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
    /// use rx_rust::operators::just::Just;
    /// use rx_rust::operators::delay::DelayableObservable;
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
    fn delay(self, delay: Duration, scheduler: S) -> Delay<Self, S, OR>;
}

impl<T, E, OR, S, OE> DelayableObservable<T, E, OR, S> for OE
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    OE: Observable<T, E, DelayObserver<OR, S>>,
    S: Scheduler,
{
    fn delay(self, delay: Duration, scheduler: S) -> Delay<Self, S, OR> {
        Delay::new(self, delay, scheduler)
    }
}

#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_subscribe_ext::ObservableSubscribeExt,
        operators::{create::Create, just::Just},
        scheduler::tokio_scheduler::TokioScheduler,
        utils::checking_observer::CheckingObserver,
    };
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer.on_next(3);
                observer.on_terminal(Terminal::Error("error".to_string()));
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_error("error".to_owned()));
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_error("error".to_owned()));
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(3);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2, 3]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_subscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);

        let checker1 = CheckingObserver::new();
        let subscription1 = observable.clone().subscribe(checker1.clone());
        let checker2 = CheckingObserver::new();
        let subscription2 = observable.clone().subscribe(checker2.clone());

        assert!(checker1.is_values_matched(&[]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker1.is_values_matched(&[]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_completed());
        _ = subscription1; // keep the subscription alive
        _ = subscription2; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_multiple_operate() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        subscription.unsubscribe(); // unsubscribe
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_async_unsubscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(5)).await;
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        tokio::spawn(async move {
            subscription.unsubscribe(); // unsubscribe
        });
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        sleep(Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
    }

    /// If we remove `OR`` in `struct Delay`, the code will not compile.
    /// This test is to make sure that the code compiles without any errors.
    #[tokio::test]
    async fn test_no_compiler_error() {
        let observable = Just::new(333);
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        observable.subscribe_on(
            |value| {
                println!("Next value: {}", value);
            },
            |terminal| {
                println!("Terminal event: {:?}", terminal);
            },
        );
    }
}
