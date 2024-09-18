use super::Subject;
use crate::{
    observable::Observable,
    observer::{event::Event, Observer},
    subscription::Subscription,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

pub type ObserversMap<T, E> = HashMap<usize, Arc<dyn Observer<T, E>>>;

pub struct BaseSubject<T, E> {
    observers: Arc<RwLock<ObserversMap<T, E>>>,
    terminated: Arc<RwLock<bool>>,
}

impl<T, E> BaseSubject<T, E> {
    pub fn new() -> Self {
        BaseSubject {
            observers: Arc::new(RwLock::new(HashMap::new())),
            terminated: Arc::new(RwLock::new(false)),
        }
    }
}

impl<T, E> Default for BaseSubject<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, E> Clone for BaseSubject<T, E> {
    fn clone(&self) -> Self {
        BaseSubject {
            observers: self.observers.clone(),
            terminated: self.terminated.clone(),
        }
    }
}

impl<T, E> Observable<T, E> for BaseSubject<T, E>
where
    T: 'static,
    E: 'static,
{
    fn subscribe(self, observer: impl Observer<T, E>) -> Subscription {
        let observer = Arc::new(observer);
        let ptr = &*observer as *const dyn Observer<T, E> as *const () as usize;
        self.observers
            .write()
            .unwrap()
            .insert(ptr, observer.clone());
        Subscription::new(observer.clone(), move || {
            self.observers.write().unwrap().remove(&ptr);
        })
    }
}

impl<T, E> Observer<T, E> for BaseSubject<T, E>
where
    T: Clone + 'static,
    E: Clone + 'static,
{
    fn terminated(&self) -> bool {
        *self.terminated.read().unwrap()
    }

    fn set_terminated(&self, terminated: bool) {
        *self.terminated.write().unwrap() = terminated;
    }

    fn on(&self, event: Event<T, E>) {
        let observers = self.observers.read().unwrap();
        observers.values().for_each(|observer| {
            observer.notify_if_unterminated(event.clone());
        });
    }
}

impl<T, E> Subject<T, E> for BaseSubject<T, E>
where
    T: Clone + 'static,
    E: Clone + 'static,
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
        let observable: BaseSubject<i32, String> = BaseSubject::new();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_error() {
        let observable: BaseSubject<i32, String> = BaseSubject::new();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Error("123".to_owned())));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_error("123".to_owned()));
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_unsubscribed() {
        let observable: BaseSubject<i32, String> = BaseSubject::new();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        subscription.unsubscribe();
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unsubscribed());
    }

    #[test]
    fn test_unterminated() {
        let observable: BaseSubject<i32, String> = BaseSubject::new();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_subscribe() {
        let observable: BaseSubject<i32, String> = BaseSubject::new();
        let checker1 = CheckingObserver::new();
        let checker2 = CheckingObserver::new();
        let subscription1 = observable.clone().subscribe(checker1.clone());
        let subscription2 = observable.clone().subscribe(checker2.clone());
        assert!(checker1.is_values_matched(&[]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker1.is_values_matched(&[1]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_unterminated());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(checker1.is_values_matched(&[1, 2]));
        assert!(checker1.is_completed());
        assert!(checker2.is_values_matched(&[1, 2]));
        assert!(checker2.is_completed());
        _ = subscription1; // keep the subscription alive
        _ = subscription2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operate() {
        let observable1: BaseSubject<i32, String> = BaseSubject::new();
        let observable2: BaseSubject<i32, String> = BaseSubject::new();
        let checker = CheckingObserver::new();
        let subscription1 = observable1.clone().subscribe(observable2.clone());
        let subscription2 = observable2.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        observable1.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription2; // keep the subscription alive
        _ = subscription1; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let observable: BaseSubject<i32, String> = BaseSubject::new();
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
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_default() {
        let observable: BaseSubject<i32, String> = BaseSubject::default();
        let checker = CheckingObserver::new();
        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(1));
        assert!(checker.is_values_matched(&[1]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Next(2));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_unterminated());
        observable.notify_if_unterminated(Event::Terminated(Terminated::Completed));
        assert!(checker.is_values_matched(&[1, 2]));
        assert!(checker.is_completed());
        _ = subscription; // keep the subscription alive
    }
}
