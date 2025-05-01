use super::Subject;
use crate::{
    observable::Observable,
    observer::{Observer, Terminal, boxed_observer::BoxedObserver},
    subscription::Subscription,
    utils::unique_key_store::UniqueKeyStore,
};
use educe::Educe;
use std::sync::{Arc, Mutex, RwLock};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct PublishSubject<'or, T, E> {
    observers: Arc<Mutex<UniqueKeyStore<BoxedObserver<'or, T, E>>>>,
    terminated: Arc<RwLock<Option<Terminal<E>>>>,
}

impl<T, E> PublishSubject<'_, T, E> {
    pub fn new() -> Self {
        Self {
            observers: Arc::new(Mutex::new(UniqueKeyStore::new())),
            terminated: Arc::new(RwLock::new(None)),
        }
    }

    pub fn terminated(&self) -> Option<Terminal<E>>
    where
        E: Clone,
    {
        self.terminated.read().unwrap().as_ref().cloned()
    }
}

impl<T, E> Default for PublishSubject<'_, T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'or, 'sub, T, E> Observable<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + Send + 'or) -> Subscription<'sub> {
        if let Some(terminated) = self.terminated.read().unwrap().as_ref().cloned() {
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

impl<T, E> Observer<T, E> for PublishSubject<'_, T, E>
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
        if self.terminated.read().unwrap().is_some() {
            return;
        }
        *self.terminated.write().unwrap() = Some(terminal.clone());
        for observer in self.observers.lock().unwrap().drain() {
            observer.on_terminal(terminal.clone());
        }
    }
}

impl<'or, 'sub, T, E> Subject<'or, 'sub, T, E> for PublishSubject<'or, T, E>
where
    T: Clone + 'sub,
    E: Clone + 'sub,
    'or: 'sub,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observable::observable_ext::ObservableExt;
    use crate::observer::{Observer, Terminal};
    use crate::utils::tests_utils::checker::Checker;
    use crate::utils::tests_utils::test_struct::TestStruct;
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_completed());
        assert!(matches!(subject.terminated(), Some(Terminal::Completed)));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[111]));
        assert!(checker.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111, 222]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value = 111;
        let error = 222;

        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(observer);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(&value);
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value]));
        assert!(checker.is_error(&error));
        assert!(matches!(subject.terminated(), Some(Terminal::Error(&222))));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(observer) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&111]));
        assert!(checker.is_unterminated());
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_terminal) = observer_2.into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[111]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_lifetime() {
        // OK
        let life_marker = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker = TestStruct;

        {
            let (checker, mut observer) = Checker::<_, Infallible>::new();
            checker.on_next(&life_marker);
            let subject = PublishSubject::default();
            subscription = subject.subscribe(observer);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = PublishSubject::<'_, TestStruct, TestStruct>::default();
        let _ = observable.clone(); // make sure it's Clone when T and E are not Clone.
    }

    #[test]
    fn test_actions_after_terminal() {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let subscription_1 = subject.clone().subscribe(observer_1);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        let subscription_2 = subject.clone().subscribe(observer_2);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        subject.clone().on_terminal(Terminal::Error("error2"));
        assert!(checker_1.is_values_matched(&[111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject;

        let observable = observable.buffer_with_count(1);
        let (checker, observer) = Checker::new();
        observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject;

        let _ = observable.buffer_with_count(1);
    }
}
