use crate::{cancellable::Cancellable, observable::Observable, observer::Observer};

/// Create an Observable from scratch by calling observer methods programmatically
/// Example:
/// ```rust
/// use rx_rust::cancellable::non_cancellable::NonCancellable;
/// use rx_rust::observable::observable_subscribe_ext::ObservableSubscribeExt;
/// use rx_rust::observer::Event;
/// use rx_rust::observer::Observer;
/// use rx_rust::observer::Terminated;
/// use rx_rust::operators::create::Create;
/// let observable = Create::new(|observer: Box<dyn for<'a> Observer<&'a i32, String>>| {
///     observer.on(Event::Next(&1));
///     observer.on(Event::Next(&2));
///     observer.on(Event::Next(&3));
///     observer.on(Event::Terminated(Terminated::Completed));
///     NonCancellable
/// });
/// observable.subscribe_on_event(|v| println!("event: {:?}", v));
/// ```
pub struct Create<F> {
    subscribe_handler: F,
}

impl<F> Create<F> {
    pub fn new(subscribe_handler: F) -> Create<F> {
        Create { subscribe_handler }
    }
}

impl<T, E, C, F> Observable<T, E> for Create<F>
where
    C: Cancellable + 'static,
    F: Fn(Box<dyn for<'a> Observer<&'a T, E>>) -> C,
{
    fn subscribe(
        &self,
        observer: impl for<'a> Observer<&'a T, E> + 'static,
    ) -> impl Cancellable + 'static {
        (self.subscribe_handler)(Box::new(observer))
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{
//         cancellable::non_cancellable::NonCancellable,
//         observer::{Event, Observer, Terminated},
//         operators::create::Create,
//         utils::test_helper::ObservableChecker,
//     };

//     #[test]
//     fn test_normal() {
//         let value = 123;
//         let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
//             observer.on(Event::Next(value));
//             observer.on(Event::Terminated(Terminated::Completed));
//             NonCancellable
//         });

//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[&value]));
//         assert!(checker.is_completed());
//     }

//     #[test]
//     fn test_multiple_subscribe() {
//         let value = 123;
//         let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
//             observer.on(Event::Next(value));
//             observer.on(Event::Terminated(Terminated::Completed));
//             NonCancellable
//         });

//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[&value]));
//         assert!(checker.is_completed());

//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[&value]));
//         assert!(checker.is_completed());
//     }
// }
