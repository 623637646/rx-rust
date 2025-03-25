use super::Subject;
use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    subscription::Subscription,
    utils::unique_key_store::UniqueKeyStore,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct PublishSubject<T, E> {
    observers: Arc<Mutex<UniqueKeyStore<BoxedObserver<'static, T, E>>>>,
    terminated: Arc<Mutex<Option<Terminal<E>>>>,
}

impl<T, E> PublishSubject<T, E> {
    fn new() -> Self {
        PublishSubject {
            observers: Arc::new(Mutex::new(UniqueKeyStore::new())),
            terminated: Arc::new(Mutex::new(None)),
        }
    }
}

impl<T, E> Default for PublishSubject<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E, OR> Observable<T, E, OR> for PublishSubject<T, E>
where
    T: 'static,
    E: Clone + 'static,
    OR: Observer<T, E> + Send + 'static,
{
    fn subscribe(self, observer: OR) -> Subscription {
        if let Some(terminated) = self.terminated.lock().unwrap().as_ref().cloned() {
            observer.on_terminal(terminated);
            return Subscription::new_none_disposal();
        }
        let observers = self.observers;
        let key = observers
            .lock()
            .unwrap()
            .insert(BoxedObserver::new(observer));
        Subscription::new_with_disposal_callback(move || {
            observers.lock().unwrap().remove(key);
        })
    }
}

impl<T, E> Observer<T, E> for PublishSubject<T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        for observer in self.observers.lock().unwrap().iter_mut() {
            observer.on_next(value.clone());
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        if self.terminated.lock().unwrap().is_some() {
            return;
        }
        *self.terminated.lock().unwrap() = Some(terminal.clone());
        for observer in self.observers.lock().unwrap().drain() {
            observer.on_terminal(terminal.clone());
        }
    }
}

impl<T, E, OR> Subject<T, E, OR> for PublishSubject<T, E>
where
    T: Clone + 'static,
    E: Clone + 'static,
    OR: Observer<T, E> + Send + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use crate::observer::{Observer, Terminal};
    use crate::utils::checking_observer::CheckingObserver;

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject: PublishSubject<i32, &str> = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subscription.unsubscribe();
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription_1 = subject.clone().subscribe(checker.clone());
        let subscription_2 = subject.clone().subscribe_on(
            move |value| assert_eq!(value, 111),
            |terminal| assert_eq!(terminal, Terminal::Completed),
        );
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<String>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_terminal_then_subscribe() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive

        let checker = CheckingObserver::new();
        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error("error"));
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_terminal() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive

        let checker = CheckingObserver::new();
        let subscription = subject.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_error("error"));
        _ = subscription; // keep the subscription alive
    }
}
