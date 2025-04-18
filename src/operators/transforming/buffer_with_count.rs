use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferWithCount<OE> {
    source: OE,
    count: usize,
}

impl<OE> BufferWithCount<OE> {
    pub fn new(source: OE, count: usize) -> Self {
        Self { source, count }
    }
}

impl<'a, T, E, OR, OE> Observable<'a, Vec<T>, E, OR> for BufferWithCount<OE>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<'a, T, E, BufferWithCountObserver<T, OR>>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observer = BufferWithCountObserver {
            observer,
            values: Vec::default(),
            count: self.count,
        };
        self.source.subscribe(observer)
    }
}

pub struct BufferWithCountObserver<T, OR> {
    observer: OR,
    values: Vec<T>,
    count: usize,
}

impl<T, E, OR> Observer<T, E> for BufferWithCountObserver<T, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.push(value);
        if self.values.len() >= self.count {
            self.observer.on_next(std::mem::take(&mut self.values));
        }
    }

    fn on_terminal(mut self, terminal: Terminal<E>) {
        match terminal {
            Terminal::Completed => {
                if !self.values.is_empty() {
                    self.observer.on_next(std::mem::take(&mut self.values));
                }
                self.observer.on_terminal(Terminal::Completed);
            }
            Terminal::Error(error) => {
                self.observer.on_terminal(Terminal::Error(error));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        observable::{Observable, observable_ext::ObservableExt},
        operators::creating::create::Create,
        subject::publish_subject::PublishSubject,
        utils::tests_utils::{checking_observer::CheckingObserver, test_struct::TestStruct},
    };

    #[tokio::test]
    async fn test_completed_last_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(3);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(666);
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_last_not_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(3);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(3);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(666);
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555, 666]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_not_empty() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(3);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_one_count() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(1);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![111], vec![222]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![111], vec![222]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(3);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(444);
        assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();

        subject.on_next(555);
        assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
        assert!(checker_2.is_completed());

        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_ref() {
        let value_1 = 111;
        let value_2 = 222;
        let value_3 = 333;
        let error = -1;

        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(2);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_1);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_2);
        assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_3);
        assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[vec![&value_1, &value_2]]));
        assert!(checker.is_error(&error));

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_mut_ref() {
        let mut value_1 = 111;
        let mut value_2 = 222;
        let mut value_3 = 333;

        // Custom operations
        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value_1);
            observer.on_next(&mut value_2);
            observer.on_next(&mut value_3);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
        let observable = observable.buffer_with_count(2);

        let subscription = observable.subscribe_with_callback(
            |value| {
                for i in value {
                    *i *= 2;
                }
            },
            |terminal| assert!(matches!(terminal, Terminal::Error("error"))),
        );

        assert_eq!(value_1, 222);
        assert_eq!(value_2, 444);
        assert_eq!(value_3, 333);

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(2);

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(222);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![111, 222]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(333);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![111, 222]]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![111, 222]]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![111, 222]]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(2);
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[vec![111, 222]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![111, 222]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![111, 222]]));
        assert!(checker_2.is_unterminated());

        subject.on_terminal(Terminal::Error("error"));
        assert!(checker_1.is_values_matched(&[vec![111, 222]]));
        assert!(checker_1.is_error("error"));
        assert!(checker_2.is_values_matched(&[vec![111, 222]]));
        assert!(checker_2.is_error("error"));

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(2).buffer_with_count(2);

        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
        assert!(checker.is_unterminated());

        subject.on_next(666);
        assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
        assert!(checker.is_unterminated());

        subject.on_next(777);
        assert!(checker.is_values_matched(&[vec![vec![111, 222], vec![333, 444]]]));
        assert!(checker.is_unterminated());

        subject.on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[
            vec![vec![111, 222], vec![333, 444]],
            vec![vec![555, 666], vec![777]]
        ]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = BufferWithCount::new(observable, 3);

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(444);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.on_next(555);
        assert!(checker.is_values_matched(&[vec![111, 222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![111, 222, 333], vec![444, 555]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
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
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_terminal(Terminal::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });
            let observable = observable.buffer_with_count(2).buffer_with_count(2);

            let checker = CheckingObserver::new();
            subscription = observable.subscribe(checker);
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_terminal(Terminal::Error(TestStruct));
            Subscription::new_none_disposal()
        });
        let observable = observable.buffer_with_count(3);
        let _ = observable.clone(); // make sure it's Clone when T is not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_count(3);

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_count(3);

        let _ = observable.buffer_with_count(1);
    }
}
