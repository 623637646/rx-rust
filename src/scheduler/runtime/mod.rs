//! The [`Scheduler`](super::Scheduler) implementations for async runtimes, one module per runtime,
//! each compiled only with its feature: `tokio-scheduler`, `async-std-scheduler`,
//! `smol-scheduler` and `futures-scheduler`. A runtime with both a multi-threaded and a
//! single-threaded executor gets a `Shared` and a `Local` scheduler in its module.

#[cfg(feature = "async-std-scheduler")]
pub mod async_std;
#[cfg(feature = "futures-scheduler")]
pub mod futures;
#[cfg(feature = "smol-scheduler")]
pub mod smol;
#[cfg(feature = "tokio-scheduler")]
pub mod tokio;
