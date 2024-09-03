use super::Observable;
use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

/// The `just` function creates an observable that emits a single value and then completes.
/// ```rust
/// use rx_rust::observable::just::just;
/// use crate::rx_rust::observable::Observable;
/// let just = just(5);
/// just.subscribe_on_next(|value: &i32| {
///    assert_eq!(*value, 5);
/// });
/// ```
pub fn just<T>(value: T) -> impl for<'a> Observable<'a, &'a T, Never> {
    Just::new(value)
}

/// The `just_cloned` function creates an observable that emits a single value and then completes.
/// ```rust
/// use rx_rust::observable::just::just_cloned;
/// use crate::rx_rust::observable::Observable;
/// let just = just_cloned(5);
/// just.subscribe_on_next(|value: i32| {
///    assert_eq!(value, 5);
/// });
/// ```
pub fn just_cloned<T>(value: T) -> impl for<'a> Observable<'a, T, Never>
where
    T: Clone,
{
    Just::new(value)
}

pub(crate) struct Just<T> {
    value: T,
}

impl<T> Just<T> {
    pub(crate) fn new(value: T) -> Just<T> {
        Just { value }
    }
}

impl<'a, T> Observable<'a, &'a T, Never> for Just<T> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<&'a T, Never>,
    {
        observer.on(Event::Next(&self.value));
        observer.on(Event::Terminated(Terminated::Completed));
        NopDisposable::new()
    }
}

impl<'a, T> Observable<'a, T, Never> for Just<T>
where
    T: Clone,
{
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<T, Never>,
    {
        observer.on(Event::Next(self.value.clone()));
        observer.on(Event::Terminated(Terminated::Completed));
        NopDisposable::new()
    }
}

#[cfg(test)]
mod tests {
    use super::just_cloned;
    use crate::{
        observable::{just::just, Observable},
        observer::Terminated,
        utils::test_helper::ObservableChecker,
    };

    #[test]
    fn test_ref() {
        let value = 123;
        let checker = ObservableChecker::new();
        let observable = just(value);
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_terminals_matched(&Terminated::Completed));
    }

    #[test]
    fn test_cloned() {
        let value = 123;
        let checker = ObservableChecker::new();
        let observable = just_cloned(value);
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[value]));
        assert!(checker.is_terminals_matched(&Terminated::Completed));
    }

    #[test]
    fn test_multiple_subscribe() {
        let value = 123;
        let observable = just_cloned(value);
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
