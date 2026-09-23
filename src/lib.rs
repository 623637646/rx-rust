#![forbid(unsafe_code)]
// The README is the crate-level documentation, and its examples are doctests. They use the Tokio
// scheduler, so both only exist under that feature; docs.rs builds with it (see `Cargo.toml`).
#![cfg_attr(feature = "tokio-scheduler", doc = include_str!("../README.md"))]
#![cfg_attr(
    not(feature = "tokio-scheduler"),
    doc = "Reactive Programming in Rust, inspired by [ReactiveX](https://reactivex.io/): \
           [`Observable`](observable::Observable), [`Observer`](observer::Observer), \
           [`Disposable`](disposable::Disposable) and the [`operators`]. The full crate \
           documentation is the README, whose examples use the Tokio scheduler; build the docs \
           with the `tokio-scheduler` feature, as docs.rs does, to include it."
)]

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
