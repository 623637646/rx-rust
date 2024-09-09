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
        &self,
        observer: impl for<'a> Observer<&'a Never, E> + 'static,
    ) -> impl Cancellable + 'static {
        observer.on(Event::Terminated(Terminated::Error(self.error.clone())));
        NonCancellable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::observable_cloned::ObservableCloned, utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_cloned() {
        let observable = Throw::new(333);
        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(&333));
    }

    #[test]
    fn test_cloned_multiple_subscribe() {
        let observable = Throw::new("My error".to_owned());

        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(&"My error".to_owned()));

        let checker = CheckingObserver::new();
        observable.subscribe_cloned(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error(&"My error".to_owned()));
    }
}
