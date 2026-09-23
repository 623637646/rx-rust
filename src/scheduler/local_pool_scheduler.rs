//! [`Scheduler`] for [`futures::executor::LocalSpawner`], in the single-threaded build.

use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::{executor::LocalSpawner, task::LocalSpawnExt};
use std::time::Duration;

/// The [`LocalSpawner`] of a [`LocalPool`](futures::executor::LocalPool) is a scheduler for the
/// single-threaded build; timers come from `async-io`. The pool only makes progress while it is
/// being run (`run`, `run_until`, `run_until_stalled`).
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "local-pool-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "local-pool-scheduler")]
/// fn main() {
///     use futures::{executor::LocalPool, StreamExt};
///     use rx_rust::{observable::ObservableExt, operators::creating::from_iter::FromIter};
///     use std::time::Duration;
///
///     let mut pool = LocalPool::new();
///     let scheduler = pool.spawner();
///     let values = pool.run_until(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), scheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
impl Scheduler for LocalSpawner {
    type D = LocalSpawnerDisposal;

    /// # Panics
    ///
    /// Panics if the pool has been shut down.
    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = self
            .spawn_local_with_handle(future)
            .expect("failed to spawn future");
        BoundDropDisposal::new(LocalSpawnerDisposal(handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        async_io::Timer::after(duration)
    }
}

/// The handle of a task spawned on a [`LocalSpawner`]; disposing it cancels the task.
pub struct LocalSpawnerDisposal(futures::future::RemoteHandle<()>);

impl Disposable for LocalSpawnerDisposal {
    fn dispose(self) {
        // Dropping a `RemoteHandle` cancels the remote future.
        drop(self.0);
    }
}
