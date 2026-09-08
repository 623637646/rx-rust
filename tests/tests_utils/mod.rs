#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod drop_probe;
#[cfg(panic = "unwind")]
pub(crate) mod panic;
pub(crate) mod shared_sender;
pub(crate) mod test_channel;
pub(crate) mod test_runtime;
pub(crate) mod test_struct;
#[cfg(not(feature = "single-threaded"))]
pub(crate) mod thread_checker_scheduler;

use std::time::Duration;
pub(crate) const DURATION_3_MS: Duration = Duration::from_millis(3);
pub(crate) const DURATION_10_MS: Duration = Duration::from_millis(10);
pub(crate) const DURATION_30_MS: Duration = Duration::from_millis(30);
pub(crate) const DURATION_100_MS: Duration = Duration::from_millis(100);
