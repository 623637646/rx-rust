//! [`Scheduler`] for async-std: [`AsyncStdScheduler`] for its global thread pool.
//!
//! There is no single-threaded (`Local`) scheduler for async-std: its `spawn_local` is behind the
//! `unstable` feature, which this crate does not enable.

use crate::{
    disposable::Disposable,
    observable::Subscription,
    scheduler::{Scheduler, SchedulerTypes, Task, drive},
    thread_mode::Shared,
};
use futures::future::abortable;
use std::time::Duration;

/// The scheduler for async-std, whose runtime is global and needs no handle.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "async-std-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "async-std-scheduler")]
/// fn main() {
///     use futures::StreamExt;
///     use rx_rust::{
///         observable::ObservableExt, operators::creating::from_iter::FromIter,
///         scheduler::runtime::async_std::AsyncStdScheduler,
///     };
///     use std::time::Duration;
///
///     let values = async_std::task::block_on(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), AsyncStdScheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct AsyncStdScheduler;

impl SchedulerTypes for AsyncStdScheduler {
    type Mode = Shared;
    type Disposal = AsyncStdDisposal;
}

impl<TC, P> Scheduler<TC, P> for AsyncStdScheduler
where
    TC: Send + 'static,
    P: Send + 'static,
{
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::Disposal> {
        // `JoinHandle::cancel` is async and dropping the handle only detaches the task, so the
        // task is made abortable: aborting wakes it, and it ends at its next poll.
        let (future, abort_handle) = abortable(drive(task, delay, async_std::task::sleep));
        async_std::task::spawn(future);
        Subscription::new(AsyncStdDisposal(abort_handle))
    }
}

/// The handle of a task spawned on async-std; disposing it aborts the task.
pub struct AsyncStdDisposal(futures::future::AbortHandle);

impl Disposable for AsyncStdDisposal {
    fn dispose(self) {
        self.0.abort();
    }
}
