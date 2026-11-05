mod tests_utils;

use crate::tests_utils::checker::State;
use crate::tests_utils::test_channel::ChannelState;
use crate::tests_utils::test_channel::ReceiverObservable;
use crate::tests_utils::test_channel::test_channels;
use crate::tests_utils::test_scheduler::block_on;
use rx_rust::disposable::Disposable;
use rx_rust::disposable::callback_disposal::CallbackDisposal;
use rx_rust::disposable::dispose_on_drop::DisposeOnDrop;
use rx_rust::observable::boxed_observable::SendBoxedObservable;
use rx_rust::observer::boxed_observer::SendBoxedObserver;
use rx_rust::operators::creating::create::Create;
use rx_rust::operators::creating::empty::Empty;
use rx_rust::operators::creating::from_iter::FromIter;
use rx_rust::subject::behavior_subject::BehaviorSubject;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::MutableExt;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableExt},
    observer::{Observer, Termination},
    operators::{
        creating::{just::Just, throw::Throw},
        utility::repeat_source::RepeatSource,
    },
    subject::publish_subject::PublishSubject,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tests_utils::{checker::Checker, test_channel::test_channel, test_struct::TestStruct};

/// A callback that repeats `source` `rounds` times, then gives up, counting its calls in `calls`.
fn repeating<OE: Clone>(
    source: OE,
    rounds: usize,
    calls: &Arc<AtomicUsize>,
) -> impl FnMut() -> Option<OE> + Clone + use<OE> {
    let calls = calls.clone();
    move || (calls.fetch_add(1, Ordering::SeqCst) < rounds).then(|| source.clone())
}

fn call_count(calls: &AtomicUsize) -> usize {
    calls.load(Ordering::SeqCst)
}

#[test]
fn test_completed_no_repeat() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 0, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 0);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 0);

    channels.on_termination(0, Termination::<&str>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.len(), 1);
    assert_eq!(call_count(&calls), 1);
}

#[test]
fn test_completed_repeat_once() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 0);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 0);

    channels.on_termination(0, Termination::<&str>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 1);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 1);

    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels.len(), 2);
    assert_eq!(call_count(&calls), 2);
}

#[test]
fn test_completed_repeat_twice() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 2, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::<&str>::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 1);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 2);

    assert!(channels.on_next(2, 333).is_continue());
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Active);

    channels.on_termination(2, Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(2), ChannelState::Completed);
    assert_eq!(channels.len(), 3);
    assert_eq!(call_count(&calls), 3);
}

#[test]
fn test_completed_different_repeat_observable() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, _, &str>();
    let (checker, observer) = Checker::new();

    let mut round = 0;
    let observable = observable.repeat_source(move || {
        round += 1;
        match round {
            1 => Some(
                Just::new(222)
                    .with_error_type()
                    .into_shared()
                    .into_send_boxed(),
            ),
            2 => Some(
                FromIter::new([333, 444])
                    .with_error_type()
                    .into_shared()
                    .into_send_boxed(),
            ),
            3 => None,
            _ => panic!(),
        }
    });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker.state(), ChannelState::Subscribed);

    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333, 444]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channel_checker.state(), ChannelState::Completed);
}

#[test]
fn test_completed_synchronous_complete() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let source = observable.clone();
    let calls_cloned = calls.clone();
    let observable =
        observable.repeat_source(move || match calls_cloned.fetch_add(1, Ordering::SeqCst) {
            0 => Some(
                Empty
                    .with_item_type()
                    .with_error_type()
                    .into_shared()
                    .into_send_boxed(),
            ),
            1 => Some(source.clone().into_send_boxed()),
            _ => None,
        });

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::<&str>::Completed);
    // The empty round completes from inside its subscription, and the next one is subscribed.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 2);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(1), ChannelState::Completed);
    assert_eq!(call_count(&calls), 3);
}

/// How many synchronous completions the tests below go through: deep enough that repeating each
/// one from inside the previous subscription would overflow the stack.
const SYNCHRONOUS_COMPLETIONS: i32 = 10_000;

/// Counts the subscriptions of [`completing`] that are on the stack at once, and the most there
/// ever were.
#[derive(Clone, Default)]
struct Nesting {
    current: Arc<AtomicUsize>,
    deepest: Arc<AtomicUsize>,
}

impl Nesting {
    fn deepest(&self) -> usize {
        self.deepest.load(Ordering::SeqCst)
    }
}

/// Sends `value`, then completes, both from inside its own `subscribe`.
fn completing(
    value: i32,
    nesting: &Nesting,
) -> SendBoxedObservable<'static, 'static, 'static, i32, i32, Shared> {
    let nesting = nesting.clone();
    Create::shared_boxed(move |mut observer: SendBoxedObserver<'_, i32, i32>| {
        let current = nesting.current.fetch_add(1, Ordering::SeqCst) + 1;
        nesting.deepest.fetch_max(current, Ordering::SeqCst);
        if observer.on_next(value).is_continue() {
            observer.on_termination(Termination::Completed);
        }
        nesting.current.fetch_sub(1, Ordering::SeqCst);
        DisposeOnDrop::default()
    })
    .into_send_boxed()
}

#[test]
fn test_completed_after_many_synchronous_completions() {
    let nesting = Nesting::default();
    let (checker, observer) = Checker::new();

    let nesting_of_callback = nesting.clone();
    let mut round = 0;
    let observable = completing(0, &nesting).repeat_source(move || {
        round += 1;
        (round <= SYNCHRONOUS_COMPLETIONS).then(|| completing(round, &nesting_of_callback))
    });
    let _subscription = observable.subscribe(observer);

    let expected: Vec<_> = (0..=SYNCHRONOUS_COMPLETIONS).collect();
    assert_eq!(checker.values(), expected);
    assert_eq!(checker.state(), State::Completed);
    // Each round is subscribed once the previous subscription has returned, not inside it.
    assert_eq!(nesting.deepest(), 1);
}

#[test]
fn test_error_after_many_synchronous_completions() {
    let nesting = Nesting::default();
    let (checker, observer) = Checker::new();

    let nesting_of_callback = nesting.clone();
    let mut round = 0;
    let observable = completing(0, &nesting).repeat_source(move || {
        round += 1;
        if round <= SYNCHRONOUS_COMPLETIONS {
            Some(completing(round, &nesting_of_callback))
        } else {
            Some(
                Throw::new(-1)
                    .with_item_type()
                    .into_shared()
                    .into_send_boxed(),
            )
        }
    });
    let _subscription = observable.subscribe(observer);

    let expected: Vec<_> = (0..=SYNCHRONOUS_COMPLETIONS).collect();
    assert_eq!(checker.values(), expected);
    assert_eq!(checker.state(), State::Error(-1));
    assert_eq!(nesting.deepest(), 1);
}

#[test]
fn test_unsub_on_next_by_take_during_synchronous_completions() {
    let nesting = Nesting::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let (checker, observer) = Checker::new();

    let nesting_of_callback = nesting.clone();
    let calls_cloned = calls.clone();
    let observable = completing(0, &nesting)
        .repeat_source(move || {
            let round = calls_cloned.fetch_add(1, Ordering::SeqCst) + 1;
            Some(completing(round as i32, &nesting_of_callback))
        })
        .take(3);
    let _subscription = observable.subscribe(observer);

    // The third value stops the source before it completes again, which ends the rounds.
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(call_count(&calls), 2);
    assert_eq!(nesting.deepest(), 1);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_callback_during_synchronous_completions() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    let nesting = Nesting::default();
    let fuse: Arc<Mutex<Option<PanicOnDrop>>> = Arc::new(Mutex::new(None));
    let (checker, observer) = Checker::new();

    expect_panic_on_drop(|panic_on_drop| {
        drop(fuse.replace_value(Some(panic_on_drop)));
        let fuse = fuse.clone();
        let nesting_of_callback = nesting.clone();
        let mut round = 0;
        let observable = completing(0, &nesting).repeat_source(move || {
            round += 1;
            if round == 3 {
                drop(fuse.take_value());
            }
            Some(completing(round, &nesting_of_callback))
        });
        let _subscription = observable.subscribe(observer);
    });

    // The unwind released the observer instead of leaving it parked.
    assert_eq!(checker.values(), [0, 1, 2]);
    assert_eq!(checker.state(), State::Dropped);
}

#[cfg(panic = "unwind")]
#[test]
fn test_panicking_source_after_its_synchronous_completion() {
    use crate::tests_utils::panic::{PanicOnDrop, expect_panic_on_drop};

    let nesting = Nesting::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let fuse: Arc<Mutex<Option<PanicOnDrop>>> = Arc::new(Mutex::new(None));
    let (checker, observer) = Checker::new();

    expect_panic_on_drop(|panic_on_drop| {
        drop(fuse.replace_value(Some(panic_on_drop)));
        let fuse = fuse.clone();
        let calls_cloned = calls.clone();
        let observable = completing(0, &nesting).repeat_source(move || {
            calls_cloned.fetch_add(1, Ordering::SeqCst);
            let fuse = fuse.clone();
            // Completes, which parks the round until `subscribe` returns, then unwinds out of it.
            Some(
                Create::shared_boxed(move |observer: SendBoxedObserver<'_, i32, i32>| {
                    observer.on_termination(Termination::Completed);
                    drop(fuse.take_value());
                    DisposeOnDrop::default()
                })
                .into_send_boxed(),
            )
        });
        let _subscription = observable.subscribe(observer);
    });

    // The parked round was not repeated, and the observer parked with it was released rather than
    // kept alive by the trampoline it holds.
    assert_eq!(call_count(&calls), 1);
    assert_eq!(checker.values(), [0]);
    assert_eq!(checker.state(), State::Dropped);
}

/// What the sources of the disposal tests below did, in order.
type Log = Arc<Mutex<Vec<String>>>;

/// Where a source of the disposal tests below leaves its observer, for the test to complete it
/// later.
type Stash = Arc<Mutex<Option<SendBoxedObserver<'static, i32, i32>>>>;

/// Records its subscription and its disposal in `log`. It completes from inside its own
/// `subscribe` without a `stash`, and leaves its observer in `stash` otherwise.
fn logged(
    index: usize,
    log: &Log,
    stash: Option<&Stash>,
) -> SendBoxedObservable<'static, 'static, 'static, i32, i32, Shared> {
    let log = log.clone();
    let stash = stash.cloned();
    Create::shared_boxed(move |observer: SendBoxedObserver<'static, i32, i32>| {
        log.with_mut(|log| log.push(format!("subscribe {index}")));
        match &stash {
            Some(stash) => drop(stash.replace_value(Some(observer))),
            None => observer.on_termination(Termination::Completed),
        }
        let log = log.clone();
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            log.with_mut(|log| log.push(format!("dispose {index}")));
        }))
    })
    .into_send_boxed()
}

#[test]
fn test_completed_source_disposed_before_repeat() {
    let log = Log::default();
    let stash = Stash::default();
    let (_checker, observer) = Checker::new();

    let log_of_callback = log.clone();
    let stash_of_callback = stash.clone();
    let mut round = 0;
    let observable = logged(0, &log, Some(&stash)).repeat_source(move || {
        round += 1;
        Some(logged(round, &log_of_callback, Some(&stash_of_callback)))
    });
    let subscription = observable.subscribe(observer);
    assert_eq!(log.clone_value(), ["subscribe 0"]);

    stash
        .take_value()
        .unwrap()
        .on_termination(Termination::Completed);
    assert_eq!(
        log.clone_value(),
        ["subscribe 0", "dispose 0", "subscribe 1"]
    );

    stash
        .take_value()
        .unwrap()
        .on_termination(Termination::Completed);
    assert_eq!(
        log.clone_value(),
        [
            "subscribe 0",
            "dispose 0",
            "subscribe 1",
            "dispose 1",
            "subscribe 2"
        ]
    );

    subscription.dispose();
    assert_eq!(
        log.clone_value(),
        [
            "subscribe 0",
            "dispose 0",
            "subscribe 1",
            "dispose 1",
            "subscribe 2",
            "dispose 2"
        ]
    );
}

#[test]
fn test_synchronously_completed_sources_disposed_before_repeat() {
    let log = Log::default();
    let stash = Stash::default();
    let (_checker, observer) = Checker::new();

    let log_of_callback = log.clone();
    let stash_of_callback = stash.clone();
    let mut round = 0;
    let observable = logged(0, &log, None).repeat_source(move || {
        round += 1;
        // The second round is the first source that does not complete on its own.
        let stash = (round == 2).then_some(&stash_of_callback);
        Some(logged(round, &log_of_callback, stash))
    });
    let subscription = observable.subscribe(observer);
    assert_eq!(
        log.clone_value(),
        [
            "subscribe 0",
            "dispose 0",
            "subscribe 1",
            "dispose 1",
            "subscribe 2"
        ]
    );

    subscription.dispose();
    assert_eq!(
        log.clone_value(),
        [
            "subscribe 0",
            "dispose 0",
            "subscribe 1",
            "dispose 1",
            "subscribe 2",
            "dispose 2"
        ]
    );
}

/// Where a test leaves the disposal of its whole subscription, for a source to run it.
type Unsubscribe = Arc<Mutex<Option<Box<dyn FnOnce() + Send>>>>;

/// The subscription is disposed from the disposal of the source that completed, which the next
/// round runs before subscribing: the callback must not be asked, and nothing must be subscribed.
#[test]
fn test_unsub_on_unsub_of_completed_source() {
    let stash = Stash::default();
    let unsubscribe = Unsubscribe::default();
    let calls = Arc::new(AtomicUsize::new(0));
    let (checker, observer) = Checker::new();

    let stash_of_source = stash.clone();
    let unsubscribe_of_source = unsubscribe.clone();
    let source = Create::shared_boxed(move |observer: SendBoxedObserver<'static, i32, i32>| {
        drop(stash_of_source.replace_value(Some(observer)));
        let unsubscribe = unsubscribe_of_source.clone();
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            if let Some(unsubscribe) = unsubscribe.take_value() {
                unsubscribe();
            }
        }))
    });
    let calls_cloned = calls.clone();
    let observable = source.repeat_source(move || {
        calls_cloned.fetch_add(1, Ordering::SeqCst);
        Some(Just::new(42).with_error_type().into_shared())
    });
    let subscription = observable.subscribe(observer);
    drop(unsubscribe.replace_value(Some(Box::new(move || subscription.dispose()))));

    stash
        .take_value()
        .unwrap()
        .on_termination(Termination::Completed);
    assert!(unsubscribe.with_ref(Option::is_none));
    assert_eq!(call_count(&calls), 0);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
}

#[test]
fn test_error_no_repeat() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    // An error is not repeated: it goes downstream at once.
    channels.on_termination(0, Termination::Error("error"));
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channels.state(0), ChannelState::Error("error"));
    assert_eq!(channels.len(), 1);
    assert_eq!(call_count(&calls), 0);
}

#[test]
fn test_error_after_repeat() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 2, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 1);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Error("error"));
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channels.state(1), ChannelState::Error("error"));
    assert_eq!(channels.len(), 2);
    assert_eq!(call_count(&calls), 1);
}

#[test]
fn test_completed_source_and_repeat_are_same() {
    let mut subject = PublishSubject::<_, &str, _>::shared();
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let observable = subject
        .clone()
        .repeat_source(repeating(subject.clone(), 3, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    // Each round subscribes to the subject that has completed, which completes it at once.
    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(call_count(&calls), 4);
}

#[test]
fn test_unsubscribe_before_repeat() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels::<_, &str>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let subscription = observable.subscribe(observer);
    assert!(channels.on_next(0, 111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(call_count(&calls), 0);
}

#[test]
fn test_unsubscribe_after_repeat() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels::<_, &str>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let subscription = observable.subscribe(observer);
    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Completed);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls), 1);

    assert!(channels.on_next(1, 222).is_continue());
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);

    subscription.dispose();
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channels.state(1), ChannelState::Unsubscribed);
    assert_eq!(call_count(&calls), 1);
}

#[test]
fn test_ref() {
    let value_1 = 111;
    let value_2 = 222;
    let error = -1;
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(channels.on_next(0, &value_1).is_continue());
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);

    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [&value_1]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, &value_2).is_continue());
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Active);

    channels.on_termination(1, Termination::Error(&error));
    assert_eq!(checker.values(), [&value_1, &value_2]);
    assert_eq!(checker.state(), State::Error(&error));
    assert_eq!(channels.state(1), ChannelState::Error(&error));
}

#[test]
fn test_mut_ref() {
    let mut value_1 = 111;
    let mut value_2 = 222;
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels::<_, &str>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let subscription = observable.subscribe_with_callback(
        |value: &mut i32| {
            *value *= 2;
        },
        |_| {},
    );

    assert!(channels.on_next(0, &mut value_1).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert!(channels.on_next(1, &mut value_2).is_continue());
    channels.on_termination(1, Termination::Completed);
    drop(subscription);
    drop(channels);

    assert_eq!(value_1, 222);
    assert_eq!(value_2, 444);
}

#[test]
fn test_async() {
    block_on(|scheduler| async move {
        let (checker, observer) = Checker::new();
        let calls = Arc::new(AtomicUsize::new(0));

        let (channels, observable) = test_channels::<_, &str>();
        let observable = observable
            .clone()
            .repeat_source(repeating(observable, 1, &calls));

        let _subscription = scheduler
            .spawn(async move { observable.subscribe(observer) })
            .await;
        assert!(checker.values().is_empty());
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Subscribed);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(0, 111).is_continue());
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                channels_cloned.on_termination(0, Termination::Completed);
            })
            .await;
        assert_eq!(checker.values(), [111]);
        assert_eq!(checker.state(), State::Active);
        assert_eq!(channels.state(0), ChannelState::Completed);
        assert_eq!(channels.state(1), ChannelState::Subscribed);
        assert_eq!(call_count(&calls), 1);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                assert!(channels_cloned.on_next(1, 222).is_continue());
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Active);

        let channels_cloned = channels.clone();

        scheduler
            .spawn(async move {
                channels_cloned.on_termination(1, Termination::Completed);
            })
            .await;
        assert_eq!(checker.values(), [111, 222]);
        assert_eq!(checker.state(), State::Completed);
        assert_eq!(channels.state(1), ChannelState::Completed);
        assert_eq!(call_count(&calls), 2);
    });
}

#[test]
fn test_subscribe_by_different_observer() {
    let mut subject = PublishSubject::<_, &str, _>::shared();
    let (checker_1, observer_1) = Checker::new();
    let (checker_2, observer_2) = Checker::new();

    let subject_cloned = subject.clone();
    let mut round = 0;
    let observable = subject.clone().repeat_source(move || {
        round += 1;
        (round <= 2).then(|| {
            Just::new(round * 1000)
                .with_error_type()
                .into_shared()
                .merge_with(subject_cloned.clone())
                .into_send_boxed()
        })
    });
    // Each clone counts its own rounds.
    let observable_1 = observable;
    let observable_2 = observable_1.clone();

    let _subscription_1 = observable_1.subscribe(observer_1);
    let (on_next, on_termination) = observer_2.into_callbacks();
    let _subscription_2 = observable_2.subscribe_with_callback(on_next, on_termination);
    assert!(checker_1.values().is_empty());
    assert!(checker_2.values().is_empty());

    assert!(subject.on_next(111).is_continue());
    assert_eq!(checker_1.values(), [111]);
    assert_eq!(checker_2.values(), [111]);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker_1.values(), [111, 1000, 2000]);
    assert_eq!(checker_1.state(), State::Completed);
    assert_eq!(checker_2.values(), [111, 1000, 2000]);
    assert_eq!(checker_2.state(), State::Completed);
}

#[test]
fn test_unsub_on_next_by_take() {
    let (checker, observer) = Checker::<_, &str>::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls))
        .take(1);

    let _subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_stop());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(0), ChannelState::Unsubscribed);
    assert_eq!(call_count(&calls), 0);
}

#[test]
fn test_multiple_operation() {
    let (checker, observer) = Checker::new();
    let calls_1 = Arc::new(AtomicUsize::new(0));
    let calls_2 = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels::<_, &str>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable.clone(), 1, &calls_1))
        .repeat_source(repeating(observable, 1, &calls_2));

    let _subscription = observable.subscribe(observer);
    assert_eq!(channels.state(0), ChannelState::Subscribed);

    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::Completed);
    // The inner repeat runs its round.
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);
    assert_eq!(call_count(&calls_1), 1);
    assert_eq!(call_count(&calls_2), 0);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Completed);
    // The inner repeat gives up and completes, so the outer one runs its round.
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(2), ChannelState::Subscribed);
    assert_eq!(call_count(&calls_1), 2);
    assert_eq!(call_count(&calls_2), 1);

    assert!(channels.on_next(2, 333).is_continue());
    channels.on_termination(2, Termination::Completed);
    assert_eq!(checker.values(), [111, 222, 333]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(channels.state(2), ChannelState::Completed);
    assert_eq!(channels.len(), 3);
    assert_eq!(call_count(&calls_1), 2);
    assert_eq!(call_count(&calls_2), 2);
}

#[test]
fn test_without_convenient_api() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let (channels, observable) = test_channels::<_, &str>();
    let observable = RepeatSource::new(observable.clone(), repeating(observable, 1, &calls));

    let _subscription = observable.subscribe(observer);
    assert!(channels.on_next(0, 111).is_continue());
    channels.on_termination(0, Termination::Completed);
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channels.state(1), ChannelState::Subscribed);

    assert!(channels.on_next(1, 222).is_continue());
    channels.on_termination(1, Termination::Completed);
    assert_eq!(checker.values(), [111, 222]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(call_count(&calls), 2);
}

#[test]
fn test_next_on_sub() {
    let subject = BehaviorSubject::<_, &str, _>::shared(111);
    let (checker, observer) = Checker::new();

    let mut round = 0;
    let observable = subject.clone().repeat_source(move || {
        round += 1;
        (round == 1).then(|| Just::new(222).with_error_type())
    });

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![111]);
    assert_eq!(checker.state(), State::Active);

    subject.on_termination(Termination::Completed);
    assert_eq!(checker.values(), vec![111, 222]);
    assert_eq!(checker.state(), State::Completed);
}

#[test]
fn test_complete_on_sub() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let observable = Empty.with_item_type::<i32>().with_error_type::<&str>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 2, &calls));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Completed);
    assert_eq!(call_count(&calls), 3);
}

#[test]
fn test_error_on_sub() {
    let (checker, observer) = Checker::new();
    let calls = Arc::new(AtomicUsize::new(0));

    let observable = Throw::new("error").with_item_type::<i32>();
    let observable = observable
        .clone()
        .repeat_source(repeating(observable, 1, &calls));

    let _subscription = observable.subscribe(observer);
    assert_eq!(checker.values(), vec![]);
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(call_count(&calls), 0);
}

#[test]
fn test_next_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source emits from inside its own disposal, so the value arrives while downstream is
    // unsubscribing. RepeatSource hands the observer to its source, so the value still reaches it.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            let mut observer = observer;
            assert!(observer.on_next(111).is_continue());
        }))
    });

    let mut observable_1_option = Some(observable_1);
    let observable = observable.repeat_source(move || observable_1_option.take());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
}

#[test]
fn test_complete_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source completes from inside its own disposal, so it terminates while downstream is
    // unsubscribing. The callback must not be asked about that completion, the next round must
    // not be subscribed, and the observer must be dropped instead of completed.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Completed);
        }))
    });

    let calls = Arc::new(AtomicUsize::new(0));
    let calls_cloned = calls.clone();
    let mut observable_1_option = Some(observable_1);
    let observable = observable.repeat_source(move || {
        calls_cloned.fetch_add(1, Ordering::SeqCst);
        observable_1_option.take()
    });

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
    assert_eq!(call_count(&calls), 0);
}

#[test]
fn test_error_on_unsub() {
    let (_, observable_1, channel_checker_1) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    // The source fails from inside its own disposal, so it terminates while downstream is
    // unsubscribing. RepeatSource hands the observer to its source, so the error still reaches
    // it, and nothing is repeated, as an error ends RepeatSource.
    let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, i32, &str>| {
        DisposeOnDrop::new(CallbackDisposal::new(move || {
            observer.on_termination(Termination::Error("error"));
        }))
    });

    let mut observable_1_option = Some(observable_1);
    let observable = observable.repeat_source(move || observable_1_option.take());

    let subscription = observable.subscribe(observer);
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Active);
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);

    subscription.dispose();
    assert!(checker.values().is_empty());
    assert_eq!(checker.state(), State::Error("error"));
    assert_eq!(channel_checker_1.state(), ChannelState::Initialized);
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
            assert!(observer.on_next(Just::new(1)).is_continue());
            observer.on_termination(Termination::<String>::Completed);
            DisposeOnDrop::new(CallbackDisposal::new(|| {
                life_marker.consume_ref();
            }))
        });
        let observable = observable.repeat_source(|| None::<PublishSubject<'_, _, _, Shared>>);

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
        let observable = Create::shared_boxed(|observer: SendBoxedObserver<'_, _, String>| {
            life_marker_1 = Some(observer);
            DisposeOnDrop::default()
        });
        let observable = observable.repeat_source(|| None::<PublishSubject<'_, _, _, Shared>>);

        let (_, mut observer) = Checker::new();
        assert!(observer.on_next(&life_marker_2).is_continue());
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
    let observable = observable.repeat_source(|| None::<ReceiverObservable<'_, _, _>>);
    _ = observable.clone(); // Make sure it's Clone when T and E are not Clone.
}

#[test]
fn test_type_inference_with_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, String>();
    let observable = observable.repeat_source(|| None::<PublishSubject<'_, _, _, Shared>>);

    let observable = observable.filter(|_| true);
    let (_, observer) = Checker::new();
    let _ = observable.subscribe(observer);
}

#[test]
fn test_type_inference_without_subscribe() {
    let (_, observable, _) = test_channel::<'_, Just<i32>, String>();
    let observable = observable.repeat_source(|| None::<PublishSubject<'_, _, _, Shared>>);

    observable.filter(|_| true);
}

/// The source drops its observer without a termination (decision 0004): that is not a
/// completion, so the callback is not asked, and the observer is dropped, never terminated.
#[test]
fn test_abandon_after_next() {
    let (mut sender, observable, channel_checker) = test_channel::<'_, i32, &str>();
    let (checker, observer) = Checker::new();

    let observable = observable.repeat_source(|| -> Option<ReceiverObservable<'_, i32, &str>> {
        unreachable!("an abandoned source has not completed")
    });

    let _subscription = observable.subscribe(observer);
    assert!(sender.on_next(111).is_continue());
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Active);

    sender.abandon();
    assert_eq!(checker.values(), [111]);
    assert_eq!(checker.state(), State::Dropped);
    assert_eq!(channel_checker.state(), ChannelState::Abandoned);
}
