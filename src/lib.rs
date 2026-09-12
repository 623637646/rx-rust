#![forbid(unsafe_code)]
// The README is the crate-level documentation, and its examples are doctests. They use the Tokio
// scheduler, so both only exist under that feature; docs.rs builds with it (see `Cargo.toml`).
#![cfg_attr(feature = "tokio-scheduler", doc = include_str!("../README.md"))]

#[cfg(all(
    feature = "single-threaded",
    any(
        feature = "thread-pool-scheduler",
        feature = "tokio-scheduler",
        feature = "async-std-scheduler",
        feature = "smol-scheduler"
    )
))]
compile_error!(
    "`single-threaded` and `local-pool-scheduler` are mutually exclusive with multithreaded scheduler features (`thread-pool-scheduler`, `tokio-scheduler`, `async-std-scheduler`, and `smol-scheduler`)"
);

pub mod disposable;
pub mod observable;
pub mod observer;
pub mod operators;
pub mod scheduler;
pub mod subject;
pub mod utils;
