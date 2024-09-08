use crate::{
    cancellable::{non_cancellable::NonCancellable, Cancellable},
    observable::Observable,
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

pub struct Throw<E> {
    error: E,
}

impl<E> Throw<E> {
    pub fn new(error: E) -> Throw<E> {
        Throw { error }
    }
}

impl<E> Observable<Never, E> for Throw<E>
where
    E: Clone,
{
    fn subscribe(
        &mut self,
        mut observer: impl for<'a> Observer<&'a Never, E> + 'static,
    ) -> impl Cancellable + 'static {
        observer.on(Event::Terminated(Terminated::Error(self.error.clone())));
        NonCancellable
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
