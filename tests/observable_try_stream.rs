mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::{DURATION_3_MS, DURATION_10_MS};
use crate::tests_utils::{test_channel::test_channel, test_runtime::block_on};
use futures::{FutureExt, StreamExt, TryStreamExt};
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::Scheduler;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::utils::mutable::{MutableBool, MutableBoolHelper};
use rx_rust::utils::types::Shared;
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination, boxed_observer::BoxedObserver},
    operators::{creating::create::Create, others::observable_try_stream::ObservableTryStream},
    subject::publish_subject::PublishSubject,
};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::<'_, _, &str>::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_lazy_subscription() {
    block_on(|runtime| async move {
        let subscribed = Shared::new(MutableBool::new(false));
        let subscribed_cloned = subscribed.clone();
        let observable = Create::new(move |mut observer: BoxedObserver<'_, _, &str>| {
            subscribed_cloned.write(true);
            assert!(observer.on_next(111).is_continue());
            observer.on_termination(Termination::Completed);
            Subscription::default()
        });

        let stream = observable.into_try_stream();
        assert!(!subscribed.read());

        runtime.sleep(DURATION_10_MS).await;
        assert!(!subscribed.read());

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await;
        assert!(subscribed.read());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Error("error"));
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_error_without_next() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|runtime| async move {
        let mut subject: PublishSubject<'_, _, &str> = PublishSubject::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
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
        let mut stream = observable.into_try_stream();

        // Subscribe in the first time of poll.
        futures::select!(
            _ = stream.next().fuse() => {},
            _ = runtime.sleep(DURATION_10_MS).fuse()=>{}
        );

        assert!(subject.on_next(&value).is_continue());
        assert_eq!(stream.next().await, Some(Ok(&value)));

        subject.on_termination(Termination::Error("error"));
        assert_eq!(stream.next().await, Some(Err("error")));
        assert_eq!(stream.next().await, None);
        assert_eq!(stream.next().await, None);
    });
}

#[test]
fn test_mut_ref() {
    block_on(|_| async move {
        let mut value = 111;

        let observable = Create::new(|mut observer: BoxedObserver<'_, _, &str>| {
            assert!(observer.on_next(&mut value).is_continue());
            observer.on_termination(Termination::Completed);
            Subscription::default()
        });

        let mut stream = observable.into_try_stream();

        if let Some(Ok(value)) = stream.next().await {
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
            .spawn(async move { observable.into_try_stream() })
            .await
            .unwrap();

        let runtime_cloned = runtime.clone();
        let (checker, _subscription) = runtime
            .spawn(async move { Checker::from_try_stream(stream, runtime_cloned) })
            .await
            .unwrap();

        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(111).is_continue());
            })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let mut subject_cloned = subject.clone();
        runtime
            .spawn(async move {
                assert!(subject_cloned.on_next(222).is_continue());
                assert!(subject_cloned.on_next(333).is_continue());
            })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        runtime
            .spawn(async move {
                subject.on_termination(Termination::Error("error"));
            })
            .await
            .unwrap();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::<'_, _, &str>::default();

        // Custom operations
        let observable = subject.clone();
        let stream = ObservableTryStream::new(observable);

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_try_stream_ext() {
    block_on(|_| async move {
        // The whole point of the `Result` encoding: `TryStreamExt` applies directly.
        let stream = FromIter::new([111, 222])
            .with_error_type::<&str>()
            .into_try_stream();
        assert_eq!(stream.try_collect::<Vec<_>>().await, Ok(vec![111, 222]));

        let stream = FromIter::new([111, 222])
            .with_error_type()
            .concat_with(Throw::new("error").with_item_type())
            .into_try_stream();
        assert_eq!(stream.try_collect::<Vec<_>>().await, Err("error"));

        let mut stream = FromIter::new([111])
            .with_error_type::<&str>()
            .into_try_stream();
        assert_eq!(stream.try_next().await, Ok(Some(111)));
        assert_eq!(stream.try_next().await, Ok(None));
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Error("error"));
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|runtime| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|runtime| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        // Custom operations
        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        runtime.sleep(DURATION_10_MS).await;
        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|runtime| async move {
        let mut subject = PublishSubject::<'_, _, &str>::default();

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(subject.on_next(*i).is_continue());
        }
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|runtime| async move {
        let subject = BehaviorSubject::<'_, _, &str>::new(111);

        // Custom operations
        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        runtime.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|runtime| async move {
        // Custom operations
        let stream = Empty
            .with_item_type::<i32>()
            .with_error_type::<&str>()
            .into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_on_sub() {
    block_on(|runtime| async move {
        // Custom operations
        let stream = Throw::new("error")
            .with_item_type::<i32>()
            .into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|runtime| async move {
        // The source emits from inside its own disposal, so the value arrives while the stream is
        // being dropped. It must be dropped instead of reaching the stream.
        let observable = Create::new(|observer: BoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                let mut observer = observer;
                assert!(observer.on_next(111).is_continue());
            }))
        });

        // Custom operations
        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|runtime| async move {
        // The source completes from inside its own disposal, so it terminates while the stream is
        // being dropped. The termination must be dropped instead of ending the stream.
        let observable = Create::new(|observer: BoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        });

        // Custom operations
        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_error_on_unsub() {
    block_on(|runtime| async move {
        // Like `test_complete_on_unsub`, with an error: it must not reach the stream either.
        let observable = Create::new(|observer: BoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        });

        // Custom operations
        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, runtime.clone());
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        runtime.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
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
        let observable = Create::new(|mut observer: BoxedObserver<'_, _, &str>| {
            assert!(observer.on_next(111).is_continue());
            observer.on_termination(Termination::Completed);
            Subscription::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        _stream = observable.into_try_stream();
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
        let observable = Create::new(|mut observer: BoxedObserver<'_, _, &str>| {
            assert!(observer.on_next(&life_marker_2).is_continue());
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let _stream = observable.into_try_stream();
    }
}
