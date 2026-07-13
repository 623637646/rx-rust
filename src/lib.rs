#![forbid(unsafe_code)]

//! RxRust: A Reactive Extensions library for Rust
//!
//! This library provides a set of tools for composing asynchronous and event-based programs
//! using observable sequences and LINQ-style query operators.
//! See <https://reactivex.io/>

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
