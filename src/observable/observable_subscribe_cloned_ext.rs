use super::observable_cloned::ObservableCloned;
use crate::{
    cancellable::Cancellable,
    observer::{anonymous_observer::AnonymousObserver, Event},
};

pub trait ObservableSubscribeClonedExt<T, E> {
    /// Subscribes to the observable with the given `on_event` callback.
    fn subscribe_cloned_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: Fn(Event<T, E>) + 'static;

    /// Subscribes to the observable with the given `on_next` callback.
    fn subscribe_cloned_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: Fn(T) + 'static;
}

impl<T, E, O> ObservableSubscribeClonedExt<T, E> for O
where
    O: ObservableCloned<T, E>,
{
    /// Subscribes to the observable with the given `on_event` callback.
    /// Example:
    /// ```rust
    ///
    fn subscribe_cloned_on_event<F>(&self, on_event: F) -> impl Cancellable + 'static
    where
        F: Fn(Event<T, E>) + 'static,
    {
        let observer = AnonymousObserver::new(on_event);
        self.subscribe_cloned(observer)
    }

    fn subscribe_cloned_on_next<F>(&self, on_next: F) -> impl Cancellable + 'static
    where
        F: Fn(T) + 'static,
    {
        self.subscribe_cloned_on_event(move |event: Event<T, E>| {
            if let Event::Next(value) = event {
                on_next(value);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{observer::Observer, operators::just::Just, utils::test_helper::ObservableChecker};

    #[test]
    fn test_on_event() {
        let observable = Just::new(123);
        let checker = ObservableChecker::new();
        let checker_cloned = checker.clone();
        observable.subscribe_cloned_on_event(move |event| {
            checker_cloned.on(event);
        });
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_completed());
    }

    #[test]
    fn test_on_next() {
        let observable = Just::new(123);
        let checker = ObservableChecker::<i32, String>::new();
        let checker_cloned = checker.clone();
        observable.subscribe_cloned_on_next(move |value| {
            checker_cloned.on(Event::Next(value));
        });
        assert!(checker.is_values_matched(&[123]));
        assert!(checker.is_unterminated());
    }
}
