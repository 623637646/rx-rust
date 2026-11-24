mod tests_utils;

use crate::tests_utils::{
    checker::{Checker, State},
    test_channel::{ChannelState, test_channel},
};
use rx_rust::{
    disposable::Disposable,
    observable::{Observable, ObservableExt, either_observable::EitherObservable},
    observer::Observer,
};
use std::convert::Infallible;

#[test]
fn test_unsubscribe() {
    for (choose_left, expected) in [(true, 3), (false, 6)] {
        let (mut sender, source, channel_checker) = test_channel::<'_, i32, Infallible>();
        let observable = if choose_left {
            EitherObservable::Left(source)
        } else {
            EitherObservable::Right(source.map(|value| value * 2))
        };
        let (checker, observer) = Checker::new();

        let subscription = observable.subscribe(observer);
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert!(sender.on_next(3).is_continue());
        assert_eq!(checker.values(), [expected]);
        assert_eq!(checker.state(), State::Active);

        subscription.dispose();
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(checker.state(), State::Dropped);
    }
}
