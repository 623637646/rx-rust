use super::publish_subject::PublishSubject;
use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use std::sync::{Arc, RwLock};

pub struct BehaviorSubject<'a, T, E> {
    value: Arc<RwLock<T>>,
    publish_subject: PublishSubject<'a, T, E>,
}

impl<T, E> BehaviorSubject<'_, T, E> {
    pub fn new(value: T) -> Self {
        BehaviorSubject {
            value: Arc::new(RwLock::new(value)),
            publish_subject: PublishSubject::default(),
        }
    }

    pub fn terminated(&self) -> Option<Terminal<E>>
    where
        E: Clone,
    {
        self.publish_subject.terminated()
    }

    pub fn value(&self) -> T
    where
        T: Clone,
    {
        self.value.read().unwrap().clone()
    }
}

impl<T, E> Clone for BehaviorSubject<'_, T, E> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            publish_subject: self.publish_subject.clone(),
        }
    }
}

impl<'a, T, E, OR> Observable<'a, T, E, OR> for BehaviorSubject<'a, T, E>
where
    T: Clone + 'a,
    E: Clone + 'a,
    OR: Observer<T, E> + Send + 'a,
{
    fn subscribe(self, mut observer: OR) -> Subscription<'a> {
        if let Some(terminated) = self.publish_subject.terminated() {
            observer.on_terminal(terminated);
            Subscription::new_none_disposal()
        } else {
            observer.on_next(self.value.read().unwrap().clone());
            self.publish_subject.subscribe(observer)
        }
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<'_, T, E>
where
    T: Clone,
    E: Clone,
{
    fn on_next(&mut self, value: T) {
        if self.publish_subject.terminated().is_none() {
            *self.value.write().unwrap() = value.clone();
            self.publish_subject.on_next(value);
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        if self.publish_subject.terminated().is_none() {
            self.publish_subject.on_terminal(terminal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observable::observable_subscribe_ext::ObservableSubscribeExt;
    use crate::observer::{Observer, Terminal};
    use crate::utils::tests_utils::checking_observer::CheckingObserver;
    use crate::utils::tests_utils::test_struct::TestStruct;
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let mut subject = BehaviorSubject::new(-1);
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[-1]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), -1);

        subject.on_next(111);
        assert!(checker.is_values_matched(&[-1, 111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[-1, 111]));
        assert!(checker.is_completed());
        assert!(matches!(subject.terminated(), Some(Terminal::Completed)));
        assert_eq!(subject.value(), 111);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let mut subject = BehaviorSubject::new(-1);
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[-1]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), -1);

        subject.on_next(111);
        assert!(checker.is_values_matched(&[-1, 111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[-1, 111]));
        assert!(checker.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribe() {
        let mut subject = BehaviorSubject::new(-1);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[-1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), -1);

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1, 111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subscription_1.unsubscribe();
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1, 111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1, 111, 222]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 222);

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1, 111, 222]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 222);

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value_1 = -1;
        let value_2 = 111;
        let error = 222;

        let mut subject = BehaviorSubject::new(&value_1);
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[&value_1]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &value_1);

        subject.on_next(&value_2);
        assert!(checker.is_values_matched(&[&value_1, &value_2]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &value_2);

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[&value_1, &value_2]));
        assert!(checker.is_error(&error));
        assert!(matches!(subject.terminated(), Some(Terminal::Error(&222))));
        assert_eq!(subject.value(), &value_2);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = BehaviorSubject::new(&-1);
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[&-1]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &-1);

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(&111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&-1, &111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &111);

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&-1, &111]));
        assert!(checker.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), &111);

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[&-1, &111]));
        assert!(checker.is_unterminated());
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), &111);
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = BehaviorSubject::new(-1);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.fn_for_subscribe_on();
        let subscription_2 = observable_2.subscribe_on(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[-1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), -1);

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[-1, 111]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[-1, 111]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

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
            let mut checker: CheckingObserver<_, Infallible> = CheckingObserver::new();
            checker.on_next(Some(&life_marker));
            let subject = BehaviorSubject::new(None);
            subscription = subject.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = BehaviorSubject::<'_, _, i32>::new(TestStruct);
        let _ = observable.clone(); // make sure BehaviorSubject is Clone when T and E are not Clone.
    }

    #[test]
    fn test_actions_after_terminal() {
        let mut subject = BehaviorSubject::new(-1);
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        let subscription_1 = subject.clone().subscribe(checker_1.clone());
        assert!(checker_1.is_values_matched(&[-1]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), -1);

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(subject.terminated().is_none());
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        let subscription_2 = subject.clone().subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::Completed);
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        subject.clone().on_terminal(Terminal::Error("error2"));
        assert!(checker_1.is_values_matched(&[-1, 111]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_error("error"));
        assert!(matches!(
            subject.terminated(),
            Some(Terminal::Error("error"))
        ));
        assert_eq!(subject.value(), 111);

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }
}
