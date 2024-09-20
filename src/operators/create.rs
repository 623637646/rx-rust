use crate::{observable::Observable, observer::Observer, utils::disposal::Disposal};
use std::{marker::PhantomData, sync::Arc};

/**
This is an observable that emits the values provided by the subscribe_handler function.

# Example
```rust
use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
use rx_rust::observer::event::Event;
use rx_rust::observer::Observer;
use rx_rust::subscription::Subscription;
use rx_rust::observer::event::Terminated;
use rx_rust::operators::create::Create;
let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
    observer.notify_if_unterminated(Event::Next(1));
    observer.notify_if_unterminated(Event::Next(2));
    observer.notify_if_unterminated(Event::Next(3));
    observer.notify_if_unterminated(Event::Terminated(Terminated::Completed));
    Subscription::new_non_disposal_action(observer)
});
observable.subscribe_on_event(|event: Event<i32, String>| println!("event: {:?}", event));
```
*/
pub struct Create<T, E, O, F>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
    O: Sync + Send + 'static,
    F: Fn(Box<dyn Observer<T, E>>) -> Disposal + Sync + Send + 'static,
    O: Observer<T, E>,
{
    subscribe_handler: Arc<F>,
    _marker: PhantomData<(T, E, O)>,
}

impl<T, E, O, F> Create<T, E, O, F>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
    O: Sync + Send + 'static,
    F: Fn(Box<dyn Observer<T, E>>) -> Disposal + Sync + Send + 'static,
    O: Observer<T, E>,
{
    pub fn new(subscribe_handler: F) -> Create<T, E, O, F> {
        Create {
            subscribe_handler: Arc::new(subscribe_handler),
            _marker: PhantomData,
        }
    }
}

impl<T, E, O, F> Clone for Create<T, E, O, F>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
    O: Sync + Send + 'static,
    F: Fn(Box<dyn Observer<T, E>>) -> Disposal + Sync + Send + 'static,
    O: Observer<T, E>,
{
    fn clone(&self) -> Self {
        Create {
            subscribe_handler: self.subscribe_handler.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T, E, F, O> Observable<T, E, O> for Create<T, E, O, F>
where
    T: Sync + Send + 'static,
    E: Sync + Send + 'static,
    O: Sync + Send + 'static,
    F: Fn(Box<dyn Observer<T, E>>) -> Disposal + Sync + Send + 'static,
    O: Observer<T, E>,
{
    fn subscribe(self, observer: O) -> Disposal {
        (self.subscribe_handler)(Box::new(observer))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{observer::Terminal, utils::checking_observer::CheckingObserver};

//     #[test]
//     fn test_completed() {
//         let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
//             observer.on_next(333);
//             observer.on_terminal(Terminal::Completed);
//             Disposal::new_no_action()
//         });
//         let checker = CheckingObserver::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[333]));
//         assert!(checker.is_completed());
//     }

//     #[test]
//     fn test_error() {
//         let observable = Create::new(|mut observer: CheckingObserver<i32, &'static str>| {
//             observer.on_next(33);
//             observer.on_next(44);
//             observer.on_terminal(Terminal::Error("error"));
//             Disposal::new_no_action()
//         });

//         let checker = CheckingObserver::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[33, 44]));
//         assert!(checker.is_error("error"));
//     }

//     #[test]
//     fn test_unsubscribed() {
//         let checker: CheckingObserver<i32, String> = CheckingObserver::new();
//         {
//             let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
//                 observer.on_next(1);
//                 observer.on_next(2);
//                 Disposal::new_no_action()
//             });
//             observable.subscribe(checker.clone());
//         }
//         assert!(checker.is_values_matched(&[1, 2]));
//         assert!(checker.is_unterminated());
//     }

//     #[test]
//     fn test_unterminated() {
//         let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
//             observer.on_next(1);
//             Disposal::new_no_action()
//         });

//         let checker = CheckingObserver::new();
//         let subscription = observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[1]));
//         assert!(checker.is_unterminated());
//         _ = subscription; // keep the subscription alive
//     }

//     #[test]
//     fn test_multiple_subscribe() {
//         let observable = Create::new(|mut observer: CheckingObserver<i32, String>| {
//             observer.on_next(333);
//             observer.on_terminal(Terminal::Completed);
//             Disposal::new_no_action()
//         });

//         let checker = CheckingObserver::new();
//         observable.clone().subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[333]));
//         assert!(checker.is_completed());

//         let checker = CheckingObserver::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[333]));
//         assert!(checker.is_completed());
//     }

//     // TODO: test_async
// }
