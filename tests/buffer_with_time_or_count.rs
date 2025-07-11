mod tests_utils;

use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::scheduler::Scheduler;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::create::Create, transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
    },
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{convert::Infallible, num::NonZeroUsize, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed_time_last_empty() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_time_last_not_empty() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_time_no_delay() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            None,
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert!(checker.is_active());

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_time_small_delay() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(30)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert!(checker.is_active());

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_count_last_empty() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(3).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(30)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(222);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject.on_next(444);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject.on_next(555);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject.on_next(666);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_count_last_not_empty() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(3).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(30)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(222);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject.on_next(444);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject.on_next(555);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_time_and_count() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker.is_active());

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(777);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert!(checker.is_completed());
    });
}

#[test]
fn test_error_time_and_count() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker.is_active());

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(777);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject.clone().on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert!(checker.is_error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert!(checker_2.is_active());

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        // assert!(checker_1.is_active()); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        subject.on_next(444);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker_2.is_active());

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        subject.on_next(777);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert!(checker_2.is_completed());
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(222);
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(333);
                subject_cloned.on_next(333);
            })
            .await
            .unwrap();
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(444);
            })
            .await
            .unwrap();
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(555);
                subject_cloned.on_next(666);
            })
            .await
            .unwrap();
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(777);
            })
            .await
            .unwrap();
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(Duration::from_millis(10)).await;

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();

        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert!(checker.is_dropped());
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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker_1.values().is_empty());
        assert!(checker_1.is_active());
        assert!(checker_2.values().is_empty());
        assert!(checker_2.is_active());

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert!(checker_2.is_active());

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_active());
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        // assert!(checker_1.is_active()); // This assert may be failed in multi-thread.
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(10)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_2.is_active());

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        subject.on_next(444);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker_2.is_active());

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        subject.on_next(777);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker_2.is_active());

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker_1.is_dropped());
        assert_eq!(
            checker_2.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
                vec![777]
            ]
        );
        assert!(checker_2.is_completed());
    });
}

#[test]
fn test_multiple_operation() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable
            .buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                Duration::from_millis(90),
                runtime.clone(),
                Some(Duration::from_millis(90)),
            )
            .buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                Duration::from_millis(100),
                runtime.clone(),
                Some(Duration::from_millis(100)),
            );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        subject.on_next(111);
        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(222);
        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(333);
        subject.on_next(333);
        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(444);
        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(555);
        subject.on_next(555);
        subject.on_next(666);
        subject.on_next(666);
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]],
                vec![vec![], vec![555, 555]]
            ]
        );
        assert!(checker.is_active());

        subject.clone().on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]],
                vec![vec![], vec![555, 555]]
            ]
        );
        assert!(checker.is_error("error"));
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = BufferWithTimeOrCount::new(
            observable,
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert!(checker.is_active());

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert!(checker.is_active());

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(50)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![]
            ]
        );
        assert!(checker.is_active());

        runtime.sleep(Duration::from_millis(100)).await;
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject.on_next(777);
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![]
            ]
        );
        assert!(checker.is_active());

        subject.clone().on_termination(Termination::Error("error"));
        assert_eq!(
            checker.values(),
            [
                vec![111, 111],
                vec![222, 222],
                vec![333, 333],
                vec![444],
                vec![555, 666],
                vec![],
                vec![],
            ]
        );
        assert!(checker.is_error("error"));
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111]]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert_eq!(checker.values(), [vec![111]]);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert_eq!(checker.values(), [vec![111]]);
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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(90)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());

        runtime.sleep(Duration::from_millis(20)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

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
                observer.on_next(111);
                Subscription::new_with_disposal_callback(|| {
                    life_marker.consume_ref();
                })
            });

            let observable = observable.buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                Duration::from_millis(100),
                runtime.clone(),
                Some(Duration::from_millis(100)),
            );

            let (_, observer) = Checker::<_, ()>::new();
            _subscription = observable.subscribe(observer);
        }
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
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    });
}

#[test]
fn test_type_inference_with_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        let observable = observable.filter(|_| true);
        let (_, observer) = Checker::new();
        observable.subscribe(observer);
    });
}

#[test]
fn test_type_inference_without_subscribe() {
    block_on(|runtime| async move {
        // Custom operations
        let subject: PublishSubject<'_, i32, String> = PublishSubject::default();
        let observable = subject.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            Duration::from_millis(100),
            runtime.clone(),
            Some(Duration::from_millis(100)),
        );

        observable.filter(|_| true);
    });
}
