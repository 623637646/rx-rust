//! The scheduler the tests run on: [`block_on`] and the [`TestScheduler`] it hands to the test
//! body.
//!
//! [`block_on`] runs the body once on every scheduler the crate has, multi-threaded and
//! single-threaded alike, so that one build covers all of them. The environment variable
//! `RX_TEST_SCHEDULERS` (comma-separated [names](SchedulerKind::name)) restricts that to some of
//! them.
//!
//! The suite is built in `Shared` mode, and [`TestScheduler`] declares `Shared` for every variant,
//! the single-threaded ones included. That is sound for the same reason as
//! [`IntoShared`](rx_rust::operators::others::into_shared::IntoShared): it only makes the operators
//! downstream pick the thread-safe pointers. The `Local` mode itself is covered by
//! `tests/local_mode.rs`.

use futures::{
    channel::oneshot,
    executor::{LocalPool, ThreadPool},
    future::{BoxFuture, FutureExt},
    task::LocalSpawnExt,
};
use rx_rust::{
    disposable::{Disposable, boxed_disposal::SendBoxedDisposal},
    observable::Subscription,
    scheduler::{
        Scheduler, SchedulerExt, SchedulerTypes, Task, TaskState,
        runtime::{
            futures::{LocalPoolScheduler, ThreadPoolScheduler},
            smol::{SmolLocalScheduler, SmolScheduler},
            tokio::{TokioLocalScheduler, TokioScheduler},
        },
    },
    thread_mode::{Shared, mutable::MutableExt},
};
use std::{
    cell::RefCell,
    panic::{self, AssertUnwindSafe},
    rc::Rc,
    time::{Duration, Instant},
};

/// Which scheduler a [`TestScheduler`] is, known before its executor exists: [`block_on`] picks the
/// kinds to run before it builds anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchedulerKind {
    Tokio,
    TokioLocal,
    Smol,
    SmolLocal,
    ThreadPool,
    LocalPool,
}

impl SchedulerKind {
    /// Every kind, in the order [`block_on`] runs them.
    const ALL: [Self; 6] = [
        Self::Tokio,
        Self::TokioLocal,
        Self::Smol,
        Self::SmolLocal,
        Self::ThreadPool,
        Self::LocalPool,
    ];

    /// The name `RX_TEST_SCHEDULERS` takes and a failing test prints.
    fn name(self) -> &'static str {
        match self {
            Self::Tokio => "tokio",
            Self::TokioLocal => "tokio-local",
            Self::Smol => "smol",
            Self::SmolLocal => "smol-local",
            Self::ThreadPool => "thread-pool",
            Self::LocalPool => "local-pool",
        }
    }

    /// Runs `body` to completion on a fresh executor of this kind.
    ///
    /// On the local pool it runs until the body and everything it spawned have finished; on the
    /// others until the body has, and dropping the executor then cancels what is left.
    fn run<FU>(self, body: &impl Fn(TestScheduler) -> FU)
    where
        FU: Future<Output = ()> + 'static,
    {
        match self {
            Self::Tokio => {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime");
                let scheduler = TokioScheduler::from_handle(runtime.handle().clone());
                runtime.block_on(body(TestScheduler::Tokio(scheduler)));
            }
            Self::TokioLocal => {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed building the Runtime");
                let local_set = Rc::new(tokio::task::LocalSet::new());
                let _current = CurrentLocalGuard::enter(LocalScheduler::TokioLocal(
                    TokioLocalScheduler::from_local_set(&local_set),
                ));
                runtime.block_on(local_set.run_until(body(TestScheduler::Local)));
            }
            Self::Smol => {
                smol::block_on(body(TestScheduler::Smol(SmolScheduler::global())));
            }
            Self::SmolLocal => {
                let executor = Rc::new(smol::LocalExecutor::new());
                let _current = CurrentLocalGuard::enter(LocalScheduler::SmolLocal(
                    SmolLocalScheduler::from_executor(&executor),
                ));
                smol::block_on(executor.run(body(TestScheduler::Local)));
            }
            Self::ThreadPool => {
                futures::executor::block_on(body(TestScheduler::ThreadPool(
                    ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap()),
                )));
            }
            Self::LocalPool => {
                let mut pool = LocalPool::new();
                let spawner = pool.spawner();
                let _current = CurrentLocalGuard::enter(LocalScheduler::LocalPool(
                    LocalPoolScheduler::from_spawner(spawner.clone()),
                ));
                spawner.spawn_local(body(TestScheduler::Local)).unwrap();
                pool.run();
            }
        }
    }
}

/// A scheduler of the test suite: one variant per multi-threaded scheduler of the crate, and
/// [`Local`](Self::Local) for the single-threaded ones. [`block_on`] hands the test body each
/// scheduler in turn.
///
/// The single-threaded schedulers are not `Send`, and a scheduler of the `Shared` mode must be, so
/// [`Local`](Self::Local) carries no scheduler: it finds it in [`CURRENT_LOCAL`], which
/// [`block_on`] sets on the thread that drives their executor. Using it from another thread panics.
#[derive(Debug, Clone)]
pub(crate) enum TestScheduler {
    Tokio(TokioScheduler),
    Smol(SmolScheduler),
    ThreadPool(ThreadPoolScheduler),
    /// Whichever single-threaded scheduler [`block_on`] made current on this thread.
    Local,
}

/// A single-threaded scheduler, which [`block_on`] makes current for
/// [`TestScheduler::Local`].
#[derive(Clone)]
enum LocalScheduler {
    TokioLocal(TokioLocalScheduler),
    SmolLocal(SmolLocalScheduler),
    LocalPool(LocalPoolScheduler),
}

thread_local! {
    /// The single-threaded scheduler [`block_on`] is running the body on, if any.
    static CURRENT_LOCAL: RefCell<Option<LocalScheduler>> = const { RefCell::new(None) };
}

/// The single-threaded scheduler of this thread.
///
/// # Panics
///
/// Off the thread [`block_on`] runs the body on, or once the body has returned: the guard is
/// dropped before the executor, so a task the executor cancels on drop finds no scheduler either.
fn current_local() -> LocalScheduler {
    CURRENT_LOCAL.with(|current| current.clone_value()).expect(
        "no single-threaded TestScheduler is current: used off the thread of its executor, \
         or after the test body returned",
    )
}

/// Makes `scheduler` the [`current_local`] one for as long as it lives.
struct CurrentLocalGuard;

impl CurrentLocalGuard {
    fn enter(scheduler: LocalScheduler) -> Self {
        let previous = CURRENT_LOCAL.with(|current| current.replace_value(Some(scheduler)));
        assert!(previous.is_none(), "block_on is not reentrant");
        Self
    }
}

impl Drop for CurrentLocalGuard {
    fn drop(&mut self) {
        // Taken out first, so that the scheduler is dropped outside the lock.
        let _ = CURRENT_LOCAL.with(|current| current.take_value());
    }
}

impl TestScheduler {
    /// Runs `future` as a task of the scheduler and resolves to its output. Dropping the returned
    /// future cancels the task.
    ///
    /// A panic of the task is resumed here, so that it fails the test body the same way on every
    /// scheduler, whether or not the runtime catches the panics of its tasks.
    pub(crate) fn spawn<T>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> BoxFuture<'static, T>
    where
        T: Send + 'static,
    {
        let (sender, receiver) = oneshot::channel();
        let task = self.spawn_future(async move {
            let _ = sender.send(AssertUnwindSafe(future).catch_unwind().await);
        });
        async move {
            let _task = task;
            // The executor drops the task without running it to the end only when it is dropped
            // itself, and the test body with it.
            match receiver.await.expect("the spawned task was cancelled") {
                Ok(output) => output,
                Err(payload) => panic::resume_unwind(payload),
            }
        }
        .boxed()
    }

    /// Resolves after `duration`, timed by a task of the scheduler, so on the clock its other
    /// tasks use. The deadline is taken when this is called, not when the future is first polled.
    pub(crate) fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        let deadline = Instant::now() + duration;
        let (sender, receiver) = oneshot::channel();
        let mut sender = Some(sender);
        let timer = self.schedule_recursively(
            move |_| {
                if Instant::now() < deadline {
                    return TaskState::SleepUntil(deadline);
                }
                if let Some(sender) = sender.take() {
                    let _ = sender.send(());
                }
                TaskState::Finished
            },
            None,
        );
        async move {
            let _timer = timer;
            // An error means the executor was dropped, and the test body with it.
            let _ = receiver.await;
        }
        .boxed()
    }
}

impl SchedulerTypes for TestScheduler {
    type Mode = Shared;
    type Disposal = SendBoxedDisposal<'static>;
}

impl<TC, P> Scheduler<TC, P> for TestScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        fn erase<D: Disposable + Send + 'static>(
            subscription: Subscription<D>,
        ) -> Subscription<SendBoxedDisposal<'static>> {
            subscription.map_inner(SendBoxedDisposal::new)
        }
        match self {
            Self::Tokio(scheduler) => erase(scheduler.run_task(task, delay)),
            Self::Smol(scheduler) => erase(scheduler.run_task(task, delay)),
            Self::ThreadPool(scheduler) => erase(scheduler.run_task(task, delay)),
            Self::Local => match current_local() {
                LocalScheduler::TokioLocal(scheduler) => erase(scheduler.run_task(task, delay)),
                LocalScheduler::SmolLocal(scheduler) => erase(scheduler.run_task(task, delay)),
                LocalScheduler::LocalPool(scheduler) => erase(scheduler.run_task(task, delay)),
            },
        }
    }
}

/// Runs `body` to completion once on each scheduler, each time on a fresh executor, in the order
/// of [`SchedulerKind::ALL`]; `RX_TEST_SCHEDULERS` restricts it to the schedulers it names.
///
/// `body` is an `Fn` so that it can be called once per scheduler.
pub(crate) fn block_on<FU>(body: impl Fn(TestScheduler) -> FU)
where
    FU: Future<Output = ()> + 'static,
{
    for kind in selected_kinds() {
        let _report = ReportPanic(kind);
        kind.run(&body);
    }
}

/// The kinds to run, in the order of [`SchedulerKind::ALL`]: those `RX_TEST_SCHEDULERS` names, or
/// all of them when it is not set.
///
/// Panics on a name it does not know, so that a typo does not quietly skip every test.
fn selected_kinds() -> Vec<SchedulerKind> {
    let Ok(list) = std::env::var("RX_TEST_SCHEDULERS") else {
        return SchedulerKind::ALL.to_vec();
    };
    let names: Vec<&str> = list
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    let known = SchedulerKind::ALL.map(SchedulerKind::name);
    if let Some(unknown) = names.iter().find(|name| !known.contains(name)) {
        panic!("RX_TEST_SCHEDULERS: unknown scheduler {unknown:?}, expected one of {known:?}");
    }
    SchedulerKind::ALL
        .into_iter()
        .filter(|kind| names.contains(&kind.name()))
        .collect()
}

/// Names the scheduler in the output of a test that panics while running on it, since the test's
/// own name does not.
struct ReportPanic(SchedulerKind);

impl Drop for ReportPanic {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("the test panicked on the {} scheduler", self.0.name());
        }
    }
}
