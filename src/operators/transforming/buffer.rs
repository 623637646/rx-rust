use crate::{
    observable::Observable,
    observer::{Observer, Terminal},
    subscription::Subscription,
};
use educe::Educe;
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE2> {
    source: OE,
    boundary: OE2,
}

impl<OE, OE2> Buffer<OE, OE2> {
    pub fn new(source: OE, boundary: OE2) -> Self {
        Self { source, boundary }
    }
}

impl<'a, T, E, OR, OE, OE2> Observable<'a, Vec<T>, E, OR> for Buffer<OE, OE2>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<'a, T, E, BufferObserver<T, E, OR>>,
    OE2: Observable<'a, (), E, BoundaryObserver<T, E, OR>>,
{
    fn subscribe(self, observer: OR) -> Subscription<'a> {
        let observer = BufferObserver {
            observer: Arc::new(Mutex::new(Some(observer))),
            values: Arc::new(Mutex::new(Vec::default())),
            _marker: PhantomData,
        };
        let subscription_1 = self.source.subscribe(observer.clone());
        let observer = BoundaryObserver(observer);
        let subscription_2 = self.boundary.subscribe(observer);
        subscription_1 + subscription_2
    }
}

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct BufferObserver<T, E, OR> {
    observer: Arc<Mutex<Option<OR>>>,
    values: Arc<Mutex<Vec<T>>>,
    _marker: PhantomData<E>,
}

impl<T, E, OR> Observer<T, E> for BufferObserver<T, E, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, value: T) {
        self.values.lock().unwrap().push(value);
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        match terminal {
            Terminal::Completed => {
                if let Some(mut observer) = self.observer.lock().unwrap().take() {
                    let mut values = self.values.lock().unwrap();
                    if !values.is_empty() {
                        observer.on_next(std::mem::take(&mut values));
                    }
                    observer.on_terminal(Terminal::Completed);
                }
            }
            Terminal::Error(error) => {
                if let Some(observer) = self.observer.lock().unwrap().take() {
                    observer.on_terminal(Terminal::Error(error));
                }
            }
        }
    }
}

pub struct BoundaryObserver<T, E, OR>(BufferObserver<T, E, OR>);

impl<T, E, OR> Observer<(), E> for BoundaryObserver<T, E, OR>
where
    OR: Observer<Vec<T>, E>,
{
    fn on_next(&mut self, _: ()) {
        if let Some(observer) = self.0.observer.lock().unwrap().as_mut() {
            let mut values = self.0.values.lock().unwrap();
            observer.on_next(std::mem::take(&mut values));
        }
    }

    fn on_terminal(self, terminal: Terminal<E>) {
        self.0.on_terminal(terminal);
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
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_last_not_empty() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_from_boundary() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        boundary_subject
            .clone()
            .on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_completed_source_and_boundary_are_same() {
        let mut subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        subject.on_next(());
        assert!(checker.is_values_matched(&[vec![()]]));
        assert!(checker.is_unterminated());

        subject.on_next(());
        assert!(checker.is_values_matched(&[vec![()], vec![()]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![()], vec![()]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_empty() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_last_not_empty() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_error_from_boundary() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        boundary_subject
            .clone()
            .on_terminal(Terminal::Error("error"));
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_error("error"));

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());
        let subscription_2 = observable_2.subscribe(checker_2.clone());
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subscription_1.unsubscribe();

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
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
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_1);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![&value_1]]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_2);
        assert!(checker.is_values_matched(&[vec![], vec![&value_1]]));
        assert!(checker.is_unterminated());

        subject.on_next(&value_3);
        assert!(checker.is_values_matched(&[vec![], vec![&value_1]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::Error(&error));
        assert!(checker.is_values_matched(&[vec![], vec![&value_1]]));
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
            Subscription::new_none_disposal()
        });

        let mut boundary_subject: PublishSubject<'_, (), ()> = PublishSubject::default();
        let observable = observable.buffer(boundary_subject.clone());

        let subscription = observable.subscribe_with_callback(
            |value| {
                for i in value {
                    *i *= 2;
                }
            },
            |_| unreachable!(),
        );

        boundary_subject.on_next(());

        drop(subscription);
        drop(boundary_subject);

        assert_eq!(value_1, 222);
        assert_eq!(value_2, 444);
        assert_eq!(value_3, 666);
    }

    #[tokio::test]
    async fn test_async() {
        let subject = PublishSubject::default();
        let boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());

        let checker_cloned = checker.clone();
        let handle = tokio::spawn(async move { observable.subscribe(checker_cloned) });
        let subscription = handle.await.unwrap();
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        let mut boundary_subject_cloned = boundary_subject.clone();
        let handle = tokio::spawn(async move {
            boundary_subject_cloned.on_next(());
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(111);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        let mut boundary_subject_cloned = boundary_subject.clone();
        let handle = tokio::spawn(async move {
            boundary_subject_cloned.on_next(());
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(222);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let mut subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_next(333);
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let handle = tokio::spawn(async { subscription.unsubscribe() });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        let subject_cloned = subject.clone();
        let handle = tokio::spawn(async move {
            subject_cloned.on_terminal(Terminal::Error("error"));
        });
        handle.await.unwrap();
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker_1 = CheckingObserver::new();
        let checker_2 = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer(boundary_subject.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(checker_1.clone());

        let (on_next, on_terminal) = checker_2.clone().into_callbacks();
        let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
        assert!(checker_1.is_values_matched(&[]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[]));
        assert!(checker_2.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(111);
        assert!(checker_1.is_values_matched(&[vec![]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![]]));
        assert!(checker_2.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(222);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.on_next(333);
        assert!(checker_1.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_1.is_unterminated());
        assert!(checker_2.is_values_matched(&[vec![], vec![111]]));
        assert!(checker_2.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker_1.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_1.is_completed());
        assert!(checker_2.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker_2.is_completed());

        _ = subscription_1; // keep the subscription alive
        _ = subscription_2; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject_1 = PublishSubject::default();
        let mut boundary_subject_2 = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .buffer(boundary_subject_1.clone())
            .buffer(boundary_subject_2.clone());

        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject_2.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject_1.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject_2.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![vec![]]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![], vec![vec![]]]));
        assert!(checker.is_unterminated());

        boundary_subject_2.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![vec![]], vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject_1.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![vec![]], vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject_2.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![vec![]], vec![], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![vec![]], vec![], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![vec![]], vec![], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[
            vec![],
            vec![vec![]],
            vec![],
            vec![vec![111]],
            vec![vec![222, 333]]
        ]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_multiple_operation_same_boundary() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .buffer(boundary_subject.clone())
            .buffer(boundary_subject.clone());

        let subscription = observable.clone().subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![vec![]]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![vec![]]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![vec![]], vec![vec![111]], vec![vec![222, 333]]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[tokio::test]
    async fn test_without_convenient_api() {
        let mut subject = PublishSubject::default();
        let mut boundary_subject = PublishSubject::default();
        let checker = CheckingObserver::new();

        // Custom operations
        let observable = subject.clone();
        let observable = Buffer::new(observable, boundary_subject.clone());

        let subscription = observable.subscribe(checker.clone());
        assert!(checker.is_values_matched(&[]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        subject.on_next(111);
        assert!(checker.is_values_matched(&[vec![]]));
        assert!(checker.is_unterminated());

        boundary_subject.on_next(());
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(222);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.on_next(333);
        assert!(checker.is_values_matched(&[vec![], vec![111]]));
        assert!(checker.is_unterminated());

        subject.clone().on_terminal(Terminal::<&str>::Completed);
        assert!(checker.is_values_matched(&[vec![], vec![111], vec![222, 333]]));
        assert!(checker.is_completed());

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_lifetime() {
        // OK
        let life_marker_1 = TestStruct;
        let life_marker_2 = TestStruct;
        let subscription;

        // Error
        // let subscription;
        // let life_marker_1 = TestStruct;
        // let life_marker_2 = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(111);
                Subscription::new_with_disposal_callback(|| {
                    life_marker_1.consume_ref();
                })
            });
            let boundary_subject = Create::new(|mut observer| {
                observer.on_next(());
                Subscription::new_with_disposal_callback(|| {
                    life_marker_2.consume_ref();
                })
            });
            let observable = observable.buffer(boundary_subject);

            let checker: CheckingObserver<Vec<i32>, ()> = CheckingObserver::new();
            subscription = observable.subscribe(checker.clone());
        }

        _ = subscription; // keep the subscription alive
    }

    #[test]
    fn test_clone() {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_terminal(Terminal::Error("error"));
            Subscription::new_none_disposal()
        });
        let boundary_subject = PublishSubject::default();
        let observable = observable.buffer(boundary_subject);
        let _ = observable.clone(); // make sure it's Clone when T is not Clone.
        observable.subscribe(CheckingObserver::new());
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let boundary_subject = PublishSubject::default();
        let observable = subject.buffer(boundary_subject);

        let observable = observable.buffer_with_count(1);
        let checker = CheckingObserver::new();
        observable.subscribe(checker);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let boundary_subject: PublishSubject<'_, (), String> = PublishSubject::default();
        let observable = subject.buffer(boundary_subject);

        let _ = observable.buffer_with_count(1);
    }
}
