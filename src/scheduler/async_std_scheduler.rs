//! [`Scheduler`] for async-std.

use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use futures::future::abortable;
use std::time::{Duration, Instant};

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
///         scheduler::async_std_scheduler::AsyncStdScheduler,
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
#[derive(Debug, Clone)]
pub struct AsyncStdScheduler;

impl Scheduler for AsyncStdScheduler {
    type D = AsyncStdDisposal;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let (future, abort_handle) = abortable(future);
        async_std::task::spawn(future);
        BoundDropDisposal::new(AsyncStdDisposal(abort_handle))
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        // `async_std::task::sleep` is an `async fn`, so it would start timing
        // lazily on first poll. Capture the deadline eagerly instead, to honor
        // the `Scheduler::sleep` contract (deadline measured from this call),
        // matching the other scheduler implementations.
        let deadline = Instant::now() + duration;
        async move {
            async_std::task::sleep(deadline.saturating_duration_since(Instant::now())).await;
        }
    }
}

/// The handle of a task spawned on async-std; disposing it aborts the task.
pub struct AsyncStdDisposal(futures::future::AbortHandle);

impl Disposable for AsyncStdDisposal {
    fn dispose(self) {
        self.0.abort();
    }
}
