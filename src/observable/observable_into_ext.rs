use super::Observable;
use crate::observer::Observer;

// TODO: Do we really need this trait?
/// The `ObservableIntoExt` trait provides a convenient method to convert any type that implements
/// the `Observable` trait into an `impl Observable<T, E, OR>`. This allows for more flexible and
/// ergonomic usage of observables in the code.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `OR` - The type of the observer that will receive events from the observable. It must implement the `Observer` trait.
pub trait ObservableIntoExt<T, E, OR>
where
    OR: Observer<T, E>,
{
    /// Converts any type that implements `Observable` into `impl Observable<T, E, OR>`.
    ///
    /// # Example
    /// ```rust
    /// use rx_rust::{
    ///     observable::{
    ///         observable_into_ext::ObservableIntoExt,
    ///         observable_subscribe_ext::ObservableSubscribeExt,
    ///     },
    ///     operators::just::Just,
    /// };
    /// let observable = Just::new(123);
    /// let observable = observable.into_observable();
    /// observable.subscribe_on(
    ///     |value| {
    ///         println!("value: {}", value);
    ///     },
    ///     |terminal| {
    ///         println!("terminal: {:?}", terminal);
    ///     },
    /// );
    /// ```
    fn into_observable(self) -> impl Observable<T, E, OR>;
}

impl<T, E, OR, OE> ObservableIntoExt<T, E, OR> for OE
where
    OR: Observer<T, E>,
    OE: Observable<T, E, OR>,
{
    fn into_observable(self) -> impl Observable<T, E, OR> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::{Observer, Terminal};
    use crate::operators::create::Create;
    use crate::subscription::Subscription;
    use crate::utils::checking_observer::CheckingObserver;
    use std::time::Duration;
    use tokio::time::sleep;

    #[test]
    fn test_completed() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_error() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Error("error".to_string()));
            Subscription::new_none_disposal()
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_error("error".to_string()));
    }

    #[test]
    fn test_unterminated() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            Subscription::new_none_disposal()
        });
        let observable = observable.into_observable();
        let checker: CheckingObserver<i32, String> = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_unterminated());
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let observable = Create::new(|mut observer| {
            observer.on_next(1);
            let handle = tokio::spawn(async {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_next(2);
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                observer.on_terminal(Terminal::<String>::Completed);
            });
            Subscription::new_with_disposal_callback(move || handle.abort())
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        let subscription = observable.subscribe(checker.clone());
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

    #[test]
    fn test_multiple() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_none_disposal()
        });
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }
}
