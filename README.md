# rx-rust

[![crates.io](https://img.shields.io/crates/v/rx-rust.svg)](https://crates.io/crates/rx-rust)
[![docs.rs](https://img.shields.io/docsrs/rx-rust)](https://docs.rs/rx-rust)
[![license](https://img.shields.io/crates/l/rx-rust.svg)](LICENSE)
![MSRV](https://img.shields.io/badge/MSRV-1.88-blue.svg)

Reactive Extensions for Rust. `rx-rust` is a toolkit for composing asynchronous and event-driven
programs by chaining observables and operators in a declarative style, inspired by
[ReactiveX](https://reactivex.io/).

- 100+ operators, one per file, grouped the way reactivex.io groups them.
- `#![forbid(unsafe_code)]`, no required runtime dependency: the async runtime (Tokio, async-std,
  smol, or the `futures` executors) is selected by a feature flag.
- A built-in stop signal: an observer that has seen enough returns `Flow::Stop`, and the source
  stops pushing, even a synchronous one that is still inside `subscribe`.
- Crosses into `async` freely: a `Future` or a `Stream` becomes an observable, and an observable
  becomes a `Future` of its first item or a `Stream` of all of them.

## Installation

Add the crate with the scheduler feature for the runtime you use:

```toml
[dependencies]
rx-rust = { version = "0.3", features = ["tokio-scheduler"] }
```

Pick exactly one of the scheduler features below; `tokio-scheduler` is the most common choice. The
time-based operators (`delay`, `debounce`, `timeout`, `Interval`, `Timer`, …) take a scheduler
argument, and only compile when a scheduler feature is enabled.

### Feature flags

Feature                  | Description                                                                                    | Pulls in
------------------------ | ---------------------------------------------------------------------------------------------- | --------
`tokio-scheduler`        | Schedule on a Tokio runtime. The scheduler is a `tokio::runtime::Handle`.                       | `futures`, `tokio/rt`, `tokio/time`
`async-std-scheduler`    | Schedule on async-std. The scheduler is `AsyncStdScheduler`.                                   | `futures`, `async-std`
`smol-scheduler`         | Schedule on smol. The scheduler is `SmolScheduler`.                                            | `futures`, `smol`
`thread-pool-scheduler`  | Schedule on a `futures::executor::ThreadPool`.                                                 | `futures/thread-pool`, `async-io`
`local-pool-scheduler`   | Schedule on a `futures::executor::LocalPool`, through its `LocalSpawner`. The only scheduler for the single-threaded build. | `single-threaded`, `futures`, `async-io`
`single-threaded`        | Single-threaded build, no scheduler: shared state is `Rc` instead of `Arc`, and nothing needs to be `Send` or `Sync`. | –
`futures`                | Enabled by every scheduler feature. Gates the `Stream` bridges: `FromStream`, `FromTryStream`, `into_stream`, `into_try_stream`. | `futures`

`single-threaded` (and so `local-pool-scheduler`) is mutually exclusive with `thread-pool-scheduler`,
`tokio-scheduler`, `async-std-scheduler` and `smol-scheduler`; enabling both is a compile error.

## Quick start

Build pipelines by combining operators from `ObservableExt`, then subscribe with callbacks or with
your own `Observer`:

```rust
use rx_rust::{observable::ObservableExt, operators::creating::range::Range};

Range::new(1..=5)
    .map(|value| value * 2)
    .filter(|value| *value % 3 == 0)
    .subscribe_with_callback(
        |value| println!("next: {value}"),
        |termination| println!("done: {termination:?}"),
    );
```

`subscribe` returns a `Subscription`, and dropping it unsubscribes. A synchronous source such as
`Range` delivers everything before `subscribe` returns, so the example above can ignore it; a source
that emits later must have its subscription kept alive for as long as the events are wanted:

```rust
#[tokio::main]
async fn main() {
    use rx_rust::{observable::ObservableExt, operators::creating::interval::Interval};
    use std::time::Duration;
    use tokio::time::sleep;

    let scheduler = tokio::runtime::Handle::current();
    let subscription = Interval::new(Duration::from_millis(10), scheduler, None)
        .subscribe_with_callback(|tick| println!("tick {tick}"), |_| {});

    sleep(Duration::from_millis(35)).await;
    drop(subscription); // unsubscribes: no more ticks
}
```

## Core concepts

`Observable<'or, T, E>` is a source of `T` items that ends with a `Termination<E>`: either
`Completed` or `Error(E)`. A source that cannot fail uses `E = Infallible`, and some conversions
(`into_future`, `into_stream`) are only available for those. `'or` is the lifetime an observer may
borrow for; `'static` for anything that goes through a scheduler.

An `Observer<T, E>` has two methods:

```rust
use rx_rust::observer::{Flow, Observer, Termination};

struct Printer;

impl Observer<i32, std::convert::Infallible> for Printer {
    fn on_next(&mut self, value: i32) -> Flow {
        println!("{value}");
        Flow::Continue
    }

    fn on_termination(self, termination: Termination<std::convert::Infallible>) {
        println!("{termination:?}");
    }
}
```

`on_termination` takes `self`: a termination is the last event, and the observer is consumed by it.

### Stopping from the observer

`on_next` returns a `Flow`. `Flow::Continue` lets the source go on; `Flow::Stop` is a promise that
the observer accepts nothing more, not even a termination. The source stops pushing and drops the
observer. This is how a consumer ends an infinite synchronous source, which never leaves `subscribe`
and so has no subscription to drop yet:

```rust
use rx_rust::{observable::ObservableExt, observer::Flow, operators::creating::range::Range};

let mut seen = Vec::new();
Range::new(1..).subscribe_with_callback(
    |value| {
        seen.push(value);
        if value == 3 { Flow::Stop } else { Flow::Continue }
    },
    |_| {}, // not called: a stopped observer is not terminated
);
assert_eq!(seen, [1, 2, 3]);
```

A callback that returns `()` is `Flow::Continue`, so the common case needs no trailing value.
Operators forward the answer of their downstream to the source, so `take`, `first`, `take_until`
and friends stop their source the same way. When the source is not the one calling `on_next` —
when it delivers through a scheduler — the value is queued and the stop reaches the source with the
next push, which is why `Flow::Continue` is only a hint and disposal is still needed.

## Schedulers

Every time-based operator takes a value implementing `Scheduler`, so nothing is global and one
program can drive different pipelines on different runtimes:

Feature                  | Scheduler value
------------------------ | ---------------
`tokio-scheduler`        | `tokio::runtime::Handle::current()`, or a handle from a `Runtime` you own
`async-std-scheduler`    | `rx_rust::scheduler::async_std_scheduler::AsyncStdScheduler`
`smol-scheduler`         | `rx_rust::scheduler::smol_scheduler::SmolScheduler`
`thread-pool-scheduler`  | `futures::executor::ThreadPool::new()?`
`local-pool-scheduler`   | `futures::executor::LocalPool::new().spawner()`

```rust
#[tokio::main]
async fn main() {
    use futures::StreamExt;
    use rx_rust::{observable::ObservableExt, operators::creating::from_iter::FromIter};
    use std::time::Duration;

    let scheduler = tokio::runtime::Handle::current();
    let values = FromIter::new(vec![1, 2, 3])
        .delay(Duration::from_millis(5), scheduler)
        .into_stream()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(values, [1, 2, 3]);
}
```

`subscribe_on` and `observe_on` move the subscription, or the delivery of events, onto a scheduler;
the operators without a scheduler argument run synchronously on whichever thread pushes into them.

## Futures and streams

A future becomes an observable with `FromFuture`, or `FromTryFuture` when its output is a `Result`
whose `Err` should become the observable's error; both need a scheduler to run the future on. This
is how a one-shot call (an HTTP request, a database lookup) goes inside `flat_map`, `concat_map` or
`switch_map`:

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

In the other direction, `into_future()` — or `into_try_future()` for a source that can fail — turns
an observable into a future of its first item, stopping the source as soon as that item is in. It
resolves with an `Option<T>`, or a `Result<Option<T>, E>`. Put `last()`, `collect()` or `reduce()`
in front of it to pick another item or to make the source always emit:

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

A whole sequence, rather than one item of it, is consumed through `into_stream()` /
`into_try_stream()`, which turn an observable into a `Stream` of its items; `FromStream` /
`FromTryStream` go the other way. These four need the `futures` feature, which every scheduler
feature enables.

### Single, Maybe and Completable

`rx-rust` deliberately has no `Single`, `Maybe` or `Completable` types. ReactiveX added them for
languages that lack a native way to express "one asynchronous result"; Rust has one, and
`async`/`await` composes it better than any operator set could:

ReactiveX     | Rust
------------- | ----
`Single<T>`   | `Future<Output = Result<T, E>>`
`Maybe<T>`    | `Future<Output = Result<Option<T>, E>>`
`Completable` | `Future<Output = Result<(), E>>`

The operators of `Single` map onto plain async code — `map`/`flatMap` is `async { f(future.await?) }`,
`zip` is `futures::try_join!`, `amb` is `futures::future::select`, `timeout`/`delay` are the
runtime's `timeout`/`sleep`, `retry` is a `loop` around `.await`, `cache` is `FutureExt::shared`,
and dropping the future is its `Disposable`. `into_try_future()` is the bridge: its output is
exactly a `Maybe`, and with `collect()` in front, a `Single`. Keeping one-shot results as futures
also keeps this crate's operator set a single one: every operator works on `Observable`, and none
needs a second implementation for `Single`.

## Subjects

A subject is both an `Observer` and an `Observable`: push into one end, subscribe to the other.
They are the hot sources of the crate, and what `publish` / `share` / `replay` multicast through.

Subject                                     | Late subscribers get
------------------------------------------- | --------------------
`PublishSubject`                            | Only what is emitted after they subscribe.
`BehaviorSubject`                           | The latest value first, then everything after.
`ReplaySubject` (bounded or unbounded)      | The buffered values, then everything after; the termination is replayed too.
`AsyncSubject`                              | The last value, delivered on completion; nothing on error.
`unicast_subject()`                         | A single-consumer pipe: a `UnicastSender` and a `UnicastObservable` that can be subscribed once. Values sent before the subscription are buffered.

Subjects are `Clone`; clone one to keep a sending end after subscribing the other:

```rust
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    subject::publish_subject::PublishSubject,
};

let mut seen = Vec::new();

let subject = PublishSubject::<i32, std::convert::Infallible>::new();
let mut sender = subject.clone();
let subscription = subject.subscribe_with_callback(|value| seen.push(value), |_| {});

let _ = sender.on_next(1);
let _ = sender.on_next(2);
sender.on_termination(Termination::Completed);

drop(subscription);
assert_eq!(seen, [1, 2]);
```

A cold observable is turned into a shared, hot one with `share()` (`publish().ref_count()`): the
source is subscribed when the first observer arrives and unsubscribed when the last leaves.
`publish()` / `replay()` / `multicast()` give the `ConnectableController` back for manual
`connect()` / `disconnect()`.

## Operators

The full list, with a doc example for each, is on [docs.rs](https://docs.rs/rx-rust). Sources are
types in `rx_rust::operators::creating`; everything else is a method of `ObservableExt`.

Category                    | Operators
--------------------------- | ---------
Creating                    | `Create`, `Defer`, `Empty`, `FromFuture`, `FromTryFuture`, `FromIter`, `FromResult`, `FromStream`, `FromTryStream`, `Interval`, `Just`, `Never`, `Range`, `Repeat`, `Start`, `Throw`, `Timer`
Transforming                | `map`, `scan`, `flat_map`, `concat_map`, `switch_map`, `group_by`, `buffer`, `buffer_with_count`, `buffer_with_time`, `buffer_with_time_or_count`, `window`, `window_with_count`
Filtering                   | `filter`, `first`, `last`, `element_at`, `take`, `take_last`, `skip`, `skip_last`, `distinct`, `distinct_with_key_selector`, `distinct_until_changed`, `distinct_until_changed_with_key_selector`, `debounce`, `throttle`, `sample`, `ignore_elements`
Combining                   | `merge_with`, `merge_all`, `concat_with`, `concat_all`, `switch`, `zip`, `combine_latest`, `start_with`
Conditional and boolean     | `all`, `contains`, `sequence_equal`, `default_if_empty`, `amb_with`, `take_while`, `take_until`, `skip_while`, `skip_until`
Mathematical and aggregate  | `count`, `sum`, `average`, `min`, `max`, `reduce`, `collect`, `to_vec`
Error handling              | `catch`, `map_err`, `retry`
Utility                     | `delay`, `timeout`, `timestamp`, `time_interval`, `materialize`, `dematerialize`, `subscribe_on`, `observe_on`, `do_before_subscription`, `do_after_subscription`, `do_before_next`, `do_after_next`, `do_before_termination`, `do_after_termination`, `do_before_disposal`, `do_after_disposal`
Backpressure                | `on_backpressure`, `on_backpressure_buffer`, `on_backpressure_latest`
Connectable                 | `multicast`, `publish`, `publish_last`, `replay`, `share`, `share_last`, `share_replay`, `ConnectableController::{connect, disconnect, ref_count}`
Conversion                  | `into_future`, `into_try_future`, `into_stream`, `into_try_stream`, `into_boxed`, `into_cloneable_boxed`, `with_item_type`, `with_error_type`
Debugging                   | `debug`, `debug_default_print`, `hook_on_subscription`, `hook_on_next`, `hook_on_termination`

## Project layout

- `src/observable` – The `Observable` trait, `ObservableExt` (every operator as a method), and the
  boxed observables.
- `src/observer` – The `Observer` trait, `Flow`, `Termination`, and the callback observer.
- `src/disposable` – `Disposable` and the `Subscription` that disposes on drop.
- `src/operators` – One operator per file, grouped by category to mirror ReactiveX terminology.
- `src/subject` – The subjects.
- `src/scheduler` – The `Scheduler` trait and its adapters for the supported runtimes.
- `src/utils` – Shared machinery: the lock wrapper, the `Rc`/`Arc` and `Send`/`Sync` abstraction
  behind the single-threaded build, serialized delivery.
- `tests/` – One integration test file per operator, using the same checklist of cases for each;
  great as executable documentation.

## Contributing

See [DEVELOPMENT.md](DEVELOPMENT.md) for the review checklist and the test case names every operator
is expected to cover, and [AGENTS.md](AGENTS.md) for the build and test commands. Requires Rust 1.88
or later.

## License

MIT. See [LICENSE](LICENSE).
