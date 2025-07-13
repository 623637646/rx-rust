#![forbid(unsafe_code)]

//! RxRust: A Reactive Extensions library for Rust
//!
//! This library provides a set of tools for composing asynchronous and event-based programs
//! using observable sequences and LINQ-style query operators.

pub mod disposable;
pub mod observable;
pub mod observer;
pub mod operators;
pub mod scheduler;
pub mod subject;
pub mod utils;
