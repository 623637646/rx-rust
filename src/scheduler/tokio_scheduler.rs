//! [`Scheduler`] for Tokio, through [`tokio::runtime::Handle`].

use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use std::time::Duration;

/// A Tokio runtime is a scheduler through its [`Handle`](tokio::runtime::Handle):
/// `tokio::runtime::Handle::current()` from inside the runtime, or [`Runtime::handle`](tokio::runtime::Runtime::handle)
/// from outside.
///
/// The runtime must have its time driver enabled (`enable_time` or `enable_all` on the builder;
/// `#[tokio::main]` does), or [`Scheduler::sleep`] panics.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use futures::StreamExt;
///     use rx_rust::{observable::ObservableExt, operators::creating::from_iter::FromIter};
///     use std::time::Duration;
///
///     let scheduler = tokio::runtime::Handle::current();
///     let values = FromIter::new(vec![1, 2, 3])
///         .delay(Duration::from_millis(5), scheduler)
///         .into_stream()
///         .collect::<Vec<_>>()
///         .await;
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
impl Scheduler for tokio::runtime::Handle {
    type D = tokio::task::JoinHandle<()>;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = self.spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        // Enter the runtime so the timer can be created outside a runtime context.
        let _guard = self.enter();
        tokio::time::sleep(duration)
    }
}

/// Disposing aborts the task; merely dropping a `JoinHandle` would detach it instead.
impl Disposable for tokio::task::JoinHandle<()> {
    fn dispose(self) {
        self.abort();
    }
}
