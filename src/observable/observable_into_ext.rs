use super::Observable;
use crate::observer::Observer;

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
    ///     move |value| {
    ///         println!("value: {}", value);
    ///     },
    ///     move |terminal| {
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

    #[test]
    fn test_normal() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_empty()
        });
        let observable = observable.into_observable();
        let checker = CheckingObserver::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[333]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_multiple() {
        let observable = Create::new(|mut observer| {
            observer.on_next(333);
            observer.on_terminal(Terminal::<String>::Completed);
            Subscription::new_empty()
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
