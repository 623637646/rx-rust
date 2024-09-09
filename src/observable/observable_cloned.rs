use super::observable_subscribe_ext::ObservableSubscribeExt;
use super::Observable;
use crate::cancellable::Cancellable;
use crate::observer::{Event, Observer, Terminated};

/// An `ObservableCloned` is a type that can be subscribed to by an `Observer`. The `Observer` will receive cloned values.
pub trait ObservableCloned<T, E> {
    /// Subscribes an observer to this observable. Returns a cancellable that can be used to cancel the subscription.
    /// The Observer must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    /// The Cancellable must be 'static because it may be stored by callers.
    fn subscribe_cloned(
        &self,
        observer: impl Observer<T, E> + 'static,
    ) -> impl Cancellable + 'static;
}

impl<T, E, O> ObservableCloned<T, E> for O
where
    O: Observable<T, E>,
    T: Clone,
{
    /// Subscribes an observer to this observable. Returns a cancellable that can be used to cancel the subscription.
    /// The Observer must be 'static because it will be stored in hot observables or pass to `subscribe_handler` of `Create`.
    /// The Cancellable must be 'static because it may be stored by callers.
    /// Example:
    /// ```rust
    /// use rx_rust::observable::observable_cloned::ObservableCloned;
    /// use rx_rust::observer::anonymous_observer::AnonymousObserver;
    /// use rx_rust::observer::Event;
    /// use rx_rust::operators::just::Just;
    /// use rx_rust::utils::never::Never;
    /// let observable = Just::new(123);
    /// let observer = AnonymousObserver::new(|e: Event<i32, Never>| {
    ///     println!("{:?}", e);
    /// });
    /// observable.subscribe_cloned(observer);
    /// ```
    fn subscribe_cloned(
        &self,
        observer: impl Observer<T, E> + 'static,
    ) -> impl Cancellable + 'static {
        self.subscribe_on_event(move |event| observer.on(event.map_next(Clone::clone)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{operators::just::Just, utils::test_helper::ObservableChecker};

    #[test]
    fn test_subscribe_cloned() {
        let observable = Just::new(123);
        let checker = ObservableChecker::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_completed());
    }
}
