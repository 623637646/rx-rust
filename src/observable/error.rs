use super::Observable;
use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

/// The `error` function creates an observable that emits an error.
/// ```rust
/// use rx_rust::observable::error::error;
/// use rx_rust::observable::Observable;
/// use rx_rust::observer::Terminated;
/// let error = error("error".to_owned());
/// error.subscribe_on(
///     |_| panic!(),
///     |terminal| match terminal {
///         Terminated::Error(error) => {
///             assert_eq!(error, "error");
///         }
///         _ => panic!("error expected"),
///     },
/// );
/// ```
pub fn error<E>(error: E) -> impl for<'a> Observable<'a, Never, &'a E> {
    Error::new(error)
}

/// The `error_cloned` function creates an observable that emits an error.
/// ```rust
/// use rx_rust::observable::error::error_cloned;
/// use rx_rust::observable::Observable;
/// use rx_rust::observer::Terminated;
/// let error = error_cloned("error".to_owned());
/// error.subscribe_on(
///    |_| panic!(),
///     |terminal| match terminal {
///      Terminated::Error(error) => {
///        assert_eq!(error, "error".to_owned());
///      }
///      _ => panic!("error expected"),
///    },
/// );
/// ```
pub fn error_cloned<E>(error: E) -> impl for<'a> Observable<'a, Never, E>
where
    E: Clone,
{
    Error::new(error)
}

pub(crate) struct Error<E> {
    error: E,
}

impl<E> Error<E> {
    pub(crate) fn new(error: E) -> Error<E> {
        Error { error }
    }
}

impl<'a, E> Observable<'a, Never, &'a E> for Error<E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<Never, &'a E>,
    {
        observer.on(Event::Terminated(Terminated::Error(&self.error)));
        NopDisposable::new()
    }
}

impl<'a, E> Observable<'a, Never, E> for Error<E>
where
    E: Clone,
{
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<Never, E>,
    {
        observer.on(Event::Terminated(Terminated::Error(self.error.clone())));
        NopDisposable::new()
    }
}

#[cfg(test)]
mod tests {
    use super::error_cloned;
    use crate::{
        observable::{error::error, Observable},
        observer::Terminated,
        utils::test_helper::ObservableChecker,
    };

    #[test]
    fn test_ref() {
        let value = "error";
        let checker = ObservableChecker::new();
        let observable = error(value);
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_terminals_matched(&Terminated::Error(&value)));
    }

    #[test]
    fn test_cloned() {
        let value = "error";
        let checker = ObservableChecker::new();
        let observable = error_cloned(value);
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_terminals_matched(&Terminated::Error(value)));
    }

    #[test]
    fn test_multiple_subscribe() {
        let value = "error";
        let observable = error_cloned(value);
        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_terminals_matched(&Terminated::Error(value)));
        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_terminals_matched(&Terminated::Error(value)));
    }
}
