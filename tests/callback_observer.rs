mod tests_utils;

use rx_rust::{
    observable::observable_ext::ObservableExt,
    observer::{Observer, Terminal},
    operators::creating::{create::Create, just::Just},
    subject::publish_subject::PublishSubject,
    subscription::Subscription,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(on_next, on_terminal);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::<&str>::Completed);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_error() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(on_next, on_terminal);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(111);
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_error("error"));

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

    let (on_next, on_terminal) = observer_1.into_callbacks();
    let subscription_1 = observable_1.subscribe_with_callback(on_next, on_terminal);
    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable_2.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subscription_1.unsubscribe();
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_next(222);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_dropped());
    assert!(checker_2.is_values_matched(&[111, 222]));
    assert!(checker_2.is_error("error"));

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

    let (on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(on_next, on_terminal);
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    subject.on_next(&value);
    assert!(checker.is_values_matched(&[&value]));
    assert!(checker.is_active());

    subject.on_terminal(Terminal::Error(&error));
    assert!(checker.is_values_matched(&[&value]));
    assert!(checker.is_error(&error));

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_mut_ref() {
    let mut value = 111;
    let observable = Just::new(&mut value);
    let (checker, observer) = Checker::new();

    let (mut on_next, on_terminal) = observer.into_callbacks();
    let subscription = observable.subscribe_with_callback(
        |value| {
            on_next(*value);
            *value *= 2;
        },
        on_terminal,
    );

    assert!(checker.is_values_matched(&[111]));
    assert!(checker.is_completed());
    assert_eq!(value, 222);

    _ = subscription; // keep the subscription alive
}

#[tokio::test]
async fn test_async() {
    let subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let handle = tokio::spawn(async move {
        let (on_next, on_terminal) = observer.into_callbacks();
        observable.subscribe_with_callback(on_next, on_terminal)
    });
    let subscription = handle.await.unwrap();
    assert!(checker.is_values_matched(&[]));
    assert!(checker.is_active());

    let mut subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_next(&111);
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_active());

    let handle = tokio::spawn(async { subscription.unsubscribe() });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_dropped());

    let subject_cloned = subject.clone();
    let handle = tokio::spawn(async move {
        subject_cloned.on_terminal(Terminal::Error("error"));
    });
    handle.await.unwrap();
    assert!(checker.is_values_matched(&[&111]));
    assert!(checker.is_dropped());
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();

    let (on_next, on_terminal) = observer_1.into_callbacks();
    let subscription_1 = observable
        .clone()
        .subscribe_with_callback(on_next, on_terminal);
    let (on_next, on_terminal) = observer_2.into_callbacks();
    let subscription_2 = observable.subscribe_with_callback(on_next, on_terminal);
    assert!(checker_1.is_values_matched(&[]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[]));
    assert!(checker_2.is_active());

    subject.on_next(111);
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_active());
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_active());

    subject.on_terminal(Terminal::Error("error"));
    assert!(checker_1.is_values_matched(&[111]));
    assert!(checker_1.is_error("error"));
    assert!(checker_2.is_values_matched(&[111]));
    assert!(checker_2.is_error("error"));

    _ = subscription_1; // keep the subscription alive
    _ = subscription_2; // keep the subscription alive
}

#[test]
fn test_lifetime_sub() {
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

        subscription = observable.subscribe_with_callback(|_| {}, |_| {});
    }

    _ = subscription; // keep the subscription alive
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_2 = TestStruct;
    let mut life_marker_1 = None;

    // Error
    // let mut life_marker_1 = None;
    // let life_marker_2 = TestStruct;

    {
        let observable = Create::new(|observer| {
            life_marker_1 = Some(observer);
            Subscription::new_none_disposal()
        });

        let on_next = |_: i32| life_marker_2.consume_ref();
        let on_terminal = |_: Terminal<String>| life_marker_2.consume_ref();
        let subscription = observable.subscribe_with_callback(on_next, on_terminal);

        _ = subscription; // keep the subscription alive
    }
}

#[test]
fn test_fn() {
    let mut s1 = TestStruct;
    let s2 = TestStruct;

    let subject: PublishSubject<'_, i32, &str> = PublishSubject::default();

    // Custom operations
    let observable = subject.clone();

    let subscription = observable.subscribe_with_callback(
        |_| {
            s1.consume_mut();
        },
        |_| {
            s2.consume();
        },
    );
    _ = subscription; // keep the subscription alive
}
