mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::scheduler_ext::SchedulerExt;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use futures::executor::ThreadPool;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::scheduler::runtime::futures::ThreadPoolScheduler;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::combining::when::{Plans, When},
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next("222").is_continue());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

/// A source that completes with items queued keeps its plan alive until they are taken.
#[test]
fn test_completed_after_the_queue_drains() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next("222").is_stop());
    assert_eq!(checker.values(), [(111, "111"), (222, "222")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

/// An item of a source two plans share goes to the first plan, in their order, that it completes.
#[test]
fn test_completed_plans_share_a_source() {
    let (mut orders, orders_observable, orders_checker) = test_channel();
    let (mut cash, cash_observable, cash_checker) = test_channel();
    let (mut cards, cards_observable, cards_checker) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new(
        (orders_observable, cash_observable, cards_observable),
        |(order, cash, card)| {
            (
                order
                    .and(cash)
                    .then(|order, amount| format!("{order}: {amount} in cash")),
                order
                    .and(card)
                    .then(|order, number| format!("{order}: card {number}")),
            )
        },
    );

    let _subscription = observable.subscribe(observer);
    assert!(orders.on_next("tea").is_continue());
    assert!(cards.on_next(1234).is_continue());
    assert_eq!(checker.values(), ["tea: card 1234"]);

    assert!(cash.on_next(5).is_continue());
    assert!(orders.on_next("cake").is_continue());
    assert_eq!(checker.values(), ["tea: card 1234", "cake: 5 in cash"]);

    // Both plans could take the next order: the first one does.
    assert!(cards.on_next(5678).is_continue());
    assert!(cash.on_next(7).is_continue());
    assert!(orders.on_next("pie").is_continue());
    assert!(orders.on_next("tart").is_continue());
    assert_eq!(
        checker.values(),
        [
            "tea: card 1234",
            "cake: 5 in cash",
            "pie: 7 in cash",
            "tart: card 5678"
        ]
    );
    assert_eq!(checker.state(), State::Active);

    // The cash plan is gone, but the card plan still waits on the orders.
    cash.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(orders_checker.state(), ChannelState::Subscribed);
    assert_eq!(cards_checker.state(), ChannelState::Subscribed);

    assert!(orders.on_next("bun").is_continue());
    assert!(cards.on_next(9).is_continue());
    assert_eq!(checker.values().last().unwrap(), "bun: card 9");

    cards.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(orders_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(cash_checker.state(), ChannelState::Completed);
    assert_eq!(cards_checker.state(), ChannelState::Completed);
}

/// A plan that can no longer match retires, and a source no remaining plan uses is disposed,
/// while the other plans go on.
#[test]
fn test_completed_when_every_plan_is_gone() {
    let (mut sender_a, observable_a, checker_a) = test_channel::<'_, i32, Infallible>();
    let (mut sender_b, observable_b, checker_b) = test_channel::<'_, i32, Infallible>();
    let (mut sender_c, observable_c, checker_c) = test_channel::<'_, i32, Infallible>();
    let (mut sender_d, observable_d, checker_d) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new(
        (observable_a, observable_b, observable_c, observable_d),
        |(a, b, c, d)| (a.and(b).then(|a, b| a + b), c.and(d).then(|c, d| c * d)),
    );

    let _subscription = observable.subscribe(observer);
    assert!(sender_a.on_next(1).is_continue());
    assert!(sender_a.on_next(2).is_continue());
    assert!(sender_b.on_next(10).is_continue());
    assert_eq!(checker.values(), [11]);

    sender_b.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [11]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(checker_a.state(), ChannelState::Unsubscribed);
    assert_eq!(checker_b.state(), ChannelState::Completed);
    assert_eq!(checker_c.state(), ChannelState::Subscribed);
    assert_eq!(checker_d.state(), ChannelState::Subscribed);

    assert!(sender_c.on_next(3).is_continue());
    assert!(sender_d.on_next(4).is_continue());
    assert_eq!(checker.values(), [11, 12]);

    sender_d.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [11, 12]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(checker_c.state(), ChannelState::Unsubscribed);
    assert_eq!(checker_d.state(), ChannelState::Completed);
}

/// A plan that takes the last item of a completed source it shares leaves the other plan with
/// nothing to match either.
#[test]
fn test_completed_shared_source_drained_by_another_plan() {
    let (mut sender_a, observable_a, checker_a) = test_channel::<'_, i32, Infallible>();
    let (mut sender_b, observable_b, checker_b) = test_channel::<'_, i32, Infallible>();
    let (_sender_c, observable_c, checker_c) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable_a, observable_b, observable_c), |(a, b, c)| {
        (a.and(b).then(|a, b| a + b), a.and(c).then(|a, c| a - c))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender_a.on_next(1).is_continue());
    sender_a.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(checker_a.state(), ChannelState::Completed);

    assert!(sender_b.on_next(10).is_stop());
    assert_eq!(checker.values(), [11]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(checker_b.state(), ChannelState::Unsubscribed);
    assert_eq!(checker_c.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_unused_source_is_not_subscribed() {
    let (mut sender, observable, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (_, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1, observable_2), |(a, b, _)| {
        a.and(b).then(|a, b| a + b)
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);

    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [3]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Initialized);
}

/// Plans of one source each pass every item through, in the order they arrive.
#[test]
fn test_completed_plans_of_one_source() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        (a.then(|a| a.to_string()), b.then(|b| b.to_uppercase()))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next("a").is_continue());
    assert!(sender.on_next(2).is_continue());
    assert_eq!(checker.values(), ["1", "A", "2"]);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next("b").is_continue());
    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), ["1", "A", "2", "B"]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_3_sources() {
    let (mut sender, observable, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel::<'_, &str, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, bool, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1, observable_2), |(a, b, c)| {
        a.and(b).and(c).then(|a, b, c| (a, b, c))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next("a").is_continue());
    assert!(sender_1.on_next("b").is_continue());
    assert_eq!(checker.values(), []);

    assert!(sender_2.on_next(true).is_continue());
    assert_eq!(checker.values(), [(1, "a", true)]);

    assert!(sender.on_next(2).is_continue());
    sender_1.on_termination(Termination::Completed);
    assert!(sender_2.on_next(false).is_stop());
    assert_eq!(checker.values(), [(1, "a", true), (2, "b", false)]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

/// The largest shapes: six sources, a pattern of six, and six plans.
#[test]
fn test_completed_6_sources_and_6_plans() {
    let (checker, observer) = Checker::new();

    let observable = When::new(
        (
            Just::new(1),
            Just::new(2),
            Just::new(3),
            Just::new(4),
            Just::new(5),
            Just::new(6),
        ),
        |(a, b, c, d, e, f)| {
            (
                a.and(b)
                    .and(c)
                    .and(d)
                    .and(e)
                    .and(f)
                    .then(|a, b, c, d, e, f| a + b + c + d + e + f),
                f.then(|f| f * 100),
                e.and(f).then(|e, f| e * f),
                d.and(f).then(|d, f| d * f),
                c.and(f).then(|c, f| c * f),
                b.and(f).then(|b, f| b * f),
            )
        },
    );

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [21]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_6_plans_of_a_source_each() {
    let [
        (sender_0, source_0, _),
        (sender_1, source_1, _),
        (sender_2, source_2, _),
        (sender_3, source_3, _),
        (sender_4, source_4, _),
        (sender_5, source_5, _),
    ] = std::array::from_fn(|_| test_channel::<'_, _, Infallible>());
    let (checker, observer) = Checker::new();
    let observable = When::new(
        (source_0, source_1, source_2, source_3, source_4, source_5),
        |(a, b, c, d, e, f)| {
            (
                a.then(|value| value + 10),
                b.then(|value| value + 20),
                c.then(|value| value + 30),
                d.then(|value| value + 40),
                e.then(|value| value + 50),
                f.then(|value| value + 60),
            )
        },
    );
    let _subscription = observable.subscribe(observer);
    let mut senders = [sender_0, sender_1, sender_2, sender_3, sender_4, sender_5];
    let expected = [11, 21, 31, 41, 51, 61];
    for (index, sender) in senders.iter_mut().enumerate() {
        assert!(sender.on_next(1).is_continue());
        assert_eq!(checker.values(), expected[..=index]);
        assert_eq!(checker.state(), State::Active);
    }
    for sender in senders {
        sender.on_termination(Termination::Completed);
    }
    assert_eq!(checker.state(), State::Completed);
}

/// A queued match carries only one plan's arguments, even with six plans of large items.
#[test]
fn test_matched_size() {
    fn check<P: Plans>(_: &P) {
        // Each plan takes 2 KiB. Leave room for a tag and alignment, but not six payloads.
        assert!(std::mem::size_of::<P::Matched>() <= 4096);
    }

    let _observable = When::new(
        (Just::new([0_u8; 1024]), Just::new([0_u8; 1024])),
        |(a, b)| {
            let plans = (
                a.and(b).then(|_, _| ()),
                a.and(b).then(|_, _| ()),
                a.and(b).then(|_, _| ()),
                a.and(b).then(|_, _| ()),
                a.and(b).then(|_, _| ()),
                a.and(b).then(|_, _| ()),
            );
            check(&plans);
            plans
        },
    );
}

#[test]
fn test_error() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_from_another_source() {
    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender_1.on_next("111").is_continue());
    assert_eq!(checker.values(), [(111, "111")]);

    sender_1.on_termination(Termination::Error("error"));
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (channels, observable) = test_channels();
    let (channels_1, observable_1) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });
    let observable_2 = observable_1.clone();

    let subscription_1 = observable_1.subscribe(observer_1);
    let _subscription_2 = observable_2.subscribe(observer_2);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(channels_1.state(0), ChannelState::Subscribed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert!(channels_1.on_next(0, "111").is_continue());
    assert!(channels_1.on_next(1, "111").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_2.values(), [(111, "111")]);

    subscription_1.dispose();
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_1.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(checker_2.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(channels_1.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    assert!(channels_1.on_next(1, "222").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_2.values(), [(111, "111"), (222, "222")]);

    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels_1.state(1), ChannelState::Unsubscribed);
}

/// The sources are disposed in the opposite order to their subscription, the last one first.
#[test]
fn test_unsubscribe_last_source_first() {
    let disposed = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let disposed = disposed.clone();
        Create::shared_boxed(move |_: SendBoxedObserver<'_, i32, Infallible>| {
            let disposed = disposed.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                disposed.with_mut(|disposed| disposed.push(index));
            }))
        })
    };
    let (_, observer) = Checker::new();

    let observable = When::new((source(0), source(1), source(2)), |(a, b, c)| {
        a.and(b).and(c).then(|a, b, c| a + b + c)
    });

    let subscription = observable.subscribe(observer);
    subscription.dispose();
    assert_eq!(disposed.with_ref(|disposed| disposed.clone()), [2, 1, 0]);
}

/// The sources released as the last plan is gone are disposed in the same order as by a disposal,
/// the last one first.
#[test]
fn test_completed_last_source_first() {
    let disposed = Arc::new(Mutex::new(Vec::new()));
    let first_observer = Arc::new(Mutex::new(None));
    let source = |index: usize| {
        let disposed = disposed.clone();
        let first_observer = first_observer.clone();
        Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, Infallible>| {
            if index == 0 {
                first_observer.with_mut(|first_observer| *first_observer = Some(observer));
            }
            let disposed = disposed.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                disposed.with_mut(|disposed| disposed.push(index));
            }))
        })
    };
    let (checker, observer) = Checker::new();

    let observable = When::new((source(0), source(1), source(2)), |(a, b, c)| {
        a.and(b).and(c).then(|a, b, c| a + b + c)
    });

    let _subscription = observable.subscribe(observer);
    let first_observer = first_observer
        .with_mut(|first_observer| first_observer.take())
        .unwrap();
    first_observer.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(disposed.with_ref(|disposed| disposed.clone()), [2, 1, 0]);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;

    let (mut sender, observable, channel_checker) = test_channel();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(&value_1).is_continue());
    assert!(sender_1.on_next(&value_2).is_continue());
    assert_eq!(checker.values(), [(&value_1, &value_2)]);

    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.values(), [(&value_1, &value_2)]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channel_checker.state(), ChannelState::Error(&error));
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let mut value_3 = 333;

    let (mut sender, observable, _) = test_channel::<'_, &mut i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel::<'_, &mut i32, _>();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a: &mut i32, b: &mut i32| {
            *a *= 2;
            *b *= 3;
        })
    });

    let subscription = observable.subscribe_with_callback(|()| {}, |_| unreachable!());

    assert!(sender.on_next(&mut value_1).is_continue());
    assert!(sender.on_next(&mut value_2).is_continue());
    assert!(sender_1.on_next(&mut value_3).is_continue());

    drop(subscription);
    drop(sender);
    drop(sender_1);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 222);
    assert_eq!(value_3, 999);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
        let (mut sender_1, observable_1, channel_checker_1) = test_channel();
        let (checker, observer) = Checker::new();

        let observable = When::new((observable, observable_1), |(a, b)| {
            a.and(b).then(|a, b| (a, b))
        });

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert_eq!(channel_checker.state(), ChannelState::Subscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

        let _sender = scheduler
            .spawn(async move {
                assert!(sender.on_next(111).is_continue());
                sender
            })
            .await;
        assert_eq!(checker.values(), []);

        let _sender_1 = scheduler
            .spawn(async move {
                assert!(sender_1.on_next("111").is_continue());
                sender_1
            })
            .await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        assert_eq!(checker.values(), [(111, "111")]);
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
        assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
    });
}

/// A selector and its downstream can both send to other sources while a match is delivered.
/// Their matches follow the running match, in the order they were accepted.
#[test]
fn test_next_on_next() {
    let (mut sender_a, source_a, checker_a) = test_channel::<'_, i32, Infallible>();
    let (mut sender_b, source_b, checker_b) = test_channel();
    let (mut sender_c, source_c, checker_c) = test_channel();
    let (mut sender_d, source_d, checker_d) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((source_a, source_b, source_c, source_d), |(a, b, c, d)| {
        (
            a.and(b).then(move |a, b| {
                assert!(sender_c.on_next(30).is_continue());
                a + b
            }),
            c.then(|c| c),
            d.then(|d| d),
        )
    })
    .hook_on_next(move |downstream, value| {
        if value == 3 {
            assert!(sender_d.on_next(40).is_continue());
        }
        downstream.on_next(value)
    });

    let subscription = observable.subscribe(observer);
    assert!(sender_a.on_next(1).is_continue());
    assert!(sender_b.on_next(2).is_continue());
    assert_eq!(checker.values(), [3, 30, 40]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker.state(), State::Dropped);
    for source in [checker_a, checker_b, checker_c, checker_d] {
        assert_eq!(source.state(), ChannelState::Unsubscribed);
    }
}

/// Releasing a plan drops its buffered items outside the context lock. Their drops see the
/// retired source rejected already, while an independent plan can still accept another value.
#[test]
fn test_drop_queued_value_outside_lock() {
    let sender_a = Arc::new(Mutex::new(
        None::<SendBoxedObserver<'_, DropProbe, Infallible>>,
    ));
    let sender_a_on_sub = sender_a.clone();
    let disposed = DropCount::new();
    let disposal = disposed.clone();
    let source_a = Create::shared_boxed(move |observer| {
        sender_a_on_sub.replace_value(Some(observer));
        DisposeOnDrop::new(CallbackDisposal::new(move || disposal.increment()))
    });
    let (sender_b, source_b, checker_b) = test_channel::<'_, (), Infallible>();
    let (mut sender_c, source_c, checker_c) = test_channel();
    let (checker, observer) = Checker::new();
    let observable = When::new((source_a, source_b, source_c), |(a, b, c)| {
        (a.and(b).then(|_, ()| 0), c.then(|c| c))
    });
    let _subscription = observable.subscribe(observer);
    let drops = DropCount::new();
    let sender_a_on_drop = sender_a.clone();
    let value = DropProbe::new()
        .on_drop(drops.callback())
        .on_drop(Box::new(move || {
            let mut sender = sender_a_on_drop.take_value().unwrap();
            assert!(sender.on_next(DropProbe::new()).is_stop());
            assert!(sender_c.on_next(7).is_continue());
        }));
    let mut sender = sender_a.take_value().unwrap();
    assert!(sender.on_next(value).is_continue());
    sender_a.replace_value(Some(sender));

    sender_b.on_termination(Termination::Completed);
    assert_eq!(drops.get(), 1);
    assert_eq!(disposed.get(), 1);
    assert!(sender_a.with_ref(Option::is_none));
    assert_eq!(checker_b.state(), ChannelState::Completed);
    assert_eq!(checker_c.state(), ChannelState::Subscribed);
    assert_eq!(checker.values(), [7]);
    assert_eq!(checker.state(), State::Active);
}

/// Two sources send concurrently: each pair contains the same ordinal, exactly once, before
/// completion, whichever source has accumulated the larger queue.
#[test]
fn test_race_condition() {
    const VALUES: usize = 100;
    const TIMEOUT: Duration = Duration::from_secs(5);
    let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());

    for _ in 0..100 {
        let (mut sender_a, source_a, checker_a) = test_channel::<'_, _, Infallible>();
        let (mut sender_b, source_b, checker_b) = test_channel();
        let (checker, observer) = Checker::new();
        let (finished, finish) = mpsc::channel();
        let observable = When::new((source_a, source_b), |(a, b)| a.and(b).then(|a, b| (a, b)));
        let _subscription = observable.subscribe(observer);
        let _task = scheduler.schedule(
            move || {
                for value in 0..VALUES {
                    assert!(sender_a.on_next(value).is_continue());
                }
                sender_a.on_termination(Termination::Completed);
                finished.send(()).unwrap();
            },
            None,
        );

        for value in 0..VALUES {
            if sender_b.on_next(value).is_stop() {
                assert_eq!(value, VALUES - 1);
            }
        }
        finish.recv_timeout(TIMEOUT).expect("both sources finish");
        assert_eq!(
            checker.values(),
            (0..VALUES).map(|n| (n, n)).collect::<Vec<_>>()
        );
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(checker_a.state(), ChannelState::Completed);
        assert_eq!(checker_b.state(), ChannelState::Unsubscribed);
    }
}

/// Both sources may queue matches while another thread delivers the match that stops the
/// observer. Queued matches and sends after `Stop` must never reach it or terminate it.
#[test]
fn test_race_condition_stop_on_next() {
    const VALUES: usize = 100;
    const TAKEN: usize = 3;
    const TIMEOUT: Duration = Duration::from_secs(5);
    let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());

    for _ in 0..100 {
        let mut sender_a = None;
        let mut sender_b = None;
        let source_a = Create::shared_boxed(|observer: SendBoxedObserver<'_, _, Infallible>| {
            sender_a = Some(observer);
            DisposeOnDrop::default()
        });
        let source_b = Create::shared_boxed(|observer| {
            sender_b = Some(observer);
            DisposeOnDrop::default()
        });
        let (checker, observer) = Checker::stopping_after(TAKEN);
        let (finished, finish) = mpsc::channel();
        let observable = When::new((source_a, source_b), |(a, b)| a.and(b).then(|a, b| (a, b)));
        let _subscription = observable.subscribe(observer);
        let mut sender_a = sender_a.unwrap();
        let mut sender_b = sender_b.unwrap();
        let _task = scheduler.schedule(
            move || {
                for value in 0..VALUES {
                    if sender_a.on_next(value).is_stop() {
                        break;
                    }
                }
                finished.send(sender_a).unwrap();
            },
            None,
        );
        for value in 0..VALUES {
            if sender_b.on_next(value).is_stop() {
                break;
            }
        }
        let mut sender_a = finish.recv_timeout(TIMEOUT).expect("both sources finish");
        assert!(sender_a.on_next(VALUES).is_stop());
        assert!(sender_b.on_next(VALUES).is_stop());
        sender_a.on_termination(Termination::Completed);
        sender_b.on_termination(Termination::Completed);
        assert_eq!(
            checker.values(),
            (0..TAKEN).map(|n| (n, n)).collect::<Vec<_>>()
        );
        assert_eq!(checker.state(), State::Dropped);
    }
}

#[test]
fn test_subscribe_by_different_observer() {
    let (channels, observable) = test_channels();
    let (channels_1, observable_1) = test_channels();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert!(channels.on_next(0, 111).is_continue());
    assert!(channels.on_next(1, 111).is_continue());
    assert!(channels_1.on_next(0, "111").is_continue());
    assert!(channels_1.on_next(1, "111").is_continue());
    assert_eq!(checker_1.values(), [(111, "111")]);
    assert_eq!(checker_2.values(), [(111, "111")]);

    channels.on_termination(0, Termination::<Infallible>::Completed);
    channels.on_termination(1, Termination::<Infallible>::Completed);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.state(), State::Completed);
    assert_eq!(channels_1.state(0), ChannelState::Unsubscribed);
    assert_eq!(channels_1.state(1), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    })
    .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert!(sender.on_next(222).is_continue());

    assert!(sender_1.on_next("111").is_stop());
    assert_eq!(checker.values(), [(111, "111")]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_stop_on_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, observable_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::stopping_after(1);

    let observable = When::new((observable, observable_1), |(a, b)| {
        (a.and(b).then(|a, b| a + b), b.then(|b| b * 100))
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(10).is_stop());
    assert_eq!(checker.values(), [11]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsub_in_result_selector() {
    let (mut sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, source_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_in_selector = subscription.clone();
    let observable = When::new((source, source_1), |(a, b)| {
        a.and(b).then(move |a: i32, b: i32| {
            subscription_in_selector.take_value().unwrap().dispose();
            a + b
        })
    });

    subscription.replace_value(Some(observable.subscribe(observer)));
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_stop());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Unsubscribed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

/// Cancelling the last match also suppresses the completion already queued behind it.
#[test]
fn test_unsub_in_result_selector_with_pending_termination() {
    let (mut sender, source, channel_checker) = test_channel::<'_, _, Infallible>();
    let (mut sender_1, source_1, channel_checker_1) = test_channel();
    let (checker, observer) = Checker::new();
    let subscription = Arc::new(Mutex::new(None::<DisposeOnDrop<_>>));
    let subscription_in_selector = subscription.clone();
    let observable = When::new((source, source_1), |(a, b)| {
        (
            a.and(b).then(move |a: i32, b: i32| {
                subscription_in_selector.take_value().unwrap().dispose();
                a + b
            }),
            a.and(b).then(|a, b| a - b),
        )
    });

    subscription.replace_value(Some(observable.subscribe(observer)));
    assert!(sender.on_next(1).is_continue());
    sender.on_termination(Termination::Completed);
    assert_eq!(checker.state(), State::Active);
    assert!(sender_1.on_next(2).is_stop());
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_multiple_operation() {
    let (mut sender, observable, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = When::new(
        (
            observable.map(|value| value * 10),
            observable_1,
            observable_2,
        ),
        |(a, b, c)| (a.and(b).then(|a, b| a + b), a.and(c).then(|a, c| a - c)),
    )
    .filter(|value| *value > 0)
    .map(|value| value.to_string());

    let _subscription = observable.subscribe(observer);
    assert!(sender_2.on_next(100).is_continue());
    assert!(sender.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_continue());
    assert!(sender.on_next(3).is_continue());
    assert_eq!(checker.values(), ["32"]);

    assert!(sender_2.on_next(1).is_continue());
    assert!(sender.on_next(4).is_continue());
    assert_eq!(checker.values(), ["32", "39"]);
}

#[test]
fn test_next_on_sub() {
    let (mut sender, source, _) = test_channel();
    let source = source.start_with([111]);
    let (mut sender_1, source_1, channel_checker_1) = test_channel();
    let source_1 = source_1.start_with([111]);
    let (checker, observer) = Checker::new();

    let observable = When::new((source, source_1), |(a, b)| a.and(b).then(|a, b| (a, b)));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [(111, 111)]);
    assert_eq!(checker.state(), State::Active);

    assert!(sender_1.on_next(222).is_continue());
    assert!(sender.on_next(222).is_continue());
    assert_eq!(checker.values(), [(111, 111), (222, 222)]);

    sender.on_termination(Termination::<Infallible>::Completed);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = When::new((Empty, observable), |(a, b)| a.and(b).then(|_, b| b));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, _>();
    let (checker, observer) = Checker::new();

    let observable = When::new((Throw::new("error"), observable), |(a, b)| {
        a.and(b).then(|_, b| b)
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), []);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. It must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_stop());
        }))
    });

    let observable = When::new((observable, observable_1), |(a, b)| {
        (a.then(|a| a), b.then(|b| b))
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_complete_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The termination must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, Infallible>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| a + b)
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The error must be dropped instead of reaching the observer.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| a + b)
    });

    let subscription = observable.subscribe(observer);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

/// A source whose last plan is dropped while it is being subscribed is told to stop, what it sends
/// then is rejected, and its subscription is disposed as soon as it is handed over, while the
/// other plans go on.
#[test]
fn test_next_on_sub_after_release() {
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();
    let flows = Arc::new(Mutex::new(Vec::new()));
    let flows_of_source = flows.clone();
    let disposed = Arc::new(Mutex::new(false));
    let disposed_by_source = disposed.clone();

    let observable_1 = Create::shared_boxed(
        move |mut observer: SendBoxedObserver<'_, i32, Infallible>| {
            // Takes the last item of the completed first source: the plan is gone.
            let matched = observer.on_next(2);
            let rejected = observer.on_next(3);
            flows_of_source.with_mut(|flows| flows.extend([matched, rejected]));
            let disposed = disposed_by_source.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                disposed.with_mut(|disposed| *disposed = true);
            }))
        },
    );
    let observable = When::new((Just::new(1), observable_1, observable_2), |(a, b, c)| {
        (a.and(b).then(|a, b| a + b), c.then(|c| c * 100))
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(
        flows.with_ref(|flows| flows.iter().map(|flow| flow.is_stop()).collect::<Vec<_>>()),
        [true, true]
    );
    assert!(disposed.with_ref(|disposed| *disposed));
    assert_eq!(checker.values(), [3]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(4).is_continue());
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [3, 400]);
    assert_eq!(checker.state(), State::Completed);
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
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(111).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_1.consume_ref();
            }))
        });
        let observable_1 = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(()).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker_2.consume_ref();
            }))
        });
        let observable = When::new((observable, observable_1), |(a, b)| {
            a.and(b).then(|a, b| (a, b))
        });

        let (_, observer) = Checker::<_, ()>::new();
        _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_lifetime_or() {
    // OK
    let life_marker_3 = TestStruct;
    let mut life_marker_1 = None;
    let mut life_marker_2 = None;

    // Error
    // let mut life_marker_1 = None;
    // let mut life_marker_2 = None;
    // let life_marker_3 = TestStruct;

    {
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable_1 = Create::shared_boxed(|observer| {
            life_marker_2 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = When::new((observable, observable_1), |(a, b)| {
            a.and(b).then(|a, b| (a, b))
        });

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(
            observer
                .on_next((&life_marker_3, &life_marker_3))
                .is_continue()
        );
        let _subscription = observable.subscribe(observer);
    }
}

#[test]
fn test_clone() {
    let observable = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(TestStruct).is_continue());
        observer.on_termination(Termination::Error(TestStruct));
        DisposeOnDrop::default()
    });
    let observable_1 =
        Create::shared_boxed(|_: SendBoxedObserver<'_, (), TestStruct>| DisposeOnDrop::default());
    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    });
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, observable_1, _) = test_channel();
    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, b: String| a.to_string() + &b)
    });

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, i32, String>();
    let (_, observable_1, _) = test_channel::<'_, (), _>();
    let observable = When::new((observable, observable_1), |(a, b)| {
        a.and(b).then(|a, ()| a)
    });

    observable.filter(|_| true);
}

/// A source that drops its observer without a termination (decision 0004) sends nothing more, but
/// the values it already sent still match those of the other one. Once the other lets go too,
/// nothing more can come, and the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = When::new((observable_1, observable_2), |(a, b)| {
        a.and(b).then(|a, b| (a, b))
    })
    .subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_1.on_next(2).is_continue());
    sender_1.abandon();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Abandoned);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);

    assert!(sender_2.on_next(10).is_continue());
    assert!(sender_2.on_next(20).is_continue());
    assert_eq!(checker.values(), [(1, 10), (2, 20)]);
    assert_eq!(checker.state(), State::Active);

    sender_2.abandon();
    assert_eq!(checker.values(), [(1, 10), (2, 20)]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_2.state(), ChannelState::Abandoned);
}
