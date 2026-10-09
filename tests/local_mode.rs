//! The `Local` mode end to end: pipelines whose items, callbacks and state are not `Send`, built
//! from the `Local` sources and subjects and run on the single-threaded schedulers.
//!
//! The rest of the suite runs in `Shared` mode on every scheduler, the single-threaded ones
//! included (see `tests_utils::test_scheduler`), so this file is what checks that the `Local` mode
//! keeps its promise: nothing it is given needs to be `Send`. Most of that is checked by compiling
//! it; each test also asserts the basic outcome. One representative per kind of operator.
//!
//! The reverse promise, that what the `Local` mode picks is not `Send`, so that a `Local` pipeline
//! cannot cross threads, is checked by compiling too: see "What is not `Send`" below.

mod tests_utils;

use futures::executor::LocalPool;
use rx_rust::{
    disposable::{Disposable, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableExt, ObservableTypes},
    observer::{
        Observer, Termination,
        boxed_observer::{ObserverMode, SendBoxedObserver},
        callback_observer::CallbackObserver,
        emitter::Emitter,
    },
    operators::{
        combining::{combine_latest::CombineLatest, merge::Merge},
        creating::{create::Create, from_iter::FromIter},
    },
    scheduler::{
        SchedulerExt,
        runtime::{
            futures::{LocalPoolScheduler, ThreadPoolScheduler},
            smol::{SmolLocalScheduler, SmolScheduler},
            tokio::{TokioLocalScheduler, TokioScheduler},
        },
    },
    subject::{
        async_subject::AsyncSubject, behavior_subject::BehaviorSubject,
        publish_subject::PublishSubject, replay_subject::ReplaySubject,
    },
    thread_mode::{
        Local, Shared, ThreadMode,
        mutable::{MutableExt, MutableHelper},
    },
};
use std::{cell::RefCell, convert::Infallible, rc::Rc, time::Duration};
use tests_utils::{DURATION_10_MS, DURATION_100_MS};

/// What [`record`] has seen: the values, then the termination.
struct Record<T, E> {
    values: Rc<RefCell<Vec<T>>>,
    termination: Rc<RefCell<Option<Termination<E>>>>,
}

impl<T: Clone, E: Clone> Record<T, E> {
    fn values(&self) -> Vec<T> {
        self.values.clone_value()
    }

    fn termination(&self) -> Option<Termination<E>> {
        self.termination.clone_value()
    }
}

/// Subscribes `observable` with callbacks that hold `Rc`s, so that neither they nor the observer
/// they make are `Send`.
fn record<T, E, OE>(observable: OE) -> (Record<T, E>, DisposeOnDrop<OE::Disposal>)
where
    T: 'static,
    E: 'static,
    OE: ObservableTypes<Item = T, Error = E>
        + Observable<CallbackObserver<Box<dyn FnMut(T)>, Box<dyn FnOnce(Termination<E>)>>>,
{
    let values = Rc::new(RefCell::new(Vec::new()));
    let termination = Rc::new(RefCell::new(None));
    let values_cloned = values.clone();
    let termination_cloned = termination.clone();
    let on_next: Box<dyn FnMut(T)> = Box::new(move |value| {
        values_cloned.with_mut(|values| values.push(value));
    });
    let on_termination: Box<dyn FnOnce(Termination<E>)> = Box::new(move |end| {
        assert!(termination_cloned.replace_value(Some(end)).is_none());
    });
    let subscription = observable.subscribe_with_callback(on_next, on_termination);
    (
        Record {
            values,
            termination,
        },
        subscription,
    )
}

/// `Rc`s of `values`, for items that are not `Send`.
fn rcs(values: impl IntoIterator<Item = i32>) -> Vec<Rc<i32>> {
    values.into_iter().map(Rc::new).collect()
}

/// Resolves after `duration`, on any of the executors below.
async fn sleep(duration: Duration) {
    async_io::Timer::after(duration).await;
}

/// The test `$name`, once on each single-threaded scheduler: a module with one test per
/// scheduler, in which `$scheduler` names it and `$body` is the async test body run on it.
macro_rules! on_local_schedulers {
    ($name:ident, |$scheduler:ident| $body:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn local_pool() {
                let mut pool = LocalPool::new();
                let $scheduler = LocalPoolScheduler::from_spawner(pool.spawner());
                pool.run_until($body);
            }

            #[test]
            fn tokio_local() {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime");
                let local_set = Rc::new(tokio::task::LocalSet::new());
                let $scheduler = TokioLocalScheduler::from_local_set(&local_set);
                runtime.block_on(local_set.run_until($body));
            }

            #[test]
            fn smol_local() {
                let executor = Rc::new(smol::LocalExecutor::new());
                let $scheduler = SmolLocalScheduler::from_executor(&executor);
                smol::block_on(executor.run($body));
            }
        }
    };
}

// MARK: - Without a scheduler

#[test]
fn test_map_filter() {
    let offset = Rc::new(1);
    let (record, _subscription) = record(
        FromIter::new(rcs([1, 2, 3, 4]))
            .map(move |value| Rc::new(*value + *offset))
            .filter(|value| **value % 2 == 0),
    );
    assert_eq!(record.values(), rcs([2, 4]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_create() {
    let (record, _subscription) = record(Create::local(|mut emitter| {
        for value in rcs([1, 2]) {
            assert!(emitter.on_next(value).is_continue());
        }
        emitter.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    }));
    assert_eq!(record.values(), rcs([1, 2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_create_boxed() {
    let state = Rc::new(RefCell::new(0));
    let state_cloned = state.clone();
    let source = Create::local_boxed(move |mut observer| {
        state_cloned.with_mut(|count| *count += 1);
        assert!(observer.on_next(Rc::new(1)).is_continue());
        observer.on_termination(Termination::<Infallible>::Completed);
        DisposeOnDrop::default()
    });
    let (record, _subscription) = record(source);
    assert_eq!(record.values(), rcs([1]));
    assert_eq!(record.termination(), Some(Termination::Completed));
    assert_eq!(state.clone_value(), 1);
}

#[test]
fn test_into_boxed() {
    let offset = Rc::new(10);
    let boxed = FromIter::new(rcs([1, 2]))
        .map(move |value| Rc::new(*value + *offset))
        .into_boxed();
    let (record, _subscription) = record(boxed);
    assert_eq!(record.values(), rcs([11, 12]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_publish_subject() {
    let mut subject: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(subject.clone());
    assert!(subject.on_next(Rc::new(1)).is_continue());
    assert!(subject.on_next(Rc::new(2)).is_continue());
    subject.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([1, 2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_behavior_subject() {
    let mut subject: BehaviorSubject<'_, Rc<i32>, Infallible, Local> =
        BehaviorSubject::local(Rc::new(1));
    let (record, _subscription) = record(subject.clone());
    assert!(subject.on_next(Rc::new(2)).is_continue());
    subject.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([1, 2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_replay_subject() {
    let mut subject: ReplaySubject<'_, Rc<i32>, Infallible, Local> = ReplaySubject::local(Some(2));
    for value in rcs([1, 2, 3]) {
        assert!(subject.on_next(value).is_continue());
    }
    let (record, _subscription) = record(subject.clone());
    subject.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([2, 3]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_async_subject() {
    let mut subject: AsyncSubject<'_, Rc<i32>, Infallible, Local> = AsyncSubject::local();
    let (record, _subscription) = record(subject.clone());
    assert!(subject.on_next(Rc::new(1)).is_continue());
    assert!(subject.on_next(Rc::new(2)).is_continue());
    assert!(record.values().is_empty());
    subject.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_merge() {
    let mut subject_1: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let mut subject_2: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(Merge::new(subject_1.clone(), subject_2.clone()));
    assert!(subject_1.on_next(Rc::new(1)).is_continue());
    assert!(subject_2.on_next(Rc::new(2)).is_continue());
    subject_1.on_termination(Termination::Completed);
    assert!(record.termination().is_none());
    subject_2.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([1, 2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_combine_latest() {
    let mut subject_1: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let mut subject_2: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(
        CombineLatest::new(subject_1.clone(), subject_2.clone())
            .map(|(value_1, value_2)| *value_1 * 10 + *value_2),
    );
    assert!(subject_1.on_next(Rc::new(1)).is_continue());
    assert!(subject_2.on_next(Rc::new(2)).is_continue());
    assert!(subject_1.on_next(Rc::new(3)).is_continue());
    subject_1.on_termination(Termination::Completed);
    subject_2.on_termination(Termination::Completed);
    assert_eq!(record.values(), [12, 32]);
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_flat_map() {
    let mut subject: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(
        subject
            .clone()
            .flat_map(|value| FromIter::new(vec![value.clone(), Rc::new(*value * 10)])),
    );
    assert!(subject.on_next(Rc::new(1)).is_continue());
    assert!(subject.on_next(Rc::new(2)).is_continue());
    subject.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([1, 10, 2, 20]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

#[test]
fn test_switch() {
    type Inner<'a> = PublishSubject<'a, Rc<i32>, Infallible, Local>;
    let mut outer: PublishSubject<'_, Inner<'_>, Infallible, Local> = PublishSubject::local();
    let mut inner_1: Inner<'_> = PublishSubject::local();
    let mut inner_2: Inner<'_> = PublishSubject::local();
    let (record, _subscription) = record(outer.clone().switch());
    assert!(outer.on_next(inner_1.clone()).is_continue());
    assert!(inner_1.on_next(Rc::new(1)).is_continue());
    assert!(outer.on_next(inner_2.clone()).is_continue());
    // The first inner is unsubscribed: its values are dropped.
    let _ = inner_1.on_next(Rc::new(2));
    assert!(inner_2.on_next(Rc::new(3)).is_continue());
    outer.on_termination(Termination::Completed);
    inner_2.on_termination(Termination::Completed);
    assert_eq!(record.values(), rcs([1, 3]));
    assert_eq!(record.termination(), Some(Termination::Completed));
}

// MARK: - What is not `Send`

// Checked when this file compiles, with no test function: each `Local` type below must not be
// `Send`, and its `Shared` twin must be. A typo or a renamed type fails the build, so unlike a
// `compile_fail` doctest these cannot pass for the wrong reason.

/// Implemented once for every type and once more for a `Send` one, so that naming `item` without
/// the parameter is ambiguous, which does not compile, exactly when the type is `Send`.
trait AmbiguousIfSend<A> {
    fn item() {}
}

impl<T: ?Sized> AmbiguousIfSend<()> for T {}

impl<T: ?Sized + Send> AmbiguousIfSend<u8> for T {}

/// Compiles only if `$ty` is not `Send`.
macro_rules! assert_not_send {
    ($ty:ty) => {
        const _: fn() = || {
            let _ = <$ty as AmbiguousIfSend<_>>::item;
        };
    };
}

/// Compiles only if `$ty` is `Send`.
macro_rules! assert_send {
    ($ty:ty) => {
        const _: fn() = || {
            fn send<T: Send>() {}
            send::<$ty>();
        };
    };
}

assert_not_send!(Local);
assert_send!(Shared);

assert_not_send!(<Local as ThreadMode>::Ptr<i32>);
assert_send!(<Shared as ThreadMode>::Ptr<i32>);

assert_not_send!(<Local as ObserverMode>::BoxedObserver<'static, i32, ()>);
assert_send!(<Shared as ObserverMode>::BoxedObserver<'static, i32, ()>);

// What `Create::local` hands its builder: not `Send` even around a `Send` observer.
assert_not_send!(Emitter<SendBoxedObserver<'static, i32, ()>, Local>);
assert_send!(Emitter<SendBoxedObserver<'static, i32, ()>, Shared>);

assert_not_send!(PublishSubject<'static, i32, (), Local>);
assert_send!(PublishSubject<'static, i32, (), Shared>);

assert_not_send!(LocalPoolScheduler);
assert_send!(ThreadPoolScheduler);
assert_not_send!(TokioLocalScheduler);
assert_send!(TokioScheduler);
assert_not_send!(SmolLocalScheduler);
assert_send!(SmolScheduler);

// MARK: - On the single-threaded schedulers

on_local_schedulers!(test_schedule, |scheduler| async move {
    // A task that is not `Send`.
    let ran = Rc::new(RefCell::new(false));
    let ran_cloned = ran.clone();
    let _disposal = scheduler.schedule(move || ran_cloned.with_mut(|ran| *ran = true), None);
    sleep(DURATION_10_MS).await;
    assert!(ran.clone_value());
});

on_local_schedulers!(test_schedule_disposed, |scheduler| async move {
    let ran = Rc::new(RefCell::new(false));
    let ran_cloned = ran.clone();
    let disposal = scheduler.schedule(
        move || ran_cloned.with_mut(|ran| *ran = true),
        Some(DURATION_10_MS),
    );
    disposal.dispose();
    sleep(DURATION_10_MS * 3).await;
    assert!(!ran.clone_value());
});

on_local_schedulers!(test_delay, |scheduler| async move {
    let mut subject: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(subject.clone().delay(DURATION_100_MS, scheduler));
    assert!(subject.on_next(Rc::new(1)).is_continue());
    subject.on_termination(Termination::Completed);
    assert!(record.values().is_empty());
    sleep(DURATION_100_MS * 2).await;
    assert_eq!(record.values(), rcs([1]));
    assert_eq!(record.termination(), Some(Termination::Completed));
});

on_local_schedulers!(test_debounce, |scheduler| async move {
    let mut subject: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(subject.clone().debounce(DURATION_100_MS, scheduler));
    assert!(subject.on_next(Rc::new(1)).is_continue());
    assert!(subject.on_next(Rc::new(2)).is_continue());
    sleep(DURATION_100_MS * 2).await;
    assert_eq!(record.values(), rcs([2]));
    subject.on_termination(Termination::Completed);
    assert_eq!(record.termination(), Some(Termination::Completed));
});

on_local_schedulers!(test_switch_map_delayed, |scheduler| async move {
    let mut subject: PublishSubject<'_, Rc<i32>, Infallible, Local> = PublishSubject::local();
    let (record, _subscription) = record(subject.clone().switch_map(move |value| {
        FromIter::new(vec![value]).delay(DURATION_10_MS, scheduler.clone())
    }));
    assert!(subject.on_next(Rc::new(1)).is_continue());
    // Replaces the first inner before it emits.
    assert!(subject.on_next(Rc::new(2)).is_continue());
    subject.on_termination(Termination::Completed);
    sleep(DURATION_100_MS).await;
    assert_eq!(record.values(), rcs([2]));
    assert_eq!(record.termination(), Some(Termination::Completed));
});
