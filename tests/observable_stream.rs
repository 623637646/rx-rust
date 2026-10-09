mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::{DURATION_3_MS, DURATION_10_MS};
use crate::tests_utils::{test_channel::test_channel, test_scheduler::block_on};
use futures::{FutureExt, StreamExt};
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::thread_mode::mutable::MutableBoolHelper;
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    operators::others::observable_stream::ObservableStream,
};
use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = observable.into_stream();

        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
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

        sender.on_termination(Termination::<Infallible>::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_completed_lazy_subscription() {
    block_on(|scheduler| async move {
        let subscribed = Arc::new(AtomicBool::new(false));
        let subscribed_cloned = subscribed.clone();
        let observable = Create::shared_boxed(move |mut observer| {
            subscribed_cloned.write(true);
            assert!(observer.on_next(111).is_continue());
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::default()
        });

        let stream = observable.into_stream();
        assert!(!subscribed.read());

        scheduler.sleep(DURATION_10_MS).await;
        assert!(!subscribed.read());

        let (checker, _subscription) =
            Checker::<_, Infallible>::from_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_10_MS).await;
        assert!(subscribed.read());
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|scheduler| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        let stream = observable.into_stream();
        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
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
fn test_unsubscribe() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();

        let stream = observable.into_stream();

        let (checker, subscription) = Checker::from_stream(stream, scheduler.clone());
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

        let mut stream = observable.into_stream();

        // Subscribe in the first time of poll.
        futures::select!(
            _ = stream.next().fuse() => {},
            _ = scheduler.sleep(DURATION_10_MS).fuse()=>{}
        );
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(&value).is_continue());
        assert_eq!(stream.next().await, Some(&value));

        sender.on_termination(Termination::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
        assert_eq!(stream.next().await, None);
        assert_eq!(stream.next().await, None);
    });
}

#[test]
fn test_mut_ref() {
    block_on(|_| async move {
        let mut value = 111;

        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(&mut value).is_continue());
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::default()
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
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = scheduler
            .spawn(async move { observable.into_stream() })
            .await;

        let scheduler_cloned = scheduler.clone();
        let (checker, _subscription) = scheduler
            .spawn(async move { Checker::from_stream(stream, scheduler_cloned) })
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
                sender.on_termination(Termination::Completed);
            })
            .await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = ObservableStream::new(observable);

        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
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

        sender.on_termination(Termination::<Infallible>::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111, 222, 333]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_complete_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        let stream = observable.into_stream();
        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
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
fn test_unsub_after_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream, scheduler.clone());
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
        let (sender, observable, channel_checker) = test_channel::<'_, i32, _>();

        let stream = observable.into_stream();
        let (checker, subscription) = Checker::from_stream(stream, scheduler.clone());
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
fn test_order_with_continuous_next() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel();

        let stream = observable.into_stream();

        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        let values = (0..1000).collect::<Vec<_>>();
        for i in &values {
            assert!(sender.on_next(*i).is_continue());
        }
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::<Infallible>::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), values);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|scheduler| async move {
        let (sender, source, _) = test_channel();
        let source = source.start_with([111]);

        let observable = source;
        let stream = observable.into_stream();

        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        sender.on_termination(Termination::<Infallible>::Completed);
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|scheduler| async move {
        let stream = Empty.into_stream();

        let (checker, _subscription) = Checker::from_stream(stream, scheduler.clone());
        scheduler.sleep(DURATION_3_MS).await;
        assert_eq!(checker.values(), []);
        assert_eq!(checker.state(), State::Completed);
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|scheduler| async move {
        // The source emits from inside its own disposal, so the value arrives while the stream is
        // being dropped. It must be dropped instead of reaching the stream.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_continue());
                }))
            });

        let stream = observable.into_stream();

        let (checker, subscription) = Checker::from_stream(stream, scheduler.clone());
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
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        let stream = observable.into_stream();

        let (checker, subscription) = Checker::from_stream(stream, scheduler.clone());
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            observer.on_termination(Termination::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
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
        let observable =
            Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, Infallible>| {
                assert!(observer.on_next(&life_marker_2).is_continue());
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            });
        let _stream = observable.into_stream();
    }
}
