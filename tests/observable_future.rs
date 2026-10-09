mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::{test_channel::test_channel, test_scheduler::block_on};
use futures::FutureExt;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::subject::publish_subject::PublishSubject;
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    operators::others::observable_future::ObservableFuture,
};
use std::convert::Infallible;
use tests_utils::test_struct::TestStruct;

#[test]
fn test_completed() {
    block_on(|_| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();

        let mut future = observable.into_future();
        assert_eq!(channel_checker.state(), ChannelState::Initialized);

        // The first poll subscribes.
        assert!((&mut future).now_or_never().is_none());
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        // The first value resolves the future and stops the source.
        assert!(sender.on_next(111).is_stop());
        assert_eq!((&mut future).await, Some(111));
        // The source is released by the resolution itself, not by the drop of the future.
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);

        drop(future);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_completed_without_next() {
    block_on(|_| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();

        let mut future = observable.into_future();
        assert!((&mut future).now_or_never().is_none());
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        sender.on_termination(Termination::Completed);
        assert_eq!((&mut future).await, None);
        assert_eq!(channel_checker.state(), ChannelState::Completed);
    });
}

#[test]
fn test_unsubscribe() {
    block_on(|_| async move {
        let (sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();

        let mut future = observable.into_future();
        assert!((&mut future).now_or_never().is_none());
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        drop(future);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        drop(sender);
    });
}

#[test]
fn test_ref() {
    block_on(|_| async move {
        let value = 111;
        let mut subject = PublishSubject::<_, Infallible, _>::shared();

        let observable = subject.clone();
        let mut future = observable.into_future();

        // Subscribe in the first time of poll.
        assert!((&mut future).now_or_never().is_none());

        // A subject answers for itself: the future stopping takes only the future away.
        assert!(subject.on_next(&value).is_continue());
        assert_eq!(future.await, Some(&value));
    });
}

#[test]
fn test_mut_ref() {
    block_on(|_| async move {
        let mut value = 111;

        let observable =
            Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, Infallible>| {
                assert!(observer.on_next(&mut value).is_stop());
                DisposeOnDrop::default()
            });

        let future = observable.into_future();

        if let Some(value) = future.await {
            *value *= 2
        } else {
            panic!()
        }

        assert_eq!(value, 222);
    });
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let subject = PublishSubject::<_, Infallible, _>::shared();

        let observable = subject.clone();
        let future = observable.into_future();

        // The future is subscribed and driven by another task, so it is a real waker that the
        // value has to wake.
        let handle = scheduler.spawn(future);
        scheduler.sleep(DURATION_10_MS).await;

        let mut subject_cloned = subject.clone();
        scheduler
            .spawn(async move {
                assert!(subject_cloned.on_next(111).is_continue());
            })
            .await;
        assert_eq!(handle.await, Some(111));
    });
}

#[test]
fn test_without_convenient_api() {
    block_on(|_| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();

        let mut future = ObservableFuture::new(observable);
        assert!((&mut future).now_or_never().is_none());
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);

        assert!(sender.on_next(111).is_stop());
        assert_eq!((&mut future).await, Some(111));
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_single_by_operators() {
    block_on(|_| async move {
        // The future takes the first item; an operator in front of it picks another one, or
        // makes the source always emit, which is the `Single` of ReactiveX.
        let last = FromIter::new([111, 222, 333]).last().into_future().await;
        assert_eq!(last, Some(333));

        let all = FromIter::new([111, 222, 333])
            .collect::<Vec<_>>()
            .into_future()
            .await;
        assert_eq!(all, Some(vec![111, 222, 333]));
    });
}

#[test]
fn test_stop_on_next() {
    block_on(|_| async move {
        // Infinite and synchronous: only the future's answer to its first item can end it, and it
        // has to do so before `subscribe` returns.
        let future = FromIter::new(1..).into_future();
        assert_eq!(future.now_or_never(), Some(Some(1)));
    });
}

#[test]
fn test_next_on_sub() {
    block_on(|_| async move {
        let subject = BehaviorSubject::<_, Infallible, _>::shared(111);

        let observable = subject.clone();
        let future = observable.into_future();

        // The value arrives inside `subscribe`, so the first poll is already the last.
        assert_eq!(future.now_or_never(), Some(Some(111)));
    });
}

#[test]
fn test_complete_on_sub() {
    block_on(|_| async move {
        let future = Empty.with_item_type::<i32>().into_future();

        assert_eq!(future.now_or_never(), Some(None));
    });
}

#[test]
fn test_next_on_unsub() {
    block_on(|_| async move {
        // The source emits from inside its own disposal, so the value arrives while the future is
        // being dropped. It must be dropped instead of reaching anything.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    let mut observer = observer;
                    assert!(observer.on_next(111).is_stop());
                }))
            });

        let mut future = observable.into_future();
        assert!((&mut future).now_or_never().is_none());

        drop(future);
    });
}

#[test]
fn test_complete_on_unsub() {
    block_on(|_| async move {
        // The source completes from inside its own disposal, so it terminates while the future is
        // being dropped. The termination must be dropped instead of reaching anything.
        let observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        let mut future = observable.into_future();
        assert!((&mut future).now_or_never().is_none());

        drop(future);
    });
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _future;

    // Error
    // let _future;
    // let life_marker = TestStruct;

    {
        let observable =
            Create::shared_boxed(|mut observer: SendBoxedObserver<'_, _, Infallible>| {
                assert!(observer.on_next(111).is_stop());
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker.consume_ref();
                }))
            });
        _future = observable.into_future();
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
                assert!(observer.on_next(&life_marker_2).is_stop());
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            });
        let _future = observable.into_future();
    }
}
