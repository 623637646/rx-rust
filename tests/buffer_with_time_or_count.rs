mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::DURATION_30_MS;
use crate::tests_utils::DURATION_100_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_runtime::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::{
    observable::{Observable, observable_ext::ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::create::Create, transforming::buffer_with_time_or_count::BufferWithTimeOrCount,
    },
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, num::NonZeroUsize};
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![], vec![111], vec![222, 333]]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_no_delay() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            None,
        );

        let _subscription = observable.subscribe(observer);
        check_with_spawned_late!(
            runtime,
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
            },
            {
                assert_eq!(checker.values(), [vec![]]);
                assert_eq!(checker.state(), State::Active);
            }
        );
        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_10_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert_eq!(checker.values(), [vec![], vec![]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![], vec![], vec![111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(
            checker.values(),
            [vec![], vec![], vec![111], vec![222, 333]]
        );
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(555);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(666);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555, 666]]);
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(555);
        assert_eq!(checker.values(), [vec![111, 222, 333]]);
        assert_eq!(checker.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111, 222, 333], vec![444, 555]]);
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Error("error"));
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let _subscription_2 = observable_2.subscribe(observer_2);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        // assert_eq!(checker_1.state(), State::Active); // This assert may be failed in multi-threaded.
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(444);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(777);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = runtime
            .spawn(async move { observable.subscribe(observer) })
            .await
            .unwrap();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(111);
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_next(222);
                subject_cloned.on_next(222);
            })
            .await
            .unwrap();
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async { subscription.dispose() })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;

        let subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                subject_cloned.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();

        runtime.sleep(DURATION_10_MS).await;
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
        assert_eq!(checker.state(), State::Dropped);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );
        let observable_1 = observable;
        let observable_2 = observable_1.clone();

        let subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker_1.values().is_empty());
        assert_eq!(checker_1.state(), State::Active);
        assert!(checker_2.values().is_empty());
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111]]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Active);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        subscription_1.dispose();
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        // assert_eq!(checker_1.state(), State::Active); // This assert may be failed in multi-threaded.
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(checker_2.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(444);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
        assert_eq!(
            checker_2.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(555);
        subject.on_next(666);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        subject.on_next(777);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Active);

        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker_1.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker_1.state(), State::Dropped);
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
        assert_eq!(checker_2.state(), State::Completed);
    });
}

#[test]
fn test_unsub_on_next_by_take() {
    block_on(|runtime| async move {
        let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable
            .buffer_with_time_or_count(
                NonZeroUsize::new(100).unwrap(),
                DURATION_100_MS,
                runtime.clone(),
                Some(DURATION_100_MS),
            )
            .take(1);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(checker.values(), [vec![]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
                DURATION_100_MS,
                runtime.clone(),
                Some(DURATION_100_MS),
            )
            .buffer_with_time_or_count(
                NonZeroUsize::new(2).unwrap(),
                DURATION_100_MS + DURATION_30_MS,
                runtime.clone(),
                Some(DURATION_100_MS + DURATION_30_MS),
            );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![vec![111, 111], vec![111, 111]]]);
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]]
            ]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS * 2).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]]
            ]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS + DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [
                vec![vec![111, 111], vec![111, 111]],
                vec![vec![222, 222], vec![222, 222]],
                vec![vec![333, 333], vec![333, 333]],
                vec![vec![444, 444]]
            ]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Error("error"));
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        subject.on_next(111);
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(222);
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(checker.values(), [vec![111, 111], vec![222, 222]]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(333);
        subject.on_next(333);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        subject.on_next(444);
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333]]
        );
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
        assert_eq!(
            checker.values(),
            [vec![111, 111], vec![222, 222], vec![333, 333], vec![444]]
        );
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_30_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_100_MS).await;
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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Active);

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
        assert_eq!(checker.state(), State::Error("error"));
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        sender.on_termination(Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        sender.on_termination(Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_scheduler_should_be_disposed_after_unsub() {
    block_on(|runtime| async move {
        let (_sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(100).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        let subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 1);

        subscription.dispose();
        assert_eq!(runtime.get_alive_tasks_count(), 0);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(runtime.get_alive_tasks_count(), 0);
    });
}

#[test]
fn test_immediate_next() {
    block_on(|runtime| async move {
        let subject = BehaviorSubject::new(111);
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = subject.clone();
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            None,
        );

        let _subscription = observable.subscribe(observer);
        check_with_spawned_late!(
            runtime,
            {
                assert!(checker.values().is_empty());
                assert_eq!(checker.state(), State::Active);
            },
            {
                assert_eq!(checker.values(), [vec![111]]);
                assert_eq!(checker.state(), State::Active);
            }
        );
        subject
            .clone()
            .on_termination(Termination::<Infallible>::Completed);
        assert_eq!(checker.values(), [vec![111]]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_immediate_completed() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Empty.buffer_with_time_or_count(
            NonZeroUsize::new(1).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_immediate_error() {
    block_on(|runtime| async move {
        let (checker, observer) = Checker::new();

        // Custom operations
        let observable = Throw::new("error").buffer_with_time_or_count(
            NonZeroUsize::new(1).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        let _subscription = observable.subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
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
                DURATION_100_MS,
                runtime.clone(),
                Some(DURATION_100_MS),
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
            Subscription::default()
        });
        let observable = observable.buffer_with_time_or_count(
            NonZeroUsize::new(2).unwrap(),
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
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
            DURATION_100_MS,
            runtime.clone(),
            Some(DURATION_100_MS),
        );

        observable.filter(|_| true);
    });
}
