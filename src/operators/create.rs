use crate::{cancellable::Cancellable, observable::Observable, observer::Observer};

/// Create an Observable from scratch by calling observer methods programmatically
/// Example:
/// ```rust
/// use rx_rust::operators::create::Create;
/// use rx_rust::observer::Observer;
/// use rx_rust::observer::Event;
/// use rx_rust::observer::Terminated;
/// use rx_rust::cancellable::non_cancellable::NonCancellable;
/// use rx_rust::observable::observable_ext::ObservableExt;
/// let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
///     observer.on(Event::Next(1));
///     observer.on(Event::Next(2));
///     observer.on(Event::Next(3));
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

impl<'a, T, E, D, F> Observable<'a, T, E> for Create<F>
where
    D: Cancellable,
    F: Fn(Box<dyn Observer<T, E>>) -> D,
{
    fn subscribe<O>(&'a self, observer: O) -> impl Cancellable
    where
        O: Observer<T, E> + 'static,
    {
        (self.subscribe_handler)(Box::new(observer))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cancellable::non_cancellable::NonCancellable,
        observable::Observable,
        observer::{Event, Observer, Terminated},
        operators::create::Create,
        utils::test_helper::ObservableChecker,
    };

    #[test]
    fn test_normal() {
        let value = 123;
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(value));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });

        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[value]));
        assert!(checker.is_terminals_matched(&Terminated::Completed));
    }

    #[test]
    fn test_multiple_subscribe() {
        let value = 123;
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(value));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        });

        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[value]));
        assert!(checker.is_terminals_matched(&Terminated::Completed));

        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[value]));
        assert!(checker.is_terminals_matched(&Terminated::Completed));
    }
}
