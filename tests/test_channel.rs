mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::subscription::Subscription;
use rx_rust::observable::Observable;
use rx_rust::observable::observable_ext::ObservableExt;
use rx_rust::observer::{Observer, Termination};
use rx_rust::safe_lock_option;
use rx_rust::safe_lock_option_disposable;
use rx_rust::utils::types::{Mutable, Shared};
use std::convert::Infallible;
use tests_utils::checker::Checker;
use tests_utils::test_channel::test_channel;

#[test]
fn test_unsub_on_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, _, Infallible>();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_next
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_next(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_next(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_next
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_next(move |observer, value| {
                observer.on_next(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Dropped);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_completed() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, _, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, _, Infallible>();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, _, Infallible>();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Completed);
    sender_3.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Completed);
    assert_eq!(channel_checker_3.state(), ChannelState::Completed);
}

#[test]
fn test_unsub_on_error() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();
    let (checker_3, observer_3) = Checker::new();

    // no unsubscribe
    let _subscription = Some(observable_1.subscribe(observer_1));

    // unsubscribe before on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_2
            .hook_on_termination(move |observer, value| {
                safe_lock_option_disposable!(dispose: sub_cloned);
                observer.on_termination(value);
            })
            .subscribe(observer_2)
    );

    // unsubscribe after on_termination
    let sub = Shared::new(Mutable::new(None::<Subscription<'static>>));
    let sub_cloned = sub.clone();
    safe_lock_option!(replace: sub,
        observable_3
            .hook_on_termination(move |observer, value| {
                observer.on_termination(value);
                safe_lock_option_disposable!(dispose: sub_cloned);
            })
            .subscribe(observer_3)
    );

    assert!(checker_1.values().is_empty());
    assert_eq!(checker_1.state(), State::Active);
    assert!(checker_2.values().is_empty());
    assert_eq!(checker_2.state(), State::Active);
    assert!(checker_3.values().is_empty());
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_next(111);
    sender_2.on_next(111);
    sender_3.on_next(111);
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Active);
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    sender_1.on_termination(Termination::Error("error"));
    sender_2.on_termination(Termination::Error("error"));
    sender_3.on_termination(Termination::Error("error"));
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_1.state(), State::Error("error"));
    assert_eq!(checker_2.values(), [111]);
    assert_eq!(checker_2.state(), State::Error("error"));
    assert_eq!(checker_3.values(), [111]);
    assert_eq!(checker_3.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_3.state(), ChannelState::Error("error"));
}
