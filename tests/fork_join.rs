mod tests_utils;

use crate::tests_utils::DURATION_10_MS;
use crate::tests_utils::checker::State;
use crate::tests_utils::drop_probe::{DropCount, DropProbe};
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::test_channel;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::operators::creating::just::Just;
use rx_rust::operators::creating::throw::Throw;
use rx_rust::thread_mode::mutable::{MutableExt, MutableHelper};
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::combining::fork_join::ForkJoin,
};
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_struct::TestStruct};

#[test]
fn test_completed() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, &str, Infallible>();
    let (mut sender_3, observable_3, channel_checker_3) = test_channel::<'_, f64, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = ForkJoin::new((observable_1, observable_2, observable_3));

    let _subscription = observable.subscribe(observer);
    assert_eq!(channel_checker_1.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_2.state(), ChannelState::Subscribed);
    assert_eq!(channel_checker_3.state(), ChannelState::Subscribed);

    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_2.on_next("a").is_continue());
    assert!(sender_3.on_next(0.5).is_continue());
    assert!(sender_1.on_next(10).is_continue());
    assert!(checker.values().is_empty());

    sender_3.on_termination(Termination::Completed);
    sender_1.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    // The last one completes: the last item of each, in the order of the tuple.
    assert!(sender_2.on_next("b").is_continue());
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(10, "b", 0.5)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_without_value() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (sender_2, observable_2, _) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.fork_join(observable_2).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());

    // No result to report for this one: the stream completes at once, with no value.
    sender_2.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker_1.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_completed_single() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let _subscription = ForkJoin::new((FromIter::new([1, 2]),)).subscribe(observer);
    assert_eq!(checker.values(), [(2,)]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_completed_eight_sources() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let observable = ForkJoin::new((
        Just::new(1),
        Just::new("two"),
        Just::new(3.0),
        Just::new('4'),
        Just::new(5u8),
        Just::new(String::from("six")),
        Just::new(7_i64),
        Just::new(true),
    ));

    let _subscription = observable.subscribe(observer);
    assert_eq!(
        checker.values(),
        [(1, "two", 3.0, '4', 5u8, String::from("six"), 7_i64, true)]
    );
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, &str>();
    let (sender_2, observable_2, channel_checker_2) = test_channel::<'_, &str, &str>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.fork_join(observable_2).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Error("error"));
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_2.state(), ChannelState::Error("error"));
}

#[test]
fn test_unsubscribe() {
    let (mut sender_1, observable_1, channel_checker_1) = test_channel::<'_, i32, Infallible>();
    let (_sender_2, observable_2, channel_checker_2) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let subscription = observable_1.fork_join(observable_2).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    sender_1.on_termination(Termination::Completed);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Completed);
    assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
}

#[test]
fn test_unsubscribe_order() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let source = |index: usize| {
        let order = order.clone();
        Create::shared_boxed(move |_observer: SendBoxedObserver<'_, i32, Infallible>| {
            let order = order.clone();
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                order.with_mut(|order| order.push(index));
            }))
        })
    };

    let subscription =
        ForkJoin::new((source(0), source(1), source(2))).subscribe_with_callback(|_| {}, |_| {});
    subscription.dispose();
    // The last subscribed first.
    assert_eq!(order.clone_value(), [2, 1, 0]);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = "222";
    let error = 333;

    let (mut sender_1, observable_1, _) = test_channel::<'_, &i32, &i32>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, &&str, &i32>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.fork_join(observable_2).subscribe(observer);
    assert!(sender_1.on_next(&value_1).is_continue());
    assert!(sender_2.on_next(&value_2).is_continue());
    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(&value_1, &value_2)]);
    assert_eq!(checker.state(), State::Completed);

    let (sender, observable, _) = test_channel::<'_, &i32, &i32>();
    let (checker, observer) = Checker::new();
    let _subscription = ForkJoin::new((observable,)).subscribe(observer);
    sender.on_termination(Termination::Error(&error));
    assert_eq!(checker.state(), State::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222.0;

    let observable_1 = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value_1).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    });
    let observable_2 = Create::shared_boxed(|mut observer| {
        assert!(observer.on_next(&mut value_2).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    });

    // Not cloned: the items are handed over, `&mut` ones included.
    let subscription = observable_1
        .fork_join(observable_2)
        .subscribe_with_callback(
            |(value_1, value_2)| {
                *value_1 *= 2;
                *value_2 *= 2.0;
            },
            |_| {},
        );
    subscription.dispose();
    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444.0);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
        let (mut sender_2, observable_2, channel_checker_2) = test_channel::<'_, &str, _>();
        let (checker, observer) = Checker::new();

        let observable = observable_1.fork_join(observable_2);

        let subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;

        let _sender_2 = scheduler
            .spawn(async move {
                assert!(sender_1.on_next(1).is_continue());
                sender_1.on_termination(Termination::Completed);
                assert!(sender_2.on_next("a").is_continue());
                sender_2
            })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        scheduler.spawn(async { subscription.dispose() }).await;
        scheduler.sleep(DURATION_10_MS).await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channel_checker_2.state(), ChannelState::Unsubscribed);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let (checker_1, observer_1) = Checker::<_, Infallible>::new();
    let (checker_2, observer_2) = Checker::new();

    let observable_1 = ForkJoin::new((FromIter::new([1, 2]), Just::new("a")));
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

    assert_eq!(checker_1.values(), [(2, "a")]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [(2, "a")]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let observable = observable_1.fork_join(observable_2).take(1);

    let _subscription = observable.subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    assert!(sender_2.on_next("a").is_continue());
    sender_1.on_termination(Termination::Completed);
    sender_2.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_stop_on_next() {
    let (checker, observer) = Checker::<_, Infallible>::stopping_after(1);

    // The result is the last value of the stream, and downstream stops on it: it is not completed
    // on top of that.
    let _subscription = ForkJoin::new((Just::new(1), Just::new("a"))).subscribe(observer);
    assert_eq!(checker.values(), [(1, "a")]);
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_multiple_operation() {
    let (checker, observer) = Checker::<_, Infallible>::new();

    let joined = ForkJoin::new((FromIter::new([1, 2]), Just::new(3)))
        .map(|(value_1, value_2)| value_1 + value_2);
    let observable = joined.fork_join(Just::new("five"));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), [(5, "five")]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::<(Infallible, i32), Infallible>::new();

    let _subscription = ForkJoin::new((Empty, Just::new(1))).subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_error_on_sub() {
    let (_, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The first one fails as it is subscribed: the second is not subscribed at all.
    let _subscription = ForkJoin::new((
        Throw::new("error").with_item_type::<&str>().into_shared(),
        observable,
    ))
    .subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_unsub() {
    let (checker, observer) = Checker::<(i32,), Infallible>::new();

    let observable =
        Create::shared_boxed(|mut observer: SendBoxedObserver<'_, i32, Infallible>| {
            assert!(observer.on_next(1).is_continue());
            DisposeOnDrop::new(CallbackDisposal::new(move || {
                observer.on_termination(Termination::Completed);
            }))
        });

    // The completion arrives from the disposal, once the stream is over: no result is emitted.
    let subscription = ForkJoin::new((observable,)).subscribe(observer);
    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_abandon_after_next() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, &str, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1.fork_join(observable_2).subscribe(observer);
    assert!(sender_1.on_next(1).is_continue());
    // Letting go is not a completion: no result can be reported for it, and none is.
    sender_1.abandon();
    assert!(sender_2.on_next("a").is_continue());
    sender_2.on_termination(Termination::Completed);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_drop_replaced_value_outside_lock() {
    let (mut sender_1, observable_1, _) = test_channel::<'_, DropProbe, Infallible>();
    let (mut sender_2, observable_2, _) = test_channel::<'_, i32, Infallible>();
    let (checker, observer) = Checker::new();

    let _subscription = observable_1
        .fork_join(observable_2)
        .map(|(_, value)| value)
        .subscribe(observer);

    // The item a newer one replaces is dropped outside the lock: a drop that re-enters the operator
    // would deadlock otherwise, or panic on the re-entrancy check of a debug build.
    let drops = DropCount::new();
    let probe = DropProbe::new()
        .on_drop(drops.callback())
        .on_drop(Box::new(move || {
            assert!(sender_2.on_next(222).is_continue());
            sender_2.on_termination(Termination::Completed);
        }));
    assert!(sender_1.on_next(probe).is_continue());
    assert!(sender_1.on_next(DropProbe::new()).is_continue());
    assert_eq!(drops.get(), 1);

    sender_1.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_lifetime_sub() {
    // OK
    let life_marker = TestStruct;
    let _subscription;

    // Error
    // let _subscription;
    // let life_marker = TestStruct;

    {
        let observable = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(1).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });

        let observable = ForkJoin::new((observable,));

        let (_, observer) = Checker::new();
        _subscription = observable.subscribe(observer);
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
        let observable = Create::shared_boxed(|observer| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = ForkJoin::new((observable,));

        let (_, mut observer) = Checker::<_, Infallible>::new();
        assert!(observer.on_next((Some(&life_marker_2),)).is_continue());
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
    let observable = ForkJoin::new((observable.clone(), observable));
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let observable = ForkJoin::new((Just::new(1), Just::new("a")));

    let observable = ForkJoin::new((observable, Just::new(2.0)));
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let observable = ForkJoin::new((Just::new(1), Just::new("a")));

    ForkJoin::new((observable, Just::new(2.0)));
}

#[test]
fn test_observable_types_without_clone() {
    use rx_rust::observable::ObservableTypes;

    struct NotClone;

    fn inspect_tuple<OE: ObservableTypes<Item = (NotClone, bool)>>(_: OE) {}
    fn inspect_list<OE: ObservableTypes<Item = Vec<NotClone>>>(_: OE) {}

    inspect_tuple(ForkJoin::new((Just::new(NotClone), Just::new(true))));
    inspect_list(ForkJoin::new_from_iter([Just::new(NotClone)]));
}

#[test]
fn test_completed_from_iter() {
    struct NotClone(i32);
    let mut values = Vec::new();
    let subscription =
        ForkJoin::new_from_iter((0..12).map(|value| Just::new(NotClone(value)).into_shared()))
            .subscribe_with_callback(
                |items| values.extend(items.into_iter().map(|item| item.0)),
                |termination| assert_eq!(termination, Termination::Completed),
            );
    drop(subscription);
    assert_eq!(values, (0..12).collect::<Vec<_>>());
}

/// The list form, [`ForkJoin::new_from_iter`].
mod from_iter {
    use crate::tests_utils::DURATION_10_MS;
    use crate::tests_utils::checker::State;
    use crate::tests_utils::drop_probe::{DropCount, DropProbe};
    use crate::tests_utils::test_channel::ChannelState;
    use crate::tests_utils::test_channel::test_channel;
    use crate::tests_utils::test_channel::test_channels;
    use crate::tests_utils::test_scheduler::block_on;
    use crate::tests_utils::{checker::Checker, test_struct::TestStruct};
    use rx_rust::disposable::Disposable;
    use rx_rust::disposable::callback_disposal::CallbackDisposal;
    use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
    use rx_rust::observer::boxed_observer::SendBoxedObserver;
    use rx_rust::operators::creating::create::Create;
    use rx_rust::operators::creating::empty::Empty;
    use rx_rust::operators::creating::from_iter::FromIter;
    use rx_rust::operators::creating::just::Just;
    use rx_rust::operators::creating::throw::Throw;
    use rx_rust::{
        observable::{Observable, ObservableExt},
        observer::{Observer, Termination},
        operators::combining::fork_join::ForkJoin,
    };
    use std::convert::Infallible;

    #[test]
    fn test_completed() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = ForkJoin::new_from_iter(vec![observable; 3]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(channels.len(), 3);

        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());
        assert!(channels.on_next(2, 3).is_continue());
        assert!(channels.on_next(0, 10).is_continue());
        assert!(checker.values().is_empty());

        channels.on_termination(2, Termination::Completed);
        channels.on_termination(0, Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);

        // The last one completes: the last item of each, in the order of the list.
        assert!(channels.on_next(1, 20).is_continue());
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [vec![10, 20, 3]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_completed_without_value() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());

        // No result to report for this one: the stream completes at once, with no value.
        channels.on_termination(1, Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_completed_empty() {
        let (checker, observer) = Checker::<Vec<i32>, Infallible>::new();

        let _subscription = ForkJoin::new_from_iter(Vec::<Just<i32>>::new()).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error() {
        let (channels, observable) = test_channels::<'_, i32, _>();
        let (checker, observer) = Checker::new();

        let _subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        channels.on_termination(0, Termination::Completed);
        channels.on_termination(1, Termination::Error("error"));
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
    }

    #[test]
    fn test_unsubscribe() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        channels.on_termination(0, Termination::Completed);

        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
        assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    }

    #[test]
    fn test_ref() {
        let values = [111, 222];
        let error = 333;

        let (channels, observable) = test_channels::<'_, _, &i32>();
        let (checker, observer) = Checker::new();

        let _subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, &values[0]).is_continue());
        assert!(channels.on_next(1, &values[1]).is_continue());
        channels.on_termination(0, Termination::Completed);
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [vec![&values[0], &values[1]]]);
        assert_eq!(checker.state(), State::Completed);

        let (channels, observable) = test_channels::<'_, &i32, _>();
        let (checker, observer) = Checker::new();
        let _subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        channels.on_termination(0, Termination::Error(&error));
        assert_eq!(checker.state(), State::Error(&error));
    }

    #[test]
    fn test_mut_ref() {
        let mut value_1 = 111;
        let mut value_2 = 222;

        let observable_1 = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(&mut value_1).is_continue());
            observer.on_termination(Termination::<Infallible>::Completed);
            DisposeOnDrop::default()
        });
        let observable_2 = Create::shared_boxed(|mut observer| {
            assert!(observer.on_next(&mut value_2).is_continue());
            observer.on_termination(Termination::<Infallible>::Completed);
            DisposeOnDrop::default()
        });

        let subscription =
            ForkJoin::new_from_iter([observable_1.into_boxed(), observable_2.into_boxed()])
                .subscribe_with_callback(
                    |values| values.into_iter().for_each(|value| *value *= 2),
                    |_| {},
                );
        subscription.dispose();
        assert_eq!(value_1, 222);
        assert_eq!(value_2, 444);
    }

    #[test]
    fn test_async() {
        block_on(|scheduler| async move {
            let (channels, observable) = test_channels::<'_, i32, Infallible>();
            let (checker, observer) = Checker::new();

            let observable = ForkJoin::new_from_iter(vec![observable; 2]);

            let subscription = scheduler
                .spawn(async move { observable.subscribe(observer) })
                .await;

            let channels_sender = channels.clone();
            scheduler
                .spawn(async move {
                    assert!(channels_sender.on_next(0, 1).is_continue());
                    channels_sender.on_termination(0, Termination::Completed);
                    assert!(channels_sender.on_next(1, 2).is_continue());
                })
                .await;
            assert!(checker.values().is_empty());
            assert_eq!(checker.state(), State::Active);

            scheduler.spawn(async { subscription.dispose() }).await;
            scheduler.sleep(DURATION_10_MS).await;
            assert!(checker.values().is_empty());
            assert_eq!(checker.state(), State::Dropped);
            assert_eq!(channels.state(1), ChannelState::Unsubscribed);
        });
    }

    #[test]
    fn test_subscribe_by_different_observer() {
        let (channels, observable) = test_channels::<'_, i32, &str>();
        let (checker_1, observer_1) = Checker::new();
        let (checker_2, observer_2) = Checker::new();

        let observable_1 = ForkJoin::new_from_iter(vec![observable; 2]);
        let observable_2 = observable_1.clone();

        let _subscription_1 = observable_1.subscribe(observer_1);
        let (on_next, on_termination) = observer_2.into_callbacks();
        let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);

        for index in 0..4 {
            assert!(channels.on_next(index, index as i32).is_continue());
            channels.on_termination(index, Termination::Completed);
        }
        assert_eq!(checker_1.values(), [vec![0, 1]]);
        assert_eq!(checker_1.state(), State::Completed);
        assert_eq!(checker_2.values(), [vec![2, 3]]);
        assert_eq!(checker_2.state(), State::Completed);
    }

    #[test]
    fn test_unsub_on_next_by_take() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let observable = ForkJoin::new_from_iter(vec![observable; 2]).take(1);

        let _subscription = observable.subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());
        channels.on_termination(0, Termination::Completed);
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_stop_on_next() {
        let (channels, observable) = test_channels::<'_, i32, Infallible>();
        let (checker, observer) = Checker::stopping_after(1);

        let _subscription = ForkJoin::new_from_iter(vec![observable; 2]).subscribe(observer);
        assert!(channels.on_next(0, 1).is_continue());
        assert!(channels.on_next(1, 2).is_continue());
        channels.on_termination(0, Termination::Completed);
        // The result is the last value of the stream, and downstream stops on it: it is not completed
        // on top of that.
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [vec![1, 2]]);
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_multiple_operation() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let joined =
            ForkJoin::new_from_iter([FromIter::new(vec![1, 2]), FromIter::new(vec![3, 4])])
                .map(|values| values.iter().sum::<i32>());
        let observable = ForkJoin::new_from_iter([joined.clone(), joined]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![6, 6]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_next_on_sub() {
        let (checker, observer) = Checker::<_, Infallible>::new();

        let observable =
            ForkJoin::new_from_iter([FromIter::new(vec![1, 2]), FromIter::new(vec![3])]);

        let _subscription = observable.subscribe(observer);
        assert_eq!(checker.values(), [vec![2, 3]]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_complete_on_sub() {
        let (checker, observer) = Checker::<Vec<Infallible>, Infallible>::new();

        let _subscription = ForkJoin::new_from_iter([Empty, Empty]).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_error_on_sub() {
        let (channels, observable) = test_channels::<'_, i32, &str>();
        let (checker, observer) = Checker::new();

        let sources = [
            Throw::new("error")
                .with_item_type()
                .into_shared()
                .into_boxed(),
            observable.into_boxed(),
        ];

        let _subscription = ForkJoin::new_from_iter(sources).subscribe(observer);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Error("error"));
        assert_eq!(channels.len(), 0);
    }

    #[test]
    fn test_complete_on_unsub() {
        let (checker, observer) = Checker::<Vec<i32>, Infallible>::new();

        let observable =
            Create::shared_boxed(|mut observer: SendBoxedObserver<'_, i32, Infallible>| {
                assert!(observer.on_next(1).is_continue());
                DisposeOnDrop::new(CallbackDisposal::new(move || {
                    observer.on_termination(Termination::Completed);
                }))
            });

        // The completion arrives from the disposal, once the stream is over: no result is emitted.
        let subscription = ForkJoin::new_from_iter([observable]).subscribe(observer);
        subscription.dispose();
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_abandon_after_next() {
        let (mut sender_1, observable_1, _) = test_channel::<'_, i32, Infallible>();
        let (mut sender_2, observable_2, _) = test_channel::<'_, i32, Infallible>();
        let (checker, observer) = Checker::new();

        let _subscription =
            ForkJoin::new_from_iter([observable_1, observable_2]).subscribe(observer);
        assert!(sender_1.on_next(1).is_continue());
        // Letting go is not a completion: no result can be reported for it, and none is.
        sender_1.abandon();
        assert!(sender_2.on_next(2).is_continue());
        sender_2.on_termination(Termination::Completed);
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Dropped);
    }

    #[test]
    fn test_drop_replaced_value_outside_lock() {
        let (channels, observable) = test_channels::<'_, DropProbe, Infallible>();
        let (checker, observer) = Checker::new();
        let channels_probe = channels.clone();

        let _subscription = ForkJoin::new_from_iter(vec![observable; 2])
            .map(|values| values.len())
            .subscribe(observer);

        // The item a newer one replaces is dropped outside the lock: a drop that re-enters the operator
        // would deadlock otherwise, or panic on the re-entrancy check of a debug build.
        let drops = DropCount::new();
        let probe = DropProbe::new()
            .on_drop(drops.callback())
            .on_drop(Box::new(move || {
                assert!(channels_probe.on_next(1, DropProbe::new()).is_continue());
            }));
        assert!(channels.on_next(0, probe).is_continue());
        assert!(channels.on_next(0, DropProbe::new()).is_continue());
        assert_eq!(drops.get(), 1);

        channels.on_termination(0, Termination::Completed);
        channels.on_termination(1, Termination::Completed);
        assert_eq!(checker.values(), [2]);
        assert_eq!(checker.state(), State::Completed);
    }

    #[test]
    fn test_lifetime_sub() {
        // OK
        let life_marker = TestStruct;
        let _subscription;

        // Error
        // let _subscription;
        // let life_marker = TestStruct;

        {
            let observable = Create::shared_boxed(|mut observer| {
                assert!(observer.on_next(1).is_continue());
                observer.on_termination(Termination::<String>::Completed);
                DisposeOnDrop::new(CallbackDisposal::new(|| {
                    life_marker.consume_ref();
                }))
            });

            let observable = ForkJoin::new_from_iter([observable]);

            let (_, observer) = Checker::new();
            _subscription = observable.subscribe(observer);
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
            let observable = Create::shared_boxed(|observer| {
                life_marker_1 = Some(observer);
                DisposeOnDrop::default()
            });
            let observable = ForkJoin::new_from_iter([observable]);

            let (_, mut observer) = Checker::<_, Infallible>::new();
            assert!(observer.on_next(vec![Some(&life_marker_2)]).is_continue());
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
        let observable = ForkJoin::new_from_iter(vec![observable]);
        _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
    }

    #[test]
    fn test_type_inference_with_subscribe() {
        let observable = ForkJoin::new_from_iter([Just::new(1)]);

        let observable = ForkJoin::new_from_iter([observable]);
        let (_, observer) = Checker::new();
        let _ = observable.subscribe(observer);
    }

    #[test]
    fn test_type_inference_without_subscribe() {
        let observable = ForkJoin::new_from_iter([Just::new(1)]);

        ForkJoin::new_from_iter([observable]);
    }
}
