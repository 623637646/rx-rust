#![allow(dead_code)]
pub(crate) mod checker;
pub(crate) mod test_channel;
pub(crate) mod test_runtime;
pub(crate) mod test_scheduler;
pub(crate) mod test_struct;

pub(crate) const RECURSION_EXECUTION_TIMES: usize = 200;
pub(crate) const RECURSION_EXPECTED_DIFF: u128 = 10_000;
pub(crate) const RECURSION_SLEEP_TIME: u64 = 10;
