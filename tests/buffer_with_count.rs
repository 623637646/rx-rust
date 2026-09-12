mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::{ChannelState, test_channel};
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{creating::create::Create, transforming::buffer_with_count::BufferWithCount},
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(666).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_count_1() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(1).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_last_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(666).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_last_not_empty() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_error_count_1() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(1).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [vec![111]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [vec![111], vec![222]]);
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_2.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker_1.values(), [vec![111, 222, 333]]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(checker_2.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let value_3 = 333;
    let error = -1;

    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_1).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(&value_3).is_continue());
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Active);

    subject.clone().on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [vec![&value_1, &value_2]]);
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    // Custom operations
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(&mut value_1).is_continue());
        assert!(observer.on_next(&mut value_2).is_continue());
        assert!(observer.on_next(&mut value_3).is_continue());
        observer.on_termination(Termination::Error("error"));
        Subscription::default()
    });
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.subscribe_with_callback(
        |value| {
            for i in value {
                *i *= 2;
            }
        },
        |termination| assert!(matches!(termination, Termination::Error("error"))),
    );

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
    assert_eq!(value_3, 333);
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(111).is_continue());
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(222).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(333).is_continue());
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Dropped);

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 222]]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::default();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Active);

    subject.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [vec![111, 222]]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [vec![111, 222]]);
    assert_eq!(checker_2.state(), State::Error("error"));
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = observable
        .buffer_with_count(NonZeroUsize::new(3).unwrap())
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(333).is_stop());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = observable
        .buffer_with_count(NonZeroUsize::new(2).unwrap())
        .buffer_with_count(NonZeroUsize::new(2).unwrap());

    let _subscription = observable.clone().subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(666).is_continue());
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(777).is_continue());
    assert_eq!(checker.values(), [vec![vec![111, 222], vec![333, 444]]]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(
        checker.values(),
        [
            vec![vec![111, 222], vec![333, 444]],
            vec![vec![555, 666], vec![777]]
        ]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let mut subject = PublishSubject::default();
    let (checker, observer) = Checker::new();

    // Custom operations
    let observable = subject.clone();
    let observable = BufferWithCount::new(observable, NonZeroUsize::new(3).unwrap());

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(222).is_continue());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(333).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(444).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(555).is_continue());
    assert_eq!(checker.values(), [vec![111, 222, 333]]);
    assert_eq!(checker.state(), State::Active);

    subject
        .clone()
        .on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable
            .buffer_with_count(NonZeroUsize::new(2).unwrap())
            .buffer_with_count(NonZeroUsize::new(2).unwrap());

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
    }
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
            Subscription::default()
        });
        let observable = observable.buffer_with_count(NonZeroUsize::new(2).unwrap());

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next(vec![&life_marker_2]).is_continue());
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::new(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        Subscription::default()
    });
    let observable = observable.buffer_with_count(NonZeroUsize::new(3).unwrap());
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(NonZeroUsize::new(3).unwrap());

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    // Custom operations
    let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
    let observable = subject.buffer_with_count(NonZeroUsize::new(3).unwrap());

    observable.filter(|_| true);
}
