use crate::{disposable::Disposable, observable::Observable, observer::Observer};

pub struct Create<F> {
    subscribe_handler: F,
}

impl<F> Create<F> {
    pub fn new(subscribe_handler: F) -> Create<F> {
        Create { subscribe_handler }
    }
}

impl<'a, T, E, D, F> Observable<'a, T, E> for Create<F>
where
    D: Disposable,
    F: Fn(Box<dyn Observer<T, E>>) -> D,
{
    fn subscribe<O>(&'a self, observer: O) -> impl Disposable
    where
        O: Observer<T, E> + 'static,
    {
        (self.subscribe_handler)(Box::new(observer))
    }
}

// TODO: will do this.
// #[cfg(test)]
// mod tests {
//     use super::create_cloned;
//     use crate::{
//         observable::{create::create, Observable},
//         observer::Terminated,
//         utils::test_helper::ObservableChecker,
//     };

//     #[test]
//     fn test_ref() {
//         let value = 123;
//         let checker = ObservableChecker::new();
//         let observable = create(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[&value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//     }

//     #[test]
//     fn test_cloned() {
//         let value = 123;
//         let checker = ObservableChecker::new();
//         let observable = create_cloned(value);
//         observable.subscribe(checker.clone());
//         assert!(checker.is_values_matched(&[value]));
//         assert!(checker.is_terminals_matched(&Terminated::Completed));
//     }

//     #[test]
//     fn test_multiple_subscribe() {
//         let value = 123;
//         let observable = create_cloned(value);
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
