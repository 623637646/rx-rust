#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod join_handle;
pub(crate) mod stress_test;
pub(crate) mod test_channel;
pub(crate) mod test_runtime;
pub(crate) mod test_scheduler;
pub(crate) mod test_struct;
pub(crate) mod types;

use std::time::Duration;
pub(crate) const DURATION_POST_CREATER: Duration = Duration::from_millis(1);
pub(crate) const DURATION_NEXT_LOOP: Duration = Duration::from_millis(5);
pub(crate) const DURATION_DEVIATION: Duration = Duration::from_millis(20);
pub(crate) const DURATION_LOGICAL: Duration = Duration::from_millis(100);
