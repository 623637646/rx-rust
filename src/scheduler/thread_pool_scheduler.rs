//! [`Scheduler`] for [`futures::executor::ThreadPool`].

use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::{executor::ThreadPool, task::SpawnExt};
use std::time::Duration;

/// A [`ThreadPool`] of the `futures` crate is a scheduler; timers come from `async-io`.
///
/// A panic inside a scheduled task is caught by the pool and never surfaces, since the task's
/// handle is only ever dropped, not awaited.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "thread-pool-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "thread-pool-scheduler")]
/// fn main() {
///     use futures::executor::{block_on, ThreadPool};
///     use futures::StreamExt;
///     use rx_rust::{observable::ObservableExt, operators::creating::from_iter::FromIter};
///     use std::time::Duration;
///
///     let scheduler = ThreadPool::new().unwrap();
///     let values = block_on(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), scheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
impl Scheduler for ThreadPool {
    type D = ThreadPoolDisposal;

    /// # Panics
    ///
    /// Panics if the pool has been shut down.
    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = self
            .spawn_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(ThreadPoolDisposal(handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

/// The handle of a task spawned on a [`ThreadPool`]; disposing it cancels the task.
pub struct ThreadPoolDisposal(futures::future::RemoteHandle<()>);

impl Disposable for ThreadPoolDisposal {
    fn dispose(self) {
        // Dropping a `RemoteHandle` cancels the remote future.
        drop(self.0);
    }
}
