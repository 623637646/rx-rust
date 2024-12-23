#![forbid(unsafe_code)]

//! RxRust: A Reactive Extensions library for Rust
//!
//! This library provides a set of tools for composing asynchronous and event-based programs
//! using observable sequences and LINQ-style query operators.

/// Module containing the Observable trait and related types
pub mod observable;

/// Module containing the Observer trait and related types
pub mod observer;

/// Module containing various operators for Observables
pub mod operators;

/// Module containing scheduler implementations
pub mod scheduler;

/// Module containing the Subscription trait and related types
pub mod subscription;

/// Module containing utility functions and types
pub mod utils;
