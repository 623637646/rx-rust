//! [`Scheduler`] for smol.

use super::Scheduler;
use crate::{
    disposable::{Disposable, bound_drop_disposal::BoundDropDisposal},
    utils::types::MaybeSend,
};
use std::time::Duration;

/// The scheduler for smol, whose global executor needs no handle.
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
///         scheduler::smol_scheduler::SmolScheduler,
///     };
///     use std::time::Duration;
///
///     let values = smol::block_on(
///         FromIter::new(vec![1, 2, 3])
///             .delay(Duration::from_millis(5), SmolScheduler)
///             .into_stream()
///             .collect::<Vec<_>>(),
///     );
///     assert_eq!(values, [1, 2, 3]);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SmolScheduler;

impl Scheduler for SmolScheduler {
    type D = smol::Task<()>;

    fn spawn_future(
        &self,
        future: impl Future<Output = ()> + MaybeSend + 'static,
    ) -> BoundDropDisposal<Self::D> {
        let handle = smol::spawn(future);
        BoundDropDisposal::new(handle)
    }

    fn sleep(&self, duration: Duration) -> impl Future + MaybeSend + 'static + use<> {
        smol::Timer::after(duration)
    }
}

/// Disposing drops the task, which cancels it.
impl Disposable for smol::Task<()> {
    fn dispose(self) {
        // Dropping a `smol::Task` cancels it.
    }
}
