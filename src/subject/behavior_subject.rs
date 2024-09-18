use super::{base_subject::BaseSubject, Subject};
use crate::{
    observable::Observable,
    observer::{
        event::{Event, Terminated},
        Observer,
    },
    subscription::Subscription,
};
use std::sync::{Arc, RwLock};

pub struct BehaviorSubject<T, E> {
    value: Arc<RwLock<T>>,
    terminated: Arc<RwLock<Option<Terminated<E>>>>,
    base_subject: BaseSubject<T, E>,
}

impl<T, E> BehaviorSubject<T, E> {
    pub fn new(value: T) -> Self {
        BehaviorSubject {
            value: Arc::new(RwLock::new(value)),
            terminated: Arc::new(RwLock::new(None)),
            base_subject: BaseSubject::new(),
        }
    }

    pub fn get_value(&self) -> T
    where
        T: Clone,
    {
        self.value.read().unwrap().clone()
    }

    pub fn get_terminated(&self) -> Option<Terminated<E>>
    where
        E: Clone,
    {
        self.terminated.read().unwrap().clone()
    }
}

impl<T, E> Default for BehaviorSubject<T, E>
where
    T: Default,
{
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T, E> Clone for BehaviorSubject<T, E> {
    fn clone(&self) -> Self {
        BehaviorSubject {
            value: self.value.clone(),
            terminated: self.terminated.clone(),
            base_subject: self.base_subject.clone(),
        }
    }
}

impl<T, E> Observable<T, E> for BehaviorSubject<T, E>
where
    T: Clone + Sync + Send + 'static,
    E: Sync + Send + 'static,
{
    fn subscribe(self, observer: impl Observer<T, E>) -> Subscription {
        observer.notify_if_unterminated(Event::Next(self.get_value()));
        self.base_subject.subscribe(observer)
    }
}

impl<T, E> Observer<T, E> for BehaviorSubject<T, E>
where
    T: Clone + Sync + Send + 'static,
    E: Clone + Sync + Send + 'static,
{
    fn on(&self, event: Event<T, E>) {
        match event.clone() {
            Event::Next(value) => {
                *self.value.write().unwrap() = value;
            }
            Event::Terminated(value) => {
                let mut terminated = self.terminated.write().unwrap();
                assert!(terminated.is_none());
                *terminated = Some(value);
            }
        }
        self.base_subject.on(event);
    }

    fn terminated(&self) -> bool {
        self.base_subject.terminated()
    }

    fn set_terminated(&self, terminated: bool) {
        self.base_subject.set_terminated(terminated);
    }
}

impl<T, E> Subject<T, E> for BehaviorSubject<T, E>
where
    T: Clone + Sync + Send + 'static,
    E: Clone + Sync + Send + 'static,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observer::event::{Event, Terminated},
        utils::checking_observer::CheckingObserver,
    };

    #[test]
    fn test_completed() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated() == Some(Terminated::Completed));
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Error("123".to_owned())));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated() == Some(Terminated::Error("123".to_owned())));
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_error("123".to_owned()));
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribed() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        subscription.unsubscribe();
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unsubscribed());
    }

    #[test]
    fn test_unterminated() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker1 = CheckingObserver::new();
        let checker2 = CheckingObserver::new();
        let subscription1 = observable.clone().subscribe(checker1.clone());
        let subscription2 = observable.clone().subscribe(checker2.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker1.is_values_matched(&[0]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[0]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker1.is_values_matched(&[0, 1]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[0, 1]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker1.is_values_matched(&[0, 1, 2]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[0, 1, 2]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated() == Some(Terminated::Completed));
        assert!(checker1.is_values_matched(&[0, 1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[0, 1, 2]));
        assert!(checker2.is_completed());
        _ = subscription1; // keep the subscription alive
        _ = subscription2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operate() {
        let observable1: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let observable2: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription1 = observable1.clone().subscribe(observable2.clone());
        let subscription2 = observable2.clone().subscribe(checker.clone());
        assert!(observable1.get_value() == 0);
        assert!(observable1.get_terminated().is_none());
        assert!(observable2.get_value() == 0);
        assert!(observable2.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Next(1));
        assert!(observable1.get_value() == 1);
        assert!(observable1.get_terminated().is_none());
        assert!(observable2.get_value() == 1);
        assert!(observable2.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Next(2));
        assert!(observable1.get_value() == 2);
        assert!(observable1.get_terminated().is_none());
        assert!(observable2.get_value() == 2);
        assert!(observable2.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(observable1.get_value() == 2);
        assert!(observable1.get_terminated() == Some(Terminated::Completed));
        assert!(observable2.get_value() == 2);
        assert!(observable2.get_terminated() == Some(Terminated::Completed));
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_completed());
        _ = subscription2; // keep the subscription alive
        _ = subscription1; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::new(0);
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        let observable_cloned = observable.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            observable_cloned.notify_if_unterminated(Event::Next(1));
        });
        let observable_cloned = observable.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            observable_cloned.notify_if_unterminated(Event::Next(2));
        });
        let observable_cloned = observable.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
            observable_cloned.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated() == Some(Terminated::Completed));
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_default() {
        let observable: BehaviorSubject<i32, String> = BehaviorSubject::default();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(observable.get_value() == 0);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(observable.get_value() == 1);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated().is_none());
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(observable.get_value() == 2);
        assert!(observable.get_terminated() == Some(Terminated::Completed));
        assert!(checker.is_values_matched(&[0, 1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }
}
