//! [`Scheduler`] for smol: [`SmolScheduler`] for a multi-threaded executor, the global one or
//! your own, [`SmolLocalScheduler`] for a [`LocalExecutor`].

use crate::{
    disposable::Disposable,
    observable::Subscription,
    scheduler::{Scheduler, SchedulerTypes, Task, drive},
    thread_mode::{Local, Shared},
};
use smol::{Executor, LocalExecutor};
use std::{
    rc::{Rc, Weak},
    sync::Arc,
    time::Duration,
};

/// The multi-threaded scheduler for smol: tasks run on a smol [`Executor`], so they must be
/// `Send`.
///
/// It takes one of two forms:
/// - **Global** ([`global`](Self::global), also the [`Default`]): [`smol::spawn`], onto smol's
///   global executor, whose thread count `SMOL_THREADS` sets. It works from any thread.
/// - **Handle** ([`from_executor`](Self::from_executor)): an executor of your own, whose threads
///   and lifetime you control; it runs while you drive it (`executor.run(…)`) on your threads.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "smol-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "smol-scheduler")]
/// fn main() {
///     use futures::StreamExt;
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::smol::SmolScheduler,
///     };
///     use std::time::Duration;
///
///     let values = smol::block_on(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), SmolScheduler::global())
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SmolScheduler {
    target: SharedTarget,
}

/// Where a [`SmolScheduler`] sends its tasks.
#[derive(Debug, Clone, Default)]
enum SharedTarget {
    /// smol's global executor.
    #[default]
    Global,
    /// A given executor, held weakly (see the [module docs](crate::scheduler)).
    Handle(std::sync::Weak<Executor<'static>>),
}

impl SmolScheduler {
    /// Sends the tasks to smol's global executor.
    pub fn global() -> Self {
        Self::default()
    }

    /// Sends the tasks to `executor`, which the caller drives. The scheduler does not keep it
    /// alive: dropping the executor cancels the tasks it holds.
    ///
    /// # Panics
    ///
    /// Running a task panics once `executor` has been dropped.
    pub fn from_executor(executor: &Arc<Executor<'static>>) -> Self {
        Self {
            target: SharedTarget::Handle(Arc::downgrade(executor)),
        }
    }
}

async fn sleep(duration: Duration) {
    smol::Timer::after(duration).await;
}

impl SchedulerTypes for SmolScheduler {
    type Mode = Shared;
    type D = SmolDisposal;
}

impl<TC, P> Scheduler<TC, P> for SmolScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::D> {
        let future = drive(task, delay, sleep);
        let task = match &self.target {
            SharedTarget::Global => smol::spawn(future),
            SharedTarget::Handle(executor) => executor
                .upgrade()
                .expect("the executor of the SmolScheduler has been dropped")
                .spawn(future),
        };
        Subscription::new(SmolDisposal(task))
    }
}

/// The single-threaded scheduler for smol: tasks run on a [`LocalExecutor`] the caller drives, so
/// they need not be `Send`.
///
/// smol has no implicit executor for the current thread, so unlike
/// [`TokioLocalScheduler`](crate::scheduler::runtime::tokio::TokioLocalScheduler) there is no
/// ambient form: the scheduler always names its executor.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "smol-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "smol-scheduler")]
/// fn main() {
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::smol::SmolLocalScheduler,
///     };
///     use std::{cell::RefCell, rc::Rc, time::Duration};
///
///     let executor = Rc::new(smol::LocalExecutor::new());
///     let scheduler = SmolLocalScheduler::from_executor(&executor);
///
///     let values = Rc::new(RefCell::new(Vec::new()));
///     let values_observer = Rc::clone(&values);
///     let _subscription = FromIter::new(vec![1, 2, 3])
///         .delay(Duration::from_millis(5), scheduler)
///         .subscribe_with_callback(move |value| values_observer.borrow_mut().push(value), |_| {});
///
///     smol::block_on(executor.run(smol::Timer::after(Duration::from_millis(20))));
///     assert_eq!(*values.borrow(), [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SmolLocalScheduler {
    /// Held weakly (see the [module docs](crate::scheduler)).
    executor: Weak<LocalExecutor<'static>>,
}

impl SmolLocalScheduler {
    /// Sends the tasks to `executor`, which the caller drives. The scheduler does not keep it
    /// alive: dropping the executor cancels the tasks it holds.
    ///
    /// # Panics
    ///
    /// Running a task panics once `executor` has been dropped.
    pub fn from_executor(executor: &Rc<LocalExecutor<'static>>) -> Self {
        Self {
            executor: Rc::downgrade(executor),
        }
    }
}

impl SchedulerTypes for SmolLocalScheduler {
    type Mode = Local;
    type D = SmolDisposal;
}

impl<TC, P> Scheduler<TC, P> for SmolLocalScheduler
where
    TC: 'static,
    P: 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::D> {
        let executor = self
            .executor
            .upgrade()
            .expect("the executor of the SmolLocalScheduler has been dropped");
        Subscription::new(SmolDisposal(executor.spawn(drive(task, delay, sleep))))
    }
}

/// The handle of a task spawned on smol; disposing it cancels the task.
pub struct SmolDisposal(smol::Task<()>);

impl Disposable for SmolDisposal {
    fn dispose(self) {
        // Dropping a `smol::Task` cancels it.
        drop(self.0);
    }
}
