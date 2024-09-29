use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    scheduler::Scheduler,
    subscriber::Subscriber,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// This is an observable that delays the next value and completed events from the source observable by a duration. The error will post immediately.
pub struct Delay<OE, S> {
    source: OE,
    delay: Duration,
    scheduler: Arc<S>,
}

impl<OE, S> Delay<OE, S> {
    pub fn new(source: OE, delay: Duration, scheduler: S) -> Delay<OE, S> {
        Delay {
            source,
            delay,
            scheduler: Arc::new(scheduler),
        }
    }
}

impl<OE, S> Clone for Delay<OE, S>
where
    OE: Clone,
{
    fn clone(&self) -> Self {
        Delay {
            source: self.source.clone(),
            delay: self.delay,
            scheduler: self.scheduler.clone(),
        }
    }
}

impl<T, E, OE, OR, S> Observable<T, E, OR> for Delay<OE, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    OE: Observable<T, E, DelayObserver<OR, S>>,
    S: Scheduler,
{
    fn subscribe(self, observer: OR) -> Subscriber {
        let internal_observer = DelayObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            delay: self.delay,
            scheduler: self.scheduler.clone(),
        };
        self.source.subscribe(internal_observer)
    }
}

pub struct DelayObserver<OR, S> {
    observer: Arc<Mutex<Option<OR>>>,
    delay: Duration,
    scheduler: Arc<S>,
}

impl<T, E, OR, S> Observer<T, E> for DelayObserver<OR, S>
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    S: Scheduler,
{
    fn on_next(&mut self, value: T) {
        let observer = self.observer.clone();
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
                        let observer = self.observer.lock().unwrap().take();
                        if let Some(observer) = observer {
                            observer.on_terminal(terminal);
                        }
                    },
                    Some(self.delay),
                );
            }
            Terminal::Error(_) => {
                let observer = self.observer.lock().unwrap().take();
                if let Some(observer) = observer {
                    observer.on_terminal(terminal);
                }
            }
        }
    }
}

/// Make the `Observable` delayable.
pub trait DelayableObservable<T, E, OR, S>
where
    OR: Observer<T, E>,
{
    /**
    Delay the next value and completed events from the source observable by a duration. The error will post immediately.

    # Example
    ```rust
    use rx_rust::operators::just::Just;
    use rx_rust::operators::delay::DelayableObservable;
    use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use rx_rust::scheduler::tokio_scheduler::TokioScheduler;
    use std::time::Duration;
    #[tokio::main]
    async fn main() {
        let observable = Just::new(333);
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        observable.subscribe_on(
            |value| {
                println!("Next value: {}", value);
            },
            |terminal| {
                println!("Terminal event: {:?}", terminal);
            }
        );
    }
    ```
     */
    fn delay(self, delay: Duration, scheduler: S) -> impl Observable<T, E, OR>;
}

impl<T, E, OR, S, OE> DelayableObservable<T, E, OR, S> for OE
where
    T: Send + 'static,
    E: Send + 'static,
    OR: Observer<T, E> + Send + 'static,
    OE: Observable<T, E, DelayObserver<OR, S>>,
    S: Scheduler,
{
    fn delay(self, delay: Duration, scheduler: S) -> impl Observable<T, E, OR> {
        Delay::new(self, delay, scheduler)
    }
}

#[cfg(feature = "tokio-scheduler")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operators::create::Create, scheduler::tokio_scheduler::TokioScheduler,
        utils::checking_observer::CheckingObserver,
    };
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscriber::new_empty()
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscriber = observable.subscribe(checker.clone());
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
        _ = subscriber; // keep the subscriber alive
    }

    #[tokio::test]
    async fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                observer.on_next(3);
                observer.on_terminal(Terminal::Error("error".to_string()));
            });
            Subscriber::new_empty()
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscriber = observable.subscribe(checker.clone());
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
        _ = subscriber; // keep the subscriber alive
    }

    #[tokio::test]
    async fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(3);
            });
            Subscriber::new_empty()
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        let subscriber = observable.subscribe(checker.clone());
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
        _ = subscriber; // keep the subscriber alive
    }

    #[tokio::test]
    async fn test_multiple_subscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscriber::new_empty()
        });
        let observable = observable.delay(Duration::from_millis(10), TokioScheduler);

        let checker1 = CheckingObserver::new();
        let subscriber1 = observable.clone().subscribe(checker1.clone());
        let checker2 = CheckingObserver::new();
        let subscriber2 = observable.clone().subscribe(checker2.clone());

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
        _ = subscriber1; // keep the subscriber alive
        _ = subscriber2; // keep the subscriber alive
    }

    #[tokio::test]
    async fn test_multiple_operate() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscriber::new_empty()
        });
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let observable = observable.delay(Duration::from_millis(5), TokioScheduler);
        let checker = CheckingObserver::new();
        let subscriber = observable.subscribe(checker.clone());
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
        _ = subscriber; // keep the subscriber alive
    }
}
