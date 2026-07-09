# rx-rust

Reactive Extensions for Rust. `rx-rust` offers a comprehensive, zero-unsafe toolkit for composing asynchronous and event-driven programs by chaining observables and operators in a declarative style inspired by [ReactiveX](https://reactivex.io/).

## Installation
`rx-rust` is a regular Cargo library crate. Add it to your project like any other dependency:

```toml
[dependencies]
rx-rust = { version = "the_latest_version", features = ["tokio-scheduler"] } # Use tokio runtime
rx-rust = { version = "the_latest_version", features = ["async-std-scheduler"] } # Use async-std runtime
rx-rust = { version = "the_latest_version", features = ["smol-scheduler"] } # Use smol runtime
rx-rust = { version = "the_latest_version", features = ["thread-pool-scheduler"] } # Use futures thread pool
rx-rust = { version = "the_latest_version", features = ["tokio-scheduler"] } # Use futures local pool

```

### Feature Flags
Feature                          | Description | Pulls in
-------------------------------- | ----------- | --------
`single-threaded`                | Core operators optimised for single-threaded use. | –
`local-pool-scheduler`           | Scheduler backed by `futures` local pool (enable for `Interval`, `Timer`, etc.). | `single-threaded`, `futures`, `async-io`
`thread-pool-scheduler`          | Scheduler backed by `futures` thread pool. | `futures/thread-pool`, `async-io`
`tokio-scheduler`                | Scheduler integration for Tokio runtimes. | `futures`, `tokio/rt`, `tokio/time`
`async-std-scheduler`            | Scheduler based on async-std. | `futures`, `async-std`
`smol-scheduler`                 | Scheduler based on smol. | `futures`, `smol`

## Quick Start
Build pipelines by combining operators from `ObservableExt` and subscribe with callbacks or custom observers.

```rust
use rx_rust::observable::ObservableExt;
use rx_rust::observer::Termination;
use rx_rust::operators::creating::range::Range;

Range::new(1..=5)
    .map(|value| value * 2)
    .filter(|value| *value % 3 == 0)
    .subscribe_with_callback(
        |value| println!("next: {value}"),
        |termination| println!("done: {termination:?}"),
    );
```

### Scheduling Example
Time-based operators require a scheduler. The example below uses Tokio; similar code works with the other scheduler features.

```rust
 #[tokio::main]
 async fn main() {
     use rx_rust::{
         observable::ObservableExt,
         observer::Termination,
         operators::{
             creating::from_iter::FromIter,
             utility::delay::Delay,
         },
     };
     use std::{
         sync::{Arc, Mutex},
         time::Duration,
     };
     use tokio::time::sleep;

     let handle = tokio::runtime::Handle::current();
     let values = Arc::new(Mutex::new(Vec::new()));
     let terminations = Arc::new(Mutex::new(Vec::new()));
     let values_observer = Arc::clone(&values);
     let terminations_observer = Arc::clone(&terminations);

     let subscription = Delay::new(
         FromIter::new(vec![1, 2, 3]),
         Duration::from_millis(5),
         handle.clone(),
     )
     .subscribe_with_callback(
         move |value| values_observer.lock().unwrap().push(value),
         move |termination| terminations_observer
             .lock()
             .unwrap()
             .push(termination),
     );

     sleep(Duration::from_millis(10)).await;
     drop(subscription);

     assert_eq!(&*values.lock().unwrap(), &[1, 2, 3]);
     assert_eq!(
         &*terminations.lock().unwrap(),
         &[Termination::Completed]
     );
 }
```

## Project Layout
- `src/observable` – Core observable traits and the `ObservableExt` extension trait that wires in every operator.
- `src/operators` – Operator implementations grouped by category (`creating`, `transforming`, `combining`, `utility`, and more) to mirror ReactiveX terminology.
- `src/subject` – Subjects bridging observers and observables for multicast workflows.
- `src/scheduler` – Scheduler abstractions and adapters for popular async executors.
- `tests/` – Exhaustive conformance tests covering each operator; great as executable documentation.
