//! [`Scheduler`] for the executors of the `futures` crate: [`ThreadPoolScheduler`] for a
//! [`ThreadPool`], [`LocalPoolScheduler`] for a [`LocalPool`](futures::executor::LocalPool).
//! Timers come from `async-io`.

use crate::{
    disposable::Disposable,
    observable::Subscription,
    scheduler::{Scheduler, SchedulerTypes, Task, drive},
    thread_mode::{Local, Shared},
};
use futures::{
    executor::{LocalSpawner, ThreadPool},
    future::RemoteHandle,
    task::{LocalSpawnExt, SpawnExt},
};
use std::time::Duration;

/// The multi-threaded scheduler for a [`ThreadPool`], whose tasks must be `Send`.
///
/// The scheduler holds a handle on the pool, so the pool shuts down only once the scheduler and
/// the tasks that reach it are gone too. A panic inside a task is caught by the pool and never
/// surfaces.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "futures-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "futures-scheduler")]
/// fn main() {
///     use futures::{executor::{block_on, ThreadPool}, StreamExt};
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::futures::ThreadPoolScheduler,
///     };
///     use std::time::Duration;
///
///     let scheduler = ThreadPoolScheduler::from_pool(ThreadPool::new().unwrap());
///     let values = block_on(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), scheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ThreadPoolScheduler {
    pool: ThreadPool,
}

impl ThreadPoolScheduler {
    /// Runs the tasks on `pool`.
    pub fn from_pool(pool: ThreadPool) -> Self {
        Self { pool }
    }
}

impl SchedulerTypes for ThreadPoolScheduler {
    type Mode = Shared;
    type Disposal = FuturesDisposal;
}

/// # Panics
///
/// Running a task panics if the pool cannot spawn it.
impl<TC, P> Scheduler<TC, P> for ThreadPoolScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        let handle = self
            .pool
            .spawn_with_handle(drive(task, delay, sleep))
            .expect("failed to spawn future");
        Subscription::new(FuturesDisposal(handle))
    }
}

/// The single-threaded scheduler for a [`LocalPool`](futures::executor::LocalPool), whose tasks
/// need not be `Send`. The pool makes progress only while it is run (`run`, `run_until`,
/// `run_until_stalled`).
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "futures-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "futures-scheduler")]
/// fn main() {
///     use futures::{executor::LocalPool, StreamExt};
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::futures::LocalPoolScheduler,
///     };
///     use std::time::Duration;
///
///     let mut pool = LocalPool::new();
///     let scheduler = LocalPoolScheduler::from_spawner(pool.spawner());
///     let values = pool.run_until(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), scheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LocalPoolScheduler {
    spawner: LocalSpawner,
}

impl LocalPoolScheduler {
    /// Runs the tasks on the pool of `spawner`, which the scheduler does not keep alive: dropping
    /// the pool cancels the tasks it holds.
    pub fn from_spawner(spawner: LocalSpawner) -> Self {
        Self { spawner }
    }
}

impl SchedulerTypes for LocalPoolScheduler {
    type Mode = Local;
    type Disposal = FuturesDisposal;
}

/// # Panics
///
/// Running a task panics once the pool has been dropped.
impl<TC, P> Scheduler<TC, P> for LocalPoolScheduler
where
    TC: 'static,
    P: 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        let handle = self
            .spawner
            .spawn_local_with_handle(drive(task, delay, sleep))
            .expect("failed to spawn future");
        Subscription::new(FuturesDisposal(handle))
    }
}

async fn sleep(duration: Duration) {
    async_io::Timer::after(duration).await;
}

/// The handle of a task spawned by a [`ThreadPoolScheduler`] or a [`LocalPoolScheduler`];
/// disposing it cancels the task.
pub struct FuturesDisposal(RemoteHandle<()>);

impl Disposable for FuturesDisposal {
    fn dispose(self) {
        // Dropping a `RemoteHandle` cancels the remote future.
        drop(self.0);
    }
}
