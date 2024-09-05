use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observable::Observable,
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

pub struct Error<E> {
    error: E,
}

impl<E> Error<E> {
    pub fn new(error: E) -> Error<E> {
        Error { error }
    }
}

impl<'a, E> Observable<'a, Never, &'a E> for Error<E> {
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<Never, &'a E>,
    {
        observer.on(Event::Terminated(Terminated::Error(&self.error)));
        NopDisposable {}
    }
}

// #[cfg(test)]
// mod tests {
//     use super::error_cloned;
//     use crate::{
//         observable::Observable, observer::Terminated, operators::error::error,
//         utils::test_helper::ObservableChecker,
//     };

//     #[test]
//     fn test_ref() {
//         let value = "error";
//         let checker = ObservableChecker::new();
//         let observable = error(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[]));
//         assert!(checker.is_terminals_matched(&Terminated::Error(&value)));
//     }

//     #[test]
//     fn test_cloned() {
//         let value = "error";
//         let checker = ObservableChecker::new();
//         let observable = error_cloned(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[]));
//         assert!(checker.is_terminals_matched(&Terminated::Error(value)));
//     }

//     #[test]
//     fn test_multiple_subscribe() {
//         let value = "error";
//         let observable = error_cloned(value);
//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[]));
//         assert!(checker.is_terminals_matched(&Terminated::Error(value)));
//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[]));
//         assert!(checker.is_terminals_matched(&Terminated::Error(value)));
//     }
// }
