mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::{DURATION_3_MS, DURATION_10_MS};
use crate::tests_utils::{test_channel::test_channel, test_scheduler::block_on};
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::observable::Subscription;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::thread_mode::mutable::MutableBoolHelper;
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    operators::others::observable_try_stream::{
        Bounded, Latest, ObservableTryStream, Overflow, StreamBuffer, Unbounded,
    },
};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::{convert::Infallible, num::NonZeroUsize};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|scheduler| async move {
        let mut subject = PublishSubject::<_, &str, _>::shared();

        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_lazy_subscription() {
    block_on(|scheduler| async move {
        let subscribed = Arc::new(AtomicBool::new(false));
        let subscribed_cloned = subscribed.clone();
        let observable =
            Create::shared_boxed(move |mut observer: SendBoxedObserver<'_, _, &str>| {
                subscribed_cloned.write(true);
                assert!(observer.on_next(111).is_continue());
                observer.on_termination(Termination::Completed);
                Subscription::default()
            });

        let stream = observable.into_try_stream();
        assert!(!subscribed.read());

        scheduler.sleep(DURATION_10_MS).await;
        assert!(!subscribed.read());

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await;
        assert!(subscribed.read());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_error() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        assert!(sender.on_next(222).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_error_without_next() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, &str>();

        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(222).is_continue());
        assert!(sender.on_next(333).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_ref() {
    block_on(|scheduler| async move {
        let value = 111;
        let (mut sender, observable, channel_checker) = test_channel();

        let mut stream = observable.into_try_stream();

        // Subscribe in the first time of poll.
        futures::select!(
            _ = stream.next().fuse() => {},
            _ = scheduler.sleep(DURATION_10_MS).fuse()=>{}
        );
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(&value).is_continue());
        assert_eq!(stream.next().await, Some(Ok(&value)));

        sender.on_termination(Termination::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
        assert_eq!(stream.next().await, Some(Err("error")));
        assert_eq!(stream.next().await, None);
        assert_eq!(stream.next().await, None);
    });
}

#[test]
fn test_mut_ref() {
    block_on(|_| async move {
        let mut value = 111;

        let observable = Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, &str>| {
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
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = scheduler
            .spawn(async move { observable.into_try_stream() })
            .await;

        let scheduler_cloned = scheduler.clone();
        let (checker, _subscription) = scheduler
            .spawn(async move { Checker::from_try_stream(stream, scheduler_cloned) })
            .await;

        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let mut sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(222).is_continue());
                assert!(sender.on_next(333).is_continue());
                sender
            })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        scheduler
            .spawn(async move {
                sender.on_termination(Termination::Error("error"));
            })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|scheduler| async move {
        let mut subject = PublishSubject::<_, &str, _>::shared();

        let observable = subject.clone();
        let stream = ObservableTryStream::new(observable);

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(111).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
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
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        sender.on_termination(Termination::Error("error"));
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_unsub_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_continue());
        scheduler.sleep(DURATION_10_MS).await;
        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_unsub_after_completed() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_unsub_after_error() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, &str>();

        let stream = observable.into_try_stream();
        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await; // make sure the stream is ready.
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Error("error"));
        scheduler.sleep(DURATION_10_MS).await;
        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_order_with_continuous_next() {
    block_on(|scheduler| async move {
        let mut subject = PublishSubject::<_, &str, _>::shared();

        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(subject.on_next(*i).is_continue());
        }
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|scheduler| async move {
        let subject = BehaviorSubject::<_, &str, _>::shared(111);

        let observable = subject.clone();
        let stream = observable.into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        subject.on_termination(Termination::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|scheduler| async move {
        let stream = Empty
            .with_item_type::<i32>()
            .with_error_type::<&str>()
            .into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_error_on_sub() {
    block_on(|scheduler| async move {
        let stream = Throw::new("error")
            .with_item_type::<i32>()
            .into_try_stream();

        let (checker, _subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Error("error"));
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|scheduler| async move {
        // The source emits from inside its own disposal, so the value arrives while the stream is
        // being dropped. It must be dropped instead of reaching the stream.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                let mut observer = observer;
                assert!(observer.on_next(111).is_continue());
            }))
        });

        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|scheduler| async move {
        // The source completes from inside its own disposal, so it terminates while the stream is
        // being dropped. The termination must be dropped instead of ending the stream.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        });

        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    });
}

#[test]
fn test_error_on_unsub() {
    block_on(|scheduler| async move {
        // Like `test_complete_on_unsub`, with an error: it must not reach the stream either.
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
            Subscription::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Error("error"));
            }))
        });

        let stream = observable.into_try_stream();

        let (checker, subscription) = Checker::from_try_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        scheduler.sleep(DURATION_10_MS).await;
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
        let observable = Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, &str>| {
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
        let observable = Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, &str>| {
            assert!(observer.on_next(&life_marker_2).is_continue());
            life_marker_1 = Some(observer);
            Subscription::default()
        });
        let _stream = observable.into_try_stream();
    }
}

// The buffers: what a source faster than the consumer leaves for the next poll.

fn capacity(capacity: usize) -> NonZeroUsize {
    NonZeroUsize::new(capacity).unwrap()
}

// The stream subscribes on its first poll, which finds nothing to yield.
macro_rules! subscribe {
    ($stream:expr) => {
        assert_eq!($stream.next().now_or_never(), None)
    };
}

#[test]
fn test_buffer_unbounded() {
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Unbounded::new());
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(111)));
        assert_eq!(stream.next().now_or_never(), Some(Some(222)));
        assert_eq!(stream.next().now_or_never(), Some(Some(333)));
        assert_eq!(stream.next().now_or_never(), None);

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_latest() {
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Latest::new());
        subscribe!(stream);

        // One item at a time goes through unchanged.
        assert!(subject.on_next(111).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(111)));
        assert_eq!(stream.next().now_or_never(), None);

        // Items that pile up between two polls leave only the newest.
        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert!(subject.on_next(444).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(444)));
        assert_eq!(stream.next().now_or_never(), None);

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_latest_completed() {
    // The newest item survives a completion that arrives before it is polled.
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Latest::new());
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(Some(222)));
        assert_eq!(stream.next().now_or_never(), Some(None));
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_latest_error() {
    // The newest item is yielded before the error that followed it.
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, &str, _>::shared();
        let mut stream = subject.clone().into_try_stream_with(Latest::new());
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        subject.on_termination(Termination::Error("error"));
        assert_eq!(stream.next().now_or_never(), Some(Some(Ok(222))));
        assert_eq!(stream.next().now_or_never(), Some(Some(Err("error"))));
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_bounded_drop_oldest() {
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject
            .clone()
            .into_stream_with(Bounded::drop_oldest(capacity(2)));
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert!(subject.on_next(444).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(333)));
        assert_eq!(stream.next().now_or_never(), Some(Some(444)));
        assert_eq!(stream.next().now_or_never(), None);

        // Polling makes room again.
        assert!(subject.on_next(555).is_continue());
        assert!(subject.on_next(666).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(555)));
        assert!(subject.on_next(777).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(666)));
        assert_eq!(stream.next().now_or_never(), Some(Some(777)));
        assert_eq!(stream.next().now_or_never(), None);

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_bounded_drop_newest() {
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject
            .clone()
            .into_stream_with(Bounded::drop_newest(capacity(2)));
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert!(subject.on_next(444).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(111)));
        assert_eq!(stream.next().now_or_never(), Some(Some(222)));
        assert_eq!(stream.next().now_or_never(), None);

        // Polling makes room again.
        assert!(subject.on_next(555).is_continue());
        assert!(subject.on_next(666).is_continue());
        assert!(subject.on_next(777).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(555)));
        assert_eq!(stream.next().now_or_never(), Some(Some(666)));
        assert_eq!(stream.next().now_or_never(), None);

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_bounded_one_is_latest() {
    block_on(|_| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject
            .clone()
            .into_stream_with(Bounded::new(capacity(1), Overflow::DropOldest));
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(333)));
        assert_eq!(stream.next().now_or_never(), None);
    });
}

// Polls a stream of probed values for the number alone, since a probe cannot be compared. The
// probe travels in a `Mutex` for the `Sync` the `Shared` mode needs.
fn poll_number<S>(stream: &mut S) -> Option<Option<i32>>
where
    S: Stream<Item = (i32, Option<Arc<Mutex<DropProbe>>>)> + Unpin,
{
    stream
        .next()
        .now_or_never()
        .map(|item| item.map(|(value, _)| value))
}

#[test]
fn test_buffer_evicted_dropped_outside_lock() {
    // The item a buffer evicts is dropped after the stream's lock is released: its drop sends
    // another item into the source, which lands in the buffer.
    block_on(|_| async move {
        let drop_count = DropCount::new();
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Latest::new());
        assert_eq!(poll_number(&mut stream), None);

        let mut subject_cloned = subject.clone();
        let probe = DropProbe::new()
            .on_drop(drop_count.callback())
            .on_drop(Box::new(move || {
                assert!(subject_cloned.on_next((333, None)).is_continue());
            }));
        assert!(
            subject
                .on_next((111, Some(Arc::new(Mutex::new(probe)))))
                .is_continue()
        );
        assert_eq!(drop_count.get(), 0);

        // 222 evicts 111, whose drop sends 333, which evicts 222.
        assert!(subject.on_next((222, None)).is_continue());
        assert_eq!(drop_count.get(), 1);
        assert_eq!(poll_number(&mut stream), Some(Some(333)));
        assert_eq!(poll_number(&mut stream), None);
    });
}

#[test]
fn test_buffer_evicted_dropped_outside_lock_drop_newest() {
    // Same as above, for the buffer that evicts the item it was just given.
    block_on(|_| async move {
        let drop_count = DropCount::new();
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject
            .clone()
            .into_stream_with(Bounded::drop_newest(capacity(1)));
        assert_eq!(poll_number(&mut stream), None);

        assert!(subject.on_next((111, None)).is_continue());

        let mut subject_cloned = subject.clone();
        let probe = DropProbe::new()
            .on_drop(drop_count.callback())
            .on_drop(Box::new(move || {
                assert!(subject_cloned.on_next((333, None)).is_continue());
            }));
        // 222 is evicted at once; its drop sends 333, which is evicted too.
        assert!(
            subject
                .on_next((222, Some(Arc::new(Mutex::new(probe)))))
                .is_continue()
        );
        assert_eq!(drop_count.get(), 1);
        assert_eq!(poll_number(&mut stream), Some(Some(111)));
        assert_eq!(poll_number(&mut stream), None);
    });
}

#[test]
fn test_buffer_custom() {
    // A buffer whose item is not the source's: the items between two polls become one batch.
    struct Batch<T>(Vec<T>);

    impl<T> StreamBuffer<T> for Batch<T> {
        type Item = Vec<T>;

        fn push(&mut self, item: T) -> Option<T> {
            self.0.push(item);
            None
        }

        fn pop(&mut self) -> Option<Vec<T>> {
            if self.0.is_empty() {
                None
            } else {
                Some(std::mem::take(&mut self.0))
            }
        }
    }

    block_on(|_| async move {
        let mut subject = PublishSubject::<_, &str, _>::shared();
        let mut stream = subject.clone().into_try_stream_with(Batch(Vec::new()));
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(Ok(vec![111]))));

        assert!(subject.on_next(222).is_continue());
        assert!(subject.on_next(333).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(Ok(vec![222, 333]))));
        assert_eq!(stream.next().now_or_never(), None);

        subject.on_termination(Termination::Error("error"));
        assert_eq!(stream.next().now_or_never(), Some(Some(Err("error"))));
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_ref() {
    block_on(|_| async move {
        let values = [111, 222];
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Latest::new());
        subscribe!(stream);

        assert!(subject.on_next(&values[0]).is_continue());
        assert!(subject.on_next(&values[1]).is_continue());
        assert_eq!(stream.next().now_or_never(), Some(Some(&values[1])));

        subject.on_termination(Termination::Completed);
        assert_eq!(stream.next().now_or_never(), Some(None));
    });
}

#[test]
fn test_buffer_async() {
    // A consumer slower than the source sees only the newest item of each burst.
    block_on(|scheduler| async move {
        let mut subject = PublishSubject::<_, Infallible, _>::shared();
        let mut stream = subject.clone().into_stream_with(Latest::new());
        subscribe!(stream);

        assert!(subject.on_next(111).is_continue());
        assert!(subject.on_next(222).is_continue());

        let scheduler_cloned = scheduler.clone();
        let (checker, _subscription) = Checker::from_stream(
            stream.then(move |value| {
                let scheduler = scheduler_cloned.clone();
                async move {
                    scheduler.sleep(DURATION_10_MS).await;
                    value
                }
            }),
            scheduler.clone(),
        );
        scheduler.sleep(DURATION_3_MS).await;
        // 222 is being processed; this burst arrives meanwhile.
        assert!(subject.on_next(333).is_continue());
        assert!(subject.on_next(444).is_continue());
        subject.on_termination(Termination::Completed);

        scheduler.sleep(DURATION_10_MS).await;
        scheduler.sleep(DURATION_10_MS).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [222, 444]);
        assert_eq!(checker.state(), State::Completed);
    });
}
