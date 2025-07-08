mod tests_utils;

use crate::tests_utils::{
    test_channel::test_channel,
    test_runtime::{block_on, sleep, spawn},
};
use futures::{FutureExt, StreamExt};
use rx_rust::{
    observable::observable_ext::ObservableExt,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, others::observable_stream::ObservableStream},
    subject::publish_subject::PublishSubject,
    subscription::{Subscription, disposable::Disposable},
};
use std::{
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(async {
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_stream();

        let (checker, _) = Checker::from_stream(stream);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(333);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        subject.on_termination(Termination::<Infallible>::Completed);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_lazy_subscription() {
    block_on(async {
        let subscribed = Arc::new(Mutex::new(false));
        let subscribed_cloned = subscribed.clone();
        let observable = Create::new(move |mut observer| {
            *subscribed_cloned.lock().unwrap() = true;
            observer.on_next(111);
            observer.on_termination(Termination::Completed);
            Subscription::new_none_disposal()
        });

        let stream = observable.into_stream();
        assert!(!*subscribed.lock().unwrap());

        sleep(Duration::from_millis(10)).await;
        assert!(!*subscribed.lock().unwrap());

        let (checker, _) = Checker::<_, Infallible>::from_stream(stream);
        sleep(Duration::from_millis(10)).await;
        assert!(*subscribed.lock().unwrap());
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_completed_without_next() {
    block_on(async {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _) = Checker::from_stream(stream);
        sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::Completed);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_completed());
        assert!(channel_checker.is_completed());
    });
}

#[test]
fn test_unsubscribe() {
    block_on(async {
        let mut subject: PublishSubject<'_, _, Infallible> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_stream();

        let (checker, disposal) = Checker::from_stream(stream);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(333);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        disposal.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_ref() {
    block_on(async {
        let value = 111;
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let mut stream = observable.into_stream();

        // Subscribe in the first time of poll.
        futures::select!(
            _ = stream.next().fuse() => {},
            _ = sleep(Duration::from_millis(10)).fuse()=>{}
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
    block_on(async {
        let mut value = 111;

        let observable = Create::new(|mut observer| {
            observer.on_next(&mut value);
            observer.on_termination(Termination::Completed);
            Subscription::new_none_disposal()
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
    block_on(async {
        let subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();

        let stream = spawn(async move { observable.into_stream() })
            .await
            .unwrap();

        let (checker, _) = spawn(async move { Checker::from_stream(stream) })
            .await
            .unwrap();

        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        spawn(async move {
            subject_cloned.on_next(111);
        })
        .await
        .unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        let mut subject_cloned = subject.clone();
        spawn(async move {
            subject_cloned.on_next(222);
            subject_cloned.on_next(333);
        })
        .await
        .unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        spawn(async move {
            subject.on_termination(Termination::Completed);
        })
        .await
        .unwrap();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(async {
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = ObservableStream::new(observable);

        let (checker, _) = Checker::from_stream(stream);
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        sleep(Duration::from_millis(10)).await;
        assert!(checker.values().is_empty());
        assert!(checker.is_active());

        subject.on_next(111);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());

        subject.on_next(222);
        subject.on_next(333);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_active());

        subject.on_termination(Termination::<Infallible>::Completed);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert!(checker.is_completed());
    });
}
#[test]
fn test_complete_after_next() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _) = Checker::from_stream(stream);
        sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sender.on_termination(Termination::Completed);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream);
        sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_dropped());
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(async {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream);
        sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_termination(Termination::Completed);
        sleep(Duration::from_millis(10)).await;
        subscription.dispose();
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), []);
        assert!(checker.is_completed());
    });
}

#[test]
fn test_undisposed_schedule() {
    block_on(async {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();

        // Custom operations
        let stream = observable.into_stream();
        let (checker, _) = Checker::from_stream(stream);
        sleep(Duration::from_millis(10)).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());

        sender.on_next(111);
        sleep(Duration::from_millis(10)).await;
        assert_eq!(checker.values(), [111]);
        assert!(checker.is_active());
        assert!(channel_checker.is_subscribed());
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
            Subscription::new_none_disposal()
        });
        let _stream = observable.into_stream();
    }
}
