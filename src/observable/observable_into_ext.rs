use super::Observable;

pub trait ObservableIntoExt<T, E> {
    fn into_observable(self) -> impl for<'a> Observable<'a, T, E>;
}

impl<T, E, O> ObservableIntoExt<T, E> for O
where
    O: for<'a> Observable<'a, T, E>,
{
    fn into_observable(self) -> impl for<'b> Observable<'b, T, E> {
        self
    }
}

#[cfg(test)]
mod tests {

    use crate::cancellable::non_cancellable::NonCancellable;
    use crate::observable::observable_into_ext::ObservableIntoExt;
    use crate::observable::Observable;
    use crate::observer::{Event, Observer, Terminated};
    use crate::operators::create::Create;
    use crate::operators::just::Just;
    use crate::utils::test_helper::ObservableChecker;

    #[test]
    fn test_create() {
        let value = 123;
        let observable = Create::new(|observer: Box<dyn Observer<i32, String>>| {
            observer.on(Event::Next(value));
            observer.on(Event::Terminated(Terminated::Completed));
            NonCancellable
        })
        .into_observable();

        let checker = ObservableChecker::new();
        observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_completed());
    }

    // #[test]
    // fn test_just() {
    //     let value = 123;
    //     let observable = Just::new(value);
    //     let observable = observable.into_observable();

    //     let checker = ObservableChecker::new();
    //     observable.subscribe(checker.clone());
    //     assert!(checker.is_values_matched(&[&value]));
    //     assert!(checker.is_completed());
    // }
}
