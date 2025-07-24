#![cfg(feature = "futures")]

mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::{test_channel::test_channel, test_runtime::block_on};
use futures::{FutureExt, StreamExt};
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::scheduler::Scheduler;
use rx_rust::utils::safe_lock::SafeLock;
use rx_rust::utils::types::{Mutable, MutableHelper, Shared};
use rx_rust::{
    observable::observable_ext::ObservableExt,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, others::observable_stream::ObservableStream},
    subject::publish_subject::PublishSubject,
};
use std::{convert::Infallible, time::Duration};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_stream();

        let (checker, _subscription) = Checker::from_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(333);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::<Infallible>::Completed);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_lazy_subscription() {
    block_on(|runtime| async move {
        let subscribed = Shared::new(Mutable::new(false));
        let subscribed_cloned = subscribed.clone();
        let observable = Create::new(move |mut observer| {
            subscribed_cloned.safe_lock_set(true);
            observer.on_next(111);
            observer.on_termination(Termination::Completed);
            Subscription::default()
        });

        let stream = observable.into_stream();
        assert!(!*subscribed.lock_ref());

        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(!*subscribed.lock_ref());

        let (checker, _subscription) =
            Checker::<_, Infallible>::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(*subscribed.lock_ref());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _subscription) = Checker::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_stream();

        let (checker, subscription) = Checker::from_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(333);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_ref() {
    block_on(|runtime| async move {
        let value = 111;
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let mut stream = observable.into_stream();

        // Subscribe in the first time of poll.
        futures::select!(
            _ = stream.next().fuse() => {},
            _ = runtime.clone().sleep(Duration::from_millis(10)).fuse()=>{}
        );

        subject.on_next(&value);
        assert_eq!(stream.next().await, Some(&value));

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().await, None);
        assert_eq!(stream.next().await, None);
    });
}

#[test]
fn test_mut_ref() {
    block_on(|_| async move {
        let mut value = 111;

        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value);
            observer.on_termination(Termination::Completed);
            Subscription::default()
        });

        let mut stream = observable.into_stream();

        if let Some(value) = stream.next().await {
            *value *= 2
        } else {
            panic!()
        }

        assert_eq!(stream.next().await, None);
        assert_eq!(stream.next().await, None);

        assert_eq!(value, 222);
    });
}

#[test]
fn test_async() {
    block_on(|runtime| async move {
        let subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();

        let stream = runtime
            .clone()
            .spawn(async move { observable.into_stream() })
            .await
            .unwrap();

        let runtime_cloned = runtime.clone();
        let (checker, _subscription) = runtime
            .clone()
            .spawn(async move { Checker::from_stream(stream, runtime_cloned) })
            .await
            .unwrap();

        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(111);
            })
            .await
            .unwrap();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .clone()
            .spawn(async move {
                subject_cloned.on_next(222);
                subject_cloned.on_next(333);
            })
            .await
            .unwrap();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .clone()
            .spawn(async move {
                subject.on_termination(Termination::Completed);
            })
            .await
            .unwrap();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = ObservableStream::new(observable);

        let (checker, _subscription) = Checker::from_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subject.on_next(111);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_next(222);
        subject.on_next(333);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::<Infallible>::Completed);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}
#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _subscription) = Checker::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        sender.on_termination(Termination::Completed);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _subscription) = Checker::from_stream(stream, runtime.clone());
        runtime.clone().sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_next(111);
        runtime.clone().sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    });
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _stream;

    // Error
    // let _stream;
    // let life_marker = TestStruct;

    {
        let observable = Create::new(|mut observer| {
            observer.on_next(111);
            observer.on_termination(Termination::Completed);
            Subscription::new_with_disposal_callback(|| {
                life_marker.consume_ref();
            })
        });
        _stream = observable.into_stream();
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
        let observable = Create::new(|mut observer: BoxedObserver<'_, _, Infallible>| {
            observer.on_next(&life_marker_2);
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let _stream = observable.into_stream();
    }
}
