#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod join_handle;
pub(crate) mod test_channel;
pub(crate) mod test_runtime;
pub(crate) mod test_scheduler;
pub(crate) mod test_struct;
#[cfg(not(feature = "single-threaded"))]
pub(crate) mod test_thread_scheduler;

use std::time::Duration;
pub(crate) const RECURSION_EXECUTION_TIMES: usize = 200;
pub(crate) const RECURSION_EXPECTED_DIFF: Duration = Duration::from_millis(40);
pub(crate) const RECURSION_PERIOD: Duration = Duration::from_millis(5);
