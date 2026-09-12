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

`single-threaded` (including `local-pool-scheduler`) is mutually exclusive with
`thread-pool-scheduler`, `tokio-scheduler`, `async-std-scheduler`, and
`smol-scheduler`.

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

## Single, Maybe and Completable
`rx-rust` deliberately has no `Single`, `Maybe` or `Completable` types. ReactiveX added them for languages that lack a native way to express "one asynchronous result"; Rust has one, and `async`/`await` composes it better than any operator set could:

ReactiveX     | Rust
------------- | ----
`Single<T>`   | `Future<Output = Result<T, E>>`
`Maybe<T>`    | `Future<Output = Result<Option<T>, E>>`
`Completable` | `Future<Output = Result<(), E>>`

The operators of `Single` map onto plain async code:

`Single` operator   | Rust
------------------- | ----
`map`, `flatMap`    | `async { f(future.await?) }`
`zip`               | `futures::try_join!`
`amb`               | `futures::select!`
`timeout`, `delay`  | the runtime's `timeout` / `sleep`
`retry`             | a `loop` around `.await`
`cache`             | `FutureExt::shared`
`Disposable`        | dropping the future cancels it

Keeping one-shot results as futures also keeps this crate's operator set a single one: every operator works on `Observable`, and none needs a second implementation for `Single`.

### Crossing between futures and observables
A future becomes an observable with `FromFuture`, or `FromTryFuture` when its output is a `Result` whose `Err` should become the observable's error; both need a scheduler to run the future on. This is how a one-shot call (an HTTP request, a database lookup) goes inside `flat_map`, `concat_map` or `switch_map`:

```rust
#[tokio::main]
async fn main() {
    use futures::StreamExt;
    use rx_rust::{
        observable::ObservableExt,
        operators::creating::{from_future::FromFuture, range::Range},
    };

    async fn fetch_name(id: u32) -> String {
        format!("user-{id}")
    }

    let handle = tokio::runtime::Handle::current();
    let names = Range::new(1..=3)
        .concat_map(move |id| FromFuture::new(fetch_name(id), handle.clone()))
        .into_stream()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(names, ["user-1", "user-2", "user-3"]);
}
```

In the other direction, `into_future()` — or `into_try_future()` for a source that can fail — turns an observable into a future of its first item, stopping the source as soon as that item is in. It resolves with an `Option<T>`, or a `Result<Option<T>, E>`: exactly what `Maybe` stands for. Put `last()`, `collect()` or `reduce()` in front of it to pick another item or to make the source always emit — that is a `Single`:

```rust
#[tokio::main]
async fn main() {
    use rx_rust::{
        observable::ObservableExt,
        operators::creating::{from_result::FromResult, range::Range},
    };

    let first = Range::new(1..=5).into_future().await;
    assert_eq!(first, Some(1));

    let sum = Range::new(1..=5).sum().into_future().await;
    assert_eq!(sum, Some(15));

    let failed = FromResult::new(Err::<i32, _>("boom")).into_try_future().await;
    assert_eq!(failed, Err("boom"));
}
```

A whole sequence, rather than one item of it, is consumed through `into_stream()` / `into_try_stream()` (feature `futures`), which turn an observable into a `Stream` of its items.

## Project Layout
- `src/observable` – Core observable traits and the `ObservableExt` extension trait that wires in every operator.
- `src/operators` – Operator implementations grouped by category (`creating`, `transforming`, `combining`, `utility`, and more) to mirror ReactiveX terminology.
- `src/subject` – Subjects bridging observers and observables, for multicast workflows and for single-consumer pipes.
- `src/scheduler` – Scheduler abstractions and adapters for popular async executors.
- `tests/` – Exhaustive conformance tests covering each operator; great as executable documentation.
