mod tests_utils;

use crate::tests_utils::DURATION_1_MS;
use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::clone_probe::CloneProbe;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::never::Never;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::operators::creating::timer::Timer;
use rx_rust::scheduler::virtual_time::VirtualTime;
use rx_rust::subject::unicast_subject::{self, BoxedUnicastObservable};
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Flow, Observer, Termination},
    operators::combining::group_join::GroupJoin,
};
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

/// A window, by its left value: the checker it is subscribed with, and the subscription.
type Window<L, R, E> = (
    L,
    Checker<R, E>,
    DisposeOnDrop<unicast_subject::Disposal<Shared>>,
);

/// The windows a group join emitted, each subscribed with a checker as it arrives.
struct Windows<L, R, E>(Arc<Mutex<Vec<Window<L, R, E>>>>);

impl<L, R, E> Windows<L, R, E> {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    /// The callback that subscribes every window it receives.
    fn on_next<'a>(
        &self,
    ) -> impl FnMut((L, BoxedUnicastObservable<'a, R, E, Shared>)) + Send + use<'a, L, R, E>
    where
        L: Send,
        R: Send + 'a,
        E: Send + 'a,
    {
        let windows = self.0.clone();
        move |(left, window)| {
            let (checker, observer) = Checker::new();
            let subscription = window.subscribe(observer);
            windows.with_mut(|windows| windows.push((left, checker, subscription)));
        }
    }

    fn len(&self) -> usize {
        self.0.with_ref(Vec::len)
    }

    fn lefts(&self) -> Vec<L>
    where
        L: Clone,
    {
        self.0
            .with_ref(|windows| windows.iter().map(|(left, ..)| left.clone()).collect())
    }

    fn values(&self, index: usize) -> Vec<R>
    where
        R: Clone,
    {
        self.0.with_ref(|windows| windows[index].1.values())
    }

    fn state(&self, index: usize) -> State<E>
    where
        E: Clone,
    {
        self.0.with_ref(|windows| windows[index].1.state())
    }
}

#[test]
fn test_completed() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Subscribed);
    assert_eq!(right_checker.state(), ChannelState::Subscribed);

    // 1 gets a window at once, with nothing in it yet.
    assert!(left.on_next(1).is_continue());
    assert_eq!(windows.lefts(), [1]);
    assert!(windows.values(0).is_empty());
    assert_eq!(windows.state(0), State::Active);
    assert_eq!(left_durations.state(0), ChannelState::Subscribed);

    assert!(right.on_next("a").is_continue());
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(right_durations.state(0), ChannelState::Subscribed);

    // The window of 2 starts with the right values that are open.
    assert!(left.on_next(2).is_continue());
    assert_eq!(windows.lefts(), [1, 2]);
    assert_eq!(windows.values(1), ["a"]);
    assert_eq!(windows.state(1), State::Active);

    // "a" closes: the windows keep what they received.
    assert!(right_durations.on_next(0, ()).is_stop());
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.values(1), ["a"]);
    assert_eq!(right_durations.state(0), ChannelState::Unsubscribed);

    assert!(right.on_next("b").is_continue());
    assert_eq!(windows.values(0), ["a", "b"]);
    assert_eq!(windows.values(1), ["a", "b"]);

    // 1 closes: its window completes.
    left_durations.on_termination(0, Termination::Completed);
    assert_eq!(windows.values(0), ["a", "b"]);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(windows.state(1), State::Active);

    assert!(right.on_next("c").is_continue());
    assert_eq!(windows.values(0), ["a", "b"]);
    assert_eq!(windows.values(1), ["a", "b", "c"]);

    // "b" and "c" wait for left values that will not come: their windows close. The window of 2
    // is still open, so the stream goes on.
    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Subscribed);
    assert_eq!(right_durations.state(1), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(2), ChannelState::Unsubscribed);

    // "d" reaches the window of 2, and opens no window of its own.
    assert!(right.on_next("d").is_continue());
    assert_eq!(windows.values(1), ["a", "b", "c", "d"]);
    assert_eq!(right_durations.len(), 3);

    // The last window completes, and so does the stream.
    assert!(left_durations.on_next(1, ()).is_stop());
    assert_eq!(windows.values(1), ["a", "b", "c", "d"]);
    assert_eq!(windows.state(1), State::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// No right value can come once the right source has completed: the open windows complete, and a
/// later left value gets a window that is already complete, with the right values still open.
#[test]
fn test_completed_from_right() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable =
        left_observable.group_join(right_observable, move |_| left_duration.clone(), |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    right.on_termination(Termination::Completed);
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_checker.state(), ChannelState::Completed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);

    assert!(left.on_next(2).is_continue());
    assert_eq!(windows.lefts(), [1, 2]);
    assert_eq!(windows.values(1), ["a"]);
    assert_eq!(windows.state(1), State::Completed);
    assert_eq!(left_durations.len(), 1);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_without_open_windows() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(right_observable, |_| Empty, |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(1).is_continue());
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Completed);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_both_sources() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    // The window of 1 is still open, for the right values to come.
    left.on_termination(Termination::Completed);
    assert_eq!(windows.state(0), State::Active);
    assert_eq!(checker.state(), State::Active);

    right.on_termination(Termination::Completed);
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Completed);
    assert_eq!(right_checker.state(), ChannelState::Completed);
}

#[test]
fn test_error() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (right_durations, right_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        move |_| left_duration.clone(),
        move |_| right_duration.clone(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    left.on_termination(Termination::Error("error"));
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
    assert_eq!(right_durations.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_right() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (right, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());

    right.on_termination(Termination::Error("error"));
    assert_eq!(windows.state(0), State::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Error("error"));
}

#[test]
fn test_error_from_duration() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        move |_| left_duration.clone(),
        |_| Never.with_error_type(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("a").is_continue());

    left_durations.on_termination(1, Termination::Error("error"));
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Error("error"));
    assert_eq!(windows.values(1), ["a"]);
    assert_eq!(windows.state(1), State::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
}

/// The window of 1 is completed by the right source's completion, which disposes its duration:
/// an error the duration sends anyway is dropped.
#[test]
fn test_error_from_closed_window_is_dropped() {
    let (mut left, left_observable, _) = test_channel();
    let (right, right_observable, _) = test_channel::<'_, &str, _>();
    let duration_observer = Arc::new(Mutex::new(None));
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let duration_observer_cloned = duration_observer.clone();
    let observable = left_observable.group_join(
        right_observable,
        move |_| {
            let duration_observer = duration_observer_cloned.clone();
            Create::shared_boxed(move |observer: SendBoxedObserver<'_, (), &str>| {
                duration_observer.replace_value(Some(observer));
                DisposeOnDrop::default()
            })
        },
        |_| Never.with_error_type(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    right.on_termination(Termination::Completed);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(checker.state(), State::Active);

    let duration_observer = duration_observer.take_value().unwrap();
    duration_observer.on_termination(Termination::Error("error"));
    assert_eq!(checker.state(), State::Active);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

/// Disposing the outer subscription ends the open windows without a termination.
#[test]
fn test_unsubscribe() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable =
        left_observable.group_join(right_observable, move |_| left_duration.clone(), |_| Never);

    let subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());

    subscription.dispose();
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Dropped);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(left_durations.state(0), ChannelState::Unsubscribed);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let (mut left, left_observable, left_checker) = test_channel();
    let (mut right, right_observable, right_checker) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(&value_1).is_continue());
    assert!(right.on_next(&value_2).is_continue());
    assert_eq!(windows.lefts(), [&value_1]);
    assert_eq!(windows.values(0), [&value_2]);

    left.on_termination(Termination::Error(&error));
    assert_eq!(windows.state(0), State::Error(&error));
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(left_checker.state(), ChannelState::Error(&error));
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
        let (mut right, right_observable, right_checker) = test_channel();
        let windows = Windows::new();
        let (checker, observer) = Checker::<(), _>::new();

        let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

        let on_next = windows.on_next();
        let subscription = scheduler
            .spawn(async move {
                observable.subscribe_with_callback(on_next, |termination| {
                    observer.on_termination(termination)
                })
            })
            .await;
        assert_eq!(windows.len(), 0);
        assert_eq!(checker.state(), State::Active);

        let _left = scheduler
            .spawn(async move {
                assert!(left.on_next(111).is_continue());
                left
            })
            .await;
        let _right = scheduler
            .spawn(async move {
                assert!(right.on_next("111").is_continue());
                right
            })
            .await;
        assert_eq!(windows.lefts(), [111]);
        assert_eq!(windows.values(0), ["111"]);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert_eq!(windows.state(0), State::Dropped);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (left_channels, left_observable) = test_channels::<_, Infallible>();
    let (right_channels, right_observable) = test_channels();
    let windows_1 = Windows::new();
    let windows_2 = Windows::new();

    let observable_1 = left_observable.group_join(right_observable, |_| Never, |_| Never);
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe_with_callback(windows_1.on_next(), |_| {});
    let _subscription_2 = observable_2.subscribe_with_callback(windows_2.on_next(), |_| {});

    assert!(left_channels.on_next(0, 111).is_continue());
    assert!(right_channels.on_next(0, "111").is_continue());
    assert!(left_channels.on_next(1, 222).is_continue());
    assert!(right_channels.on_next(1, "222").is_continue());
    assert_eq!(windows_1.lefts(), [111]);
    assert_eq!(windows_1.values(0), ["111"]);
    assert_eq!(windows_2.lefts(), [222]);
    assert_eq!(windows_2.values(0), ["222"]);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .group_join(right_observable, |_| Never, |_| Never)
        .map(|(left, _)| left)
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::stopping_after(1);

    let observable = left_observable
        .group_join(right_observable, |_| Never, |_| Never)
        .map(|(left, _)| left);

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Windows::new();
    let on_next = windows.on_next();

    let observable = left_observable
        .group_join(right_observable, |_| Never, |_| Never)
        .hook_on_next(move |observer, value| {
            let flow = observer.on_next(value);
            if let Some(subscription) = subscription_cloned.take_value() {
                subscription.dispose();
            }
            flow
        });

    subscription.replace_value(Some(
        observable.subscribe_with_callback(on_next, |_| unreachable!()),
    ));
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Dropped);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The consumer of one window stops that window, not the operator: the other windows go on.
#[test]
fn test_stop_on_next_of_window() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .group_join(right_observable, |_| Never, |_| Never)
        .flat_map(|(left, window)| window.take(1).map(move |right| (left, right)));

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("b").is_continue());
    assert_eq!(checker.values(), [(1, "a"), (2, "a")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_checker.state(), ChannelState::Subscribed);
}

/// The consumer of the first of two windows disposes the subscription on the right value both
/// windows get, and answers `Continue`: the second window does not get it.
#[test]
fn test_unsub_on_next_of_window() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_cloned = windows.clone();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    subscription.replace_value(Some(observable.subscribe_with_callback(
        move |(_, window)| {
            let (checker, observer) = Checker::new();
            let subscription = subscription_cloned.clone();
            let window_subscription = window
                .hook_on_next(move |observer, value| {
                    let flow = observer.on_next(value);
                    if let Some(subscription) = subscription.take_value() {
                        subscription.dispose();
                    }
                    flow
                })
                .subscribe(observer);
            windows_cloned.with_mut(|windows| windows.push((checker, window_subscription)));
        },
        |_| unreachable!(),
    )));
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("a").is_stop());
    windows.with_ref(|windows| {
        assert_eq!(windows[0].0.values(), ["a"]);
        assert_eq!(windows[0].0.state(), State::Dropped);
        assert_eq!(windows[1].0.values(), [] as [&str; 0]);
        assert_eq!(windows[1].0.state(), State::Dropped);
    });
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The clone of a right value for the first of two windows disposes the subscription: neither
/// window gets it.
#[test]
fn test_unsub_on_clone() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Windows::new();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    subscription.replace_value(Some(
        observable.subscribe_with_callback(windows.on_next(), |_| unreachable!()),
    ));
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    let probe = CloneProbe::new(10).on_clone(Box::new(move || {
        if let Some(subscription) = subscription_cloned.take_value() {
            subscription.dispose();
        }
    }));
    assert!(right.on_next(probe).is_stop());
    assert_eq!(windows.lefts(), [1, 2]);
    for index in 0..2 {
        assert!(windows.values(index).is_empty());
        assert_eq!(windows.state(index), State::Dropped);
    }
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The clone of an open right value for the window of a new left value disposes the subscription:
/// the window is not emitted.
#[test]
fn test_unsub_on_clone_into_new_window() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Windows::new();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    subscription.replace_value(Some(
        observable.subscribe_with_callback(windows.on_next(), |_| unreachable!()),
    ));
    let probe = CloneProbe::new(10).on_clone(Box::new(move || {
        if let Some(subscription) = subscription_cloned.take_value() {
            subscription.dispose();
        }
    }));
    // No window is open: the value is kept, not cloned.
    assert!(right.on_next(probe).is_continue());
    assert!(left.on_next(1).is_stop());
    assert_eq!(windows.len(), 0);
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// The consumer of the first of two windows disposes the subscription as the right source's
/// completion completes it: the second window is dropped without a termination.
#[test]
fn test_unsub_on_completed_of_window() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (right, right_observable, right_checker) = test_channel::<'_, &str, _>();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_cloned = windows.clone();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    subscription.replace_value(Some(observable.subscribe_with_callback(
        move |(_, window)| {
            let (checker, observer) = Checker::new();
            let subscription = subscription_cloned.clone();
            let window_subscription = window
                .hook_on_termination(move |observer, termination| {
                    observer.on_termination(termination);
                    if let Some(subscription) = subscription.take_value() {
                        subscription.dispose();
                    }
                })
                .subscribe(observer);
            windows_cloned.with_mut(|windows| windows.push((checker, window_subscription)));
        },
        |_| unreachable!(),
    )));
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    right.on_termination(Termination::Completed);
    windows.with_ref(|windows| {
        assert_eq!(windows[0].0.state(), State::Completed);
        assert_eq!(windows[1].0.state(), State::Dropped);
    });
    assert_eq!(left_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(right_checker.state(), ChannelState::Completed);
}

/// The consumer of the first of two windows disposes the subscription as an error ends it: the
/// second window, and the stream, do not get the error.
#[test]
fn test_unsub_on_error_of_window() {
    let (mut left, left_observable, left_checker) = test_channel();
    let (_, right_observable, right_checker) = test_channel::<'_, &str, _>();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_cloned = subscription.clone();
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_cloned = windows.clone();

    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    subscription.replace_value(Some(observable.subscribe_with_callback(
        move |(_, window)| {
            let (checker, observer) = Checker::new();
            let subscription = subscription_cloned.clone();
            let window_subscription = window
                .hook_on_termination(move |observer, termination| {
                    observer.on_termination(termination);
                    if let Some(subscription) = subscription.take_value() {
                        subscription.dispose();
                    }
                })
                .subscribe(observer);
            windows_cloned.with_mut(|windows| windows.push((checker, window_subscription)));
        },
        |_| unreachable!(),
    )));
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    left.on_termination(Termination::Error("error"));
    windows.with_ref(|windows| {
        assert_eq!(windows[0].0.state(), State::Error("error"));
        assert_eq!(windows[1].0.state(), State::Dropped);
    });
    assert_eq!(left_checker.state(), ChannelState::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// A window whose consumer stopped is dropped at the next right value, which it then no longer
/// gets a copy of, although its left value stays open until its duration ends.
#[test]
fn test_stop_on_next_of_window_drops_it() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .group_join(right_observable, |_| Never, |_| Never)
        .flat_map(|(left, window)| {
            window
                .take(1)
                .map(move |right: CloneProbe| (left, right.value()))
        });

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next(CloneProbe::new(1)).is_continue());
    assert_eq!(checker.values(), [(1, 1)]);

    // The window of 1 is stopped, and no other is open: 2 is not cloned.
    let cloned = Arc::new(AtomicBool::new(false));
    let cloned_cloned = cloned.clone();
    let probe = CloneProbe::new(2).on_clone(Box::new(move || {
        cloned_cloned.store(true, Ordering::SeqCst)
    }));
    assert!(right.on_next(probe).is_continue());
    assert!(!cloned.load(Ordering::SeqCst));

    // The right values are still open for a new window.
    assert!(left.on_next(3).is_continue());
    assert!(cloned.load(Ordering::SeqCst));
    assert_eq!(checker.values(), [(1, 1), (3, 1)]);
    assert_eq!(checker.state(), State::Active);
}

/// Another thread disposes the subscription while the first of two windows gets a right value,
/// and is still disposing the durations: the second window does not get it.
#[test]
fn test_unsub_from_other_thread_on_next() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (dispose_sender, dispose) = mpsc::channel::<()>();
    let (disposing_sender, disposing) = mpsc::channel::<()>();
    let (resume_sender, resume) = mpsc::channel::<()>();
    let blocked_disposal = Arc::new(Mutex::new(Some((disposing_sender, resume))));
    let first_value = Arc::new(Mutex::new(Some((dispose_sender, disposing))));
    let windows = Arc::new(Mutex::new(Vec::new()));
    let windows_cloned = windows.clone();

    let observable = left_observable.group_join(
        right_observable,
        move |_| {
            let blocked_disposal = blocked_disposal.clone();
            Create::shared_boxed(move |_: SendBoxedObserver<'_, (), Infallible>| {
                let blocked_disposal = blocked_disposal.clone();
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    if let Some((disposing, resume)) = blocked_disposal.take_value() {
                        disposing.send(()).unwrap();
                        resume.recv_timeout(TIMEOUT).unwrap();
                    }
                }))
            })
        },
        |_| Never,
    );

    let subscription = observable.subscribe_with_callback(
        move |(_, window)| {
            let (checker, observer) = Checker::new();
            let first_value = first_value.clone();
            let window_subscription = window
                .hook_on_next(move |observer, value| {
                    let flow = observer.on_next(value);
                    if let Some((dispose, disposing)) = first_value.take_value() {
                        dispose.send(()).unwrap();
                        // The other thread has stopped the context, and is disposing the
                        // durations.
                        disposing.recv_timeout(TIMEOUT).unwrap();
                    }
                    flow
                })
                .subscribe(observer);
            windows_cloned.with_mut(|windows| windows.push((checker, window_subscription)));
        },
        |_| unreachable!(),
    );
    let disposer = thread::spawn(move || {
        dispose.recv_timeout(TIMEOUT).unwrap();
        subscription.dispose();
    });
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("a").is_stop());
    windows.with_ref(|windows| {
        assert_eq!(windows[0].0.values(), ["a"]);
        assert_eq!(windows[0].0.state(), State::Dropped);
        assert_eq!(windows[1].0.values(), [] as [&str; 0]);
        assert_eq!(windows[1].0.state(), State::Dropped);
    });
    resume_sender.send(()).unwrap();
    disposer.join().unwrap();
}

/// Another thread disposes the subscription, and returns, while the error is cloned for the first
/// of two windows: neither window, nor the stream, gets it.
#[test]
fn test_unsub_from_other_thread_on_clone_of_error() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let (mut left, left_observable, _) = test_channel();
    // A test channel clones the error it records, so the error goes through a plain observer.
    let right_observer = Arc::new(Mutex::new(None));
    let right_observer_cloned = right_observer.clone();
    let right_observable =
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, CloneProbe>| {
            right_observer_cloned.replace_value(Some(observer));
            DisposeOnDrop::default()
        });
    let (dispose_sender, dispose) = mpsc::channel::<()>();
    let (disposed_sender, disposed) = mpsc::channel::<()>();
    let windows = Windows::new();

    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    let subscription = observable.subscribe_with_callback(windows.on_next(), |_| unreachable!());
    let disposer = thread::spawn(move || {
        dispose.recv_timeout(TIMEOUT).unwrap();
        subscription.dispose();
        disposed_sender.send(()).unwrap();
    });
    assert!(left.on_next(1).is_continue());
    assert!(left.on_next(2).is_continue());
    let error = CloneProbe::new(0).on_clone(Box::new(move || {
        dispose_sender.send(()).unwrap();
        disposed.recv_timeout(TIMEOUT).unwrap();
    }));
    let right_observer = right_observer.take_value().unwrap();
    right_observer.on_termination(Termination::Error(error));
    for index in 0..2 {
        assert!(matches!(windows.state(index), State::Dropped));
    }
    disposer.join().unwrap();
}

/// A duration that fails while it is subscribed, behind a delivery running on another thread,
/// stops the source of its value: the error is only queued, and the update that stores the
/// duration says the stream is over.
#[test]
fn test_duration_errors_on_sub_while_delivering() {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let (mut left, left_observable, _) = test_channel::<'_, i32, _>();
    let (mut right, right_observable, _) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();
    let (delivering_sender, delivering) = mpsc::channel();
    let (resume_sender, resume) = mpsc::channel();
    let blocked = Arc::new(Mutex::new(Some((delivering_sender, resume))));

    let observable = left_observable
        .group_join(
            right_observable,
            |_| Never.with_error_type(),
            |_| Throw::new("error"),
        )
        .hook_on_next(move |observer, value| {
            let flow = observer.on_next(value);
            if let Some((delivering, resume)) = blocked.take_value() {
                delivering.send(()).unwrap();
                resume.recv_timeout(TIMEOUT).unwrap();
            }
            flow
        });

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    let deliverer = thread::spawn(move || left.on_next(1).is_stop());
    delivering.recv_timeout(TIMEOUT).unwrap();
    assert!(right.on_next("a").is_stop());
    assert_eq!(windows.lefts(), [1]);
    assert!(windows.values(0).is_empty());
    assert_eq!(checker.state(), State::Active);

    resume_sender.send(()).unwrap();
    assert!(deliverer.join().unwrap());
    assert_eq!(windows.values(0), ["a"]);
    assert_eq!(windows.state(0), State::Error("error"));
    assert_eq!(checker.state(), State::Error("error"));
}

#[test]
fn test_multiple_operation() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let (checker, observer) = Checker::new();

    let observable = left_observable
        .map(|value| value * 10)
        .group_join(
            right_observable.filter(|value| value % 2 == 1),
            move |_| left_duration.clone(),
            |_| Never,
        )
        .flat_map(|(left, window)| window.sum().map(move |sum| left + sum));

    let _subscription = observable.subscribe(observer);
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next(1).is_continue());
    assert!(right.on_next(2).is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next(3).is_continue());
    assert!(checker.values().is_empty());

    left_durations.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [24]);
    left_durations.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [24, 14]);

    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_without_convenient_api() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = GroupJoin::new(left_observable, right_observable, |_| Never, |_| Empty);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("b").is_continue());
    assert_eq!(windows.values(0), ["a", "b"]);
    assert_eq!(windows.values(1), ["b"]);

    right.on_termination(Termination::Completed);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(windows.state(1), State::Completed);
    left.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
}

/// A left duration that completes at once completes the window as soon as it is emitted, with the
/// right values open then.
#[test]
fn test_duration_completes_on_sub() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, _) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(right_observable, |_| Empty, |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    assert!(right.on_next("a").is_continue());
    assert!(left.on_next(2).is_continue());
    assert!(right.on_next("b").is_continue());
    assert!(left.on_next(3).is_continue());
    assert_eq!(windows.lefts(), [1, 2, 3]);
    assert!(windows.values(0).is_empty());
    assert_eq!(windows.values(1), ["a"]);
    assert_eq!(windows.values(2), ["a", "b"]);
    for index in 0..3 {
        assert_eq!(windows.state(index), State::Completed);
    }
    assert_eq!(checker.state(), State::Active);
}

/// A duration that fails while it is subscribed ends the stream, and the value that opened it
/// answers its source with a stop: a source emitting while it is subscribed has no disposal yet,
/// so only the flow stops it.
#[test]
fn test_duration_errors_on_sub() {
    let (_, left_observable, _) = test_channel::<'_, i32, _>();
    let flow = Arc::new(Mutex::new(None));
    let right_observable = Create::shared_boxed({
        let flow = flow.clone();
        move |mut observer: SendBoxedObserver<'_, &str, &str>| {
            let answer = observer.on_next("a");
            flow.replace_value(Some(answer));
            DisposeOnDrop::default()
        }
    });
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Throw::new("error"),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert_eq!(flow.clone_value(), Some(Flow::Stop));
    assert!(windows.lefts().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
}

/// The windows are opened and closed on the scheduler's clock: a window closes exactly when its
/// timer fires.
#[test]
fn test_time_windows() {
    let time = VirtualTime::new();
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let scheduler = time.scheduler();
    let left_scheduler = scheduler.clone();
    let observable = left_observable.group_join(
        right_observable,
        move |_| Timer::new((), Duration::from_millis(6), left_scheduler.clone()),
        move |_| Timer::new((), Duration::from_millis(2), scheduler.clone()),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next("A").is_continue()); // Open until 6 ms.
    time.advance_by(Duration::from_millis(2));
    assert!(right.on_next(1).is_continue()); // Open until 4 ms.
    assert_eq!(windows.values(0), [1]);

    time.advance_by(Duration::from_millis(2));
    assert!(left.on_next("B").is_continue()); // 1 closed at 4 ms.
    assert!(windows.values(1).is_empty());

    time.advance_by(Duration::from_millis(2) - DURATION_1_MS);
    assert!(right.on_next(2).is_continue());
    assert_eq!(windows.values(0), [1, 2]);
    assert_eq!(windows.values(1), [2]);
    assert_eq!(windows.state(0), State::Active);

    time.advance_by(DURATION_1_MS); // A closes.
    assert_eq!(windows.state(0), State::Completed);
    assert!(right.on_next(3).is_continue());
    assert_eq!(windows.values(0), [1, 2]);
    assert_eq!(windows.values(1), [2, 3]);

    left.on_termination(Termination::Completed);
    time.advance_by(Duration::from_millis(4) - DURATION_1_MS);
    assert_eq!(windows.state(1), State::Active);
    assert_eq!(checker.state(), State::Active);
    time.advance_by(DURATION_1_MS); // B closes.
    assert_eq!(windows.state(1), State::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_next_on_sub() {
    let (mut left, left_observable, _) = test_channel::<'_, _, Infallible>();
    let left_observable = left_observable.start_with([1]);
    let (right, right_observable, _) = test_channel();
    let right_observable = right_observable.start_with(["a"]);
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert_eq!(windows.lefts(), [1]);
    assert_eq!(windows.values(0), ["a"]);

    assert!(left.on_next(2).is_continue());
    assert_eq!(windows.values(1), ["a"]);

    right.on_termination(Termination::Completed);
    left.on_termination(Termination::Completed);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(windows.state(1), State::Completed);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let windows = Windows::<i32, i32, Infallible>::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = Empty
        .with_item_type()
        .group_join(right_observable, |_| Never, |_| Never);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(right_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_sub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, _>();
    let windows = Windows::<i32, i32, _>::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable = Throw::new("error").with_item_type().group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(right_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (mut right, right_observable, right_checker) = test_channel::<'_, i32, Infallible>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = observable.group_join(right_observable, |_| Never, |_| Never);

    let subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(right.on_next(1).is_continue());
    assert_eq!(windows.len(), 0);

    subscription.dispose();
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, Infallible>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = observable.group_join(right_observable, |_| Never, |_| Never);

    let subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    subscription.dispose();
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, right_observable, right_checker) = test_channel::<'_, i32, &str>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Never.with_error_type(),
    );

    let subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    subscription.dispose();
    assert_eq!(windows.len(), 0);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(right_checker.state(), ChannelState::Unsubscribed);
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// the open windows still receive the values of the other source until they close. No window
/// completes on the source's behalf, and once nothing more can come, the observer is dropped,
/// never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut left, left_observable, left_checker) = test_channel::<'_, _, Infallible>();
    let (mut right, right_observable, right_checker) = test_channel();
    let (left_durations, left_duration) = test_channels::<(), _>();
    let windows = Windows::new();
    let (checker, observer) = Checker::<(), _>::new();

    let observable =
        left_observable.group_join(right_observable, move |_| left_duration.clone(), |_| Empty);

    let _subscription = observable.subscribe_with_callback(windows.on_next(), |termination| {
        observer.on_termination(termination)
    });
    assert!(left.on_next(1).is_continue());
    left.abandon();
    assert_eq!(checker.state(), State::Active);
    assert_eq!(left_checker.state(), ChannelState::Abandoned);

    assert!(right.on_next("a").is_continue());
    assert_eq!(windows.values(0), ["a"]);
    right.abandon();
    assert_eq!(windows.state(0), State::Active);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(right_checker.state(), ChannelState::Abandoned);

    left_durations.on_termination(0, Termination::Completed);
    assert_eq!(windows.state(0), State::Completed);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker_1 = TestStruct;
    let life_marker_2 = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker_1 = TestStruct;
    // let life_marker_2 = TestStruct;

    {
        let left_observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let right_observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(222).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = left_observable
            .group_join(
                right_observable,
                |_| Never.with_error_type(),
                |_| Never.with_error_type(),
            )
            .map(|(left, _): (i32, _)| left);

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or_sub() {
    // OK
    let life_marker_sub_1 = TestStruct;
    let life_marker_sub_2 = TestStruct;

    let mut life_marker_or_1 = None;
    let mut life_marker_or_2 = None;

    // Error
    // let mut life_marker_or_1 = None;
    // let mut life_marker_or_2 = None;

    // let life_marker_sub_1 = TestStruct;
    // let life_marker_sub_2 = TestStruct;

    {
        let left_observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                life_marker_or_1 = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_1.consume_ref();
                }))
            });
        let right_observable =
            Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
                life_marker_or_2 = Some(observer);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker_sub_2.consume_ref();
                }))
            });
        let observable = left_observable
            .group_join(right_observable, |_| Never, |_| Never)
            .map(|(left, _)| left);

        let (_, observer) = Checker::new();
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let left_observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        DisposeOnDrop::default()
    });
    let right_observable =
        Create::shared_boxed(|_: SendBoxedObserver<'_, i32, Infallible>| DisposeOnDrop::default());
    let observable = left_observable.group_join(right_observable, |_| Never, |_| Never);
    _ = observable.clone(); // Make sure it's Clone when the left items are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, left_observable, _) = test_channel::<'_, i32, String>();
    let (_, right_observable, _) = test_channel::<'_, &str, _>();
    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Empty.with_error_type(),
    );

    let observable = observable.flat_map(|(_, window)| window);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, left_observable, _) = test_channel::<'_, i32, String>();
    let (_, right_observable, _) = test_channel::<'_, &str, _>();
    let observable = left_observable.group_join(
        right_observable,
        |_| Never.with_error_type(),
        |_| Empty.with_error_type(),
    );

    observable.flat_map(|(_, window)| window);
}
