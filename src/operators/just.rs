use crate::{
    disposable::{nop_disposable::NopDisposable, Disposable},
    observable::Observable,
    observer::{Event, Observer, Terminated},
    utils::never::Never,
};

pub struct Just<T> {
    value: T,
}

impl<T> Just<T> {
    pub fn new(value: T) -> Just<T> {
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
        NopDisposable {}
    }
}

// #[cfg(test)]
// mod tests {
//     use super::just_cloned;
//     use crate::{
//         observable::Observable, observer::Terminated, operators::just::just,
//         utils::test_helper::ObservableChecker,
//     };

//     #[test]
//     fn test_ref() {
//         let value = 123;
//         let checker = ObservableChecker::new();
//         let observable = just(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[&value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//     }

//     #[test]
//     fn test_cloned() {
//         let value = 123;
//         let checker = ObservableChecker::new();
//         let observable = just_cloned(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//     }

//     #[test]
//     fn test_multiple_subscribe() {
//         let value = 123;
//         let observable = just_cloned(value);
//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//         let checker = ObservableChecker::new();
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//     }
// }
