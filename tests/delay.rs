mod tests_utils;

use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{create::Create, never::Never},
        utility::delay::Delay,
    },
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(444);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_completed_then_error() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        subject.on_next(444);
        subject.clone().on_termination(Termination::Completed);
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333, 444]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(444);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();
        let (checker_3, observer_3) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();
        let observable_3 = observable_2.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let subscription_2 = observable_2.subscribe(observer_2);
        let _subscription_3 = observable_3.subscribe(observer_3);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        subject.on_next(111);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());
        assert!(checker_3.values().is_empty());
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111]);
        assert!(checker_3.is_active());

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped()); // This assert is ok in multi-thread because the scheduler is finished.
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111]);
        assert!(checker_3.is_active());

        subject.on_next(222);
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111]);
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111]);
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111, 222]);
        assert!(checker_3.is_active());

        subject.on_next(333);
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111, 222]);
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        assert!(checker_2.is_active());
        assert_eq!(checker_3.values(), [111, 222]);
        assert!(checker_3.is_active());

        subscription_2.dispose();
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        // assert!(checker_2.is_active()); // This assert may be failed in multi-thread.
        assert_eq!(checker_3.values(), [111, 222]);
        assert!(checker_3.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        assert!(checker_2.is_dropped());
        assert_eq!(checker_3.values(), [111, 222, 333]);
        assert!(checker_3.is_active());

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [111, 222]);
        assert!(checker_2.is_dropped());
        assert_eq!(checker_3.values(), [111, 222, 333]);
        assert!(checker_3.is_error("error"));
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        let _sender = runtime
            .spawn(async move {
                sender.on_next(&111);
                sender
            })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [&111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;
        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [&111]);
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);

        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        subject.on_next(111);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_active());

        subject.on_termination(Termination::Error("error"));
        assert_eq!(checker_1.values(), [111]);
        assert!(checker_1.is_error("error"));
        assert_eq!(checker_2.values(), [111]);
        assert!(checker_2.is_error("error"));
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .delay(Duration::from_millis(50), runtime.clone())
            .delay(Duration::from_millis(50), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Delay::new(observable, Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(222);
        sender.on_next(333);
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(444);
        sender.on_termination(Termination::Error("error"));
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_error_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(20)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        subscription.dispose();
        runtime.sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_unsubscribed());
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        runtime.sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_dropped());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));

        runtime.sleep(Duration::from_millis(20)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_error("error"));
        assert!(channel_checker.is_error("error"));
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());
    });
}

#[test]
fn test_lifetime_sub() {
    block_on(|runtime| async move {
        // OK
        let life_marker = TestStruct;
        let _subscription;

        // Error
        // let _subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::new(|mut observer| {
                observer.on_next(1);
                observer.on_termination(Termination::<String>::Completed);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = observable.delay(Duration::from_millis(10), runtime.clone());

            let (_, observer) = Checker::new();
            _subscription = observable.subscribe(observer);
        }

        runtime.sleep(Duration::from_millis(20)).await;
    });
}

#[test]
fn test_clone() {
    block_on(|runtime| async move {
        let observable = Create::new(|mut observer| {
            observer.on_next(TestStruct);
            observer.on_termination(Termination::Error(TestStruct));
            Subscription::new_none_disposal()
        });
        let observable = observable.delay(Duration::from_millis(100), runtime.clone());
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.delay(Duration::from_millis(100), runtime.clone());

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let observable = Never.delay(Duration::from_millis(100), runtime.clone());

        observable.filter(|_| true);
    });
}
