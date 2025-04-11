use super::Observable;
use crate::observer::Observer;

/// The `ObservableIntoExt` trait provides a convenient method to convert any type that implements
/// the `Observable` trait into an `impl Observable<T, E, OR>`. This allows for more flexible and
/// ergonomic usage of observables in the code.
///
/// # Type Parameters
///
/// * `T` - The type of the items emitted by the observable.
/// * `E` - The type of the error that can be emitted by the observable.
/// * `OR` - The type of the observer that will receive events from the observable. It must implement the `Observer` trait.
pub trait ObservableIntoExt<'a, T, E, OR>
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
    ///     operators::creating::just::Just,
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
    fn into_observable(self) -> impl Observable<'a, T, E, OR>;
}

impl<'a, T, E, OR, OE> ObservableIntoExt<'a, T, E, OR> for OE
where
    OR: Observer<T, E>,
    OE: Observable<'a, T, E, OR>,
{
    fn into_observable(self) -> impl Observable<'a, T, E, OR> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use crate::observer::{Observer, Terminal};
    use crate::operators::creating::just::Just;
    use crate::subject::publish_subject::PublishSubject;
    use crate::utils::tests_utils::checking_observer::CheckingObserver;

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.into_observable();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.into_observable();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    // #[test]
    // fn test_unsubscribe() {
    //     let mut subject = PublishSubject::default();
    //     let checker_1 = CheckingObserver::new();
    //     let checker_2 = CheckingObserver::new();

    //     // Custom operations
    //     let observable = subject.clone();
    //     let observable_1 = observable.into_observable();
    //     let observable_2 = observable_1.clone();

    //     let subscription_1 = observable_1.subscribe(checker_1.clone());
    //     let subscription_2 = observable_2.subscribe(checker_2.clone());
    //     assert!(checker_1.is_values_matched(&[]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[]));
    //     assert!(checker_2.is_unterminated());

    //     subject.on_next(111);
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[111]));
    //     assert!(checker_2.is_unterminated());

    //     subscription_1.unsubscribe();
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[111]));
    //     assert!(checker_2.is_unterminated());

    //     subject.on_next(222);
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[111, 222]));
    //     assert!(checker_2.is_unterminated());

    //     subject.on_terminal(Terminal::Error("error"));
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[111, 222]));
    //     assert!(checker_2.is_error("error"));

    //     _ = subscription_2; // keep the subscription alive
    // }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.into_observable();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value = 111;
        let observable = Just::new(&mut value);
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = observable.into_observable();

        let mut checker_cloned_1 = checker.clone();
        let checker_cloned_2 = checker.clone();
        let subscription = observable.subscribe_on(
            |value| {
                checker_cloned_1.on_next(*value);
                *value *= 2;
            },
            |terminal| checker_cloned_2.on_terminal(terminal),
        );

        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        assert_eq!(value, 222);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.into_observable();

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

    // #[test]
    // fn test_subscribe_by_different_observer() {
    //     let mut subject = PublishSubject::default();
    //     let checker_1 = CheckingObserver::new();
    //     let checker_2 = CheckingObserver::new();

    //     // Custom operations
    //     let observable = subject.clone();
    //     let observable_1 = observable.into_observable();
    //     let observable_2 = observable_1.clone();

    //     let subscription_1 = observable_1.subscribe(checker_1.clone());

    //     let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
    //     let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);
    //     assert!(checker_1.is_values_matched(&[]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[]));
    //     assert!(checker_2.is_unterminated());

    //     subject.on_next(111);
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_unterminated());
    //     assert!(checker_2.is_values_matched(&[111]));
    //     assert!(checker_2.is_unterminated());

    //     subject.on_terminal(Terminal::Error("error"));
    //     assert!(checker_1.is_values_matched(&[111]));
    //     assert!(checker_1.is_error("error"));
    //     assert!(checker_2.is_values_matched(&[111]));
    //     assert!(checker_2.is_error("error"));

    //     _ = subscription_1; // keep the subscription alive
    //     _ = subscription_2; // keep the subscription alive
    // }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .into_observable()
            .into_observable()
            .into_observable();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }
}
