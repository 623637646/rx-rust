# rx-rust

[![crates.io](https://img.shields.io/crates/v/rx-rust.svg)](https://crates.io/crates/rx-rust)
[![CI](https://github.com/623637646/rx-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/623637646/rx-rust/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/rx-rust)](https://docs.rs/rx-rust)
[![license](https://img.shields.io/crates/l/rx-rust.svg)](https://github.com/623637646/rx-rust/blob/master/LICENSE)
![MSRV](https://img.shields.io/badge/MSRV-1.89-blue.svg)

Reactive Extensions for Rust. `rx-rust` is a toolkit for composing asynchronous and event-driven
programs by chaining observables and operators in a declarative style, inspired by
[ReactiveX](https://reactivex.io/).

- `#![forbid(unsafe_code)]`, no required runtime dependency: the async runtime (Tokio, smol, or
  the `futures` executors) is selected by a feature flag.
- Single- or multi-threaded is part of a pipeline's type: one that stays on its thread pays for
  no `Arc<Mutex<_>>` and asks nothing to be `Send`, and one build can mix both.
- A built-in stop signal: an observer that has seen enough returns `Flow::Stop`, and the source
  stops pushing, even a synchronous one that is still inside `subscribe`.
- Time-based pipelines are testable on a virtual clock: synchronously, on exact boundaries,
  without sleeping.
- Crosses into `async` freely: a `Future` or a `Stream` becomes an observable, and an observable
  becomes a `Future` of its first item or a `Stream` of all of them.

## Installation

```toml
[dependencies]
rx-rust = { version = "2.0", features = ["tokio-scheduler"] }
```

Enable the scheduler feature of the runtime you use; the features can be combined. The schedulers
live in `rx_rust::scheduler::runtime`:

Feature             | Schedulers
------------------- | ----------
`tokio-scheduler`   | `tokio::TokioScheduler::current()`, or `from_handle(handle)` for a `Runtime` you own; `TokioLocalScheduler::ambient()` for the running `LocalSet`, or `from_local_set(&local_set)`
`smol-scheduler`    | `smol::SmolScheduler::global()`, or `from_executor(&executor)`; `SmolLocalScheduler::from_executor(&local_executor)`
`futures-scheduler` | `futures::ThreadPoolScheduler::from_pool(pool)`; `LocalPoolScheduler::from_spawner(pool.spawner())`
`futures`           | No scheduler: the `Stream` bridges (`FromStream`, `into_stream`, …). Enabled by every scheduler feature.
(always)            | `rx_rust::scheduler::virtual_time::VirtualTime`, for tests

The `*Local*` schedulers run `Local` pipelines, the others `Shared` ones (see
[Thread modes](#thread-modes)).

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

`subscribe` returns a `DisposeOnDrop`, and dropping it unsubscribes. A synchronous source such as
`Range` delivers everything before `subscribe` returns, so the example above can ignore it; a source
that emits later must have its subscription kept alive for as long as the events are wanted:

```rust
#[tokio::main]
async fn main() {
    use rx_rust::{
        observable::ObservableExt, operators::creating::interval::Interval,
        scheduler::runtime::tokio::TokioScheduler,
    };
    use std::time::Duration;
    use tokio::time::sleep;

    let subscription = Interval::new(Duration::from_millis(10), TokioScheduler::current())
        .subscribe_with_callback(|tick| println!("tick {tick}"), |_| {});

    sleep(Duration::from_millis(35)).await;
    drop(subscription); // unsubscribes: no more ticks
}
```

## Core concepts

An observable is a source of items that ends with a `Termination<E>`: either `Completed` or
`Error(E)`. A source that cannot fail uses `E = Infallible`, and some conversions (`into_future`,
`into_stream`) are only available for those. An `Observer<T, E>` receives them:

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

`on_next` returns a `Flow`. `Flow::Stop` is a promise that the observer accepts nothing more, not
even a termination: the source stops pushing and drops the observer. This is how a consumer ends an
infinite synchronous source, which never leaves `subscribe` and so has no subscription to drop yet:

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

A callback that returns `()` means `Flow::Continue`. Operators forward their downstream's answer to
the source, so `take`, `first`, `take_until` and friends stop their source the same way. A stop
does not replace disposal: a source that emits later is still ended by dropping its subscription.

## Thread modes

Every observable declares a `Mode`: `thread_mode::Local` or `thread_mode::Shared`. A `Local`
pipeline keeps its shared state in `Rc<RefCell<_>>` and needs nothing to be `Send`; a `Shared` one
uses `Arc<Mutex<_>>` and can be driven from several threads. Sources pick their mode
(`Create::local` / `Create::shared`, `PublishSubject::local()` / `PublishSubject::shared()`, …;
plain values such as `Just` or `FromIter` are `Local`), an operator with several sources is
`Shared` as soon as one of them is, and a scheduler fixes the mode of what runs on it.

Erasing the type is explicit: `into_boxed` / `into_cloneable_boxed`, or `into_send_boxed` /
`into_send_cloneable_boxed` for a box that must be `Send`. The erased type keeps the mode;
`into_shared()` lifts a `Local` source to share an erased type with `Shared` ones.

Some user code runs under an internal lock — the `Clone` and `PartialEq` of items and errors, a
custom `Clock::now`, a connectable's subject factory — and must not reach back into the same
subject or pipeline; see the `thread_mode` module documentation.

## Schedulers

Every time-based operator (`delay`, `debounce`, `timeout`, `Interval`, …) takes a value
implementing `Scheduler`, so nothing is global and one program can drive different pipelines on
different runtimes. `subscribe_on` and `observe_on` move the subscription, or the delivery of
events, onto a scheduler; the operators without a scheduler argument run synchronously on whichever
thread pushes into them.

```rust
#[tokio::main]
async fn main() {
    use futures::StreamExt;
    use rx_rust::{
        observable::ObservableExt, operators::creating::from_iter::FromIter,
        scheduler::runtime::tokio::TokioScheduler,
    };
    use std::time::Duration;

    let values = FromIter::new(vec![1, 2, 3])
        .delay(Duration::from_millis(5), TokioScheduler::current())
        .into_stream()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(values, [1, 2, 3]);
}
```

### Testing on virtual time

Time comes from the scheduler's clock, never from the system's directly, so a virtual clock drives
every operator. `VirtualTime` is a clock that moves only when told to, and its `scheduler()` is a
`Scheduler` like the others. `advance_by` runs every task due by then on the calling thread, each
at the instant it was due:

```rust
use rx_rust::{
    observable::ObservableExt, operators::creating::interval::Interval,
    scheduler::virtual_time::VirtualTime,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

let time = VirtualTime::new();
let values = Arc::new(Mutex::new(Vec::new()));
let values_observer = Arc::clone(&values);
let _subscription = Interval::new(Duration::from_secs(60), time.scheduler())
    .take(3)
    .subscribe_with_callback(move |value| values_observer.lock().unwrap().push(value), |_| {});

time.advance_by(Duration::from_secs(59));
assert!(values.lock().unwrap().is_empty());
time.advance_by(Duration::from_secs(1));
assert_eq!(*values.lock().unwrap(), [0]);
// Three hours pass at once, and the interval still emits each value at its own instant.
time.advance_by(Duration::from_secs(3 * 60 * 60));
assert_eq!(*values.lock().unwrap(), [0, 1, 2]);
```

## Futures and streams

A future becomes an observable with `FromFuture`, or `FromTryFuture` when its `Err` should become
the observable's error; both run the future on a scheduler. This is how a one-shot call (an HTTP
request, a database lookup) goes inside `flat_map`, `concat_map` or `switch_map`:

```rust
#[tokio::main]
async fn main() {
    use futures::StreamExt;
    use rx_rust::{
        observable::ObservableExt,
        operators::creating::{from_future::FromFuture, range::Range},
        scheduler::runtime::tokio::TokioScheduler,
    };

    async fn fetch_name(id: u32) -> String {
        format!("user-{id}")
    }

    let scheduler = TokioScheduler::current();
    let names = Range::new(1..=3)
        .concat_map(move |id| FromFuture::new(fetch_name(id), scheduler.clone()))
        .into_stream()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(names, ["user-1", "user-2", "user-3"]);
}
```

In the other direction, `into_future()` — or `into_try_future()` for a source that can fail —
resolves with the first item, as an `Option<T>` or a `Result<Option<T>, E>`, and stops the source
as soon as that item is in. Put `last()`, `collect()` or `reduce()` in front of it to pick another
item:

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

A whole sequence goes through `into_stream()` / `into_try_stream()`, and back with `FromStream` /
`FromTryStream`. There is no `request(n)`: demand-driven consumption is what `Stream` is for, and
`into_stream_with_buffer` bounds what a fast source leaves between two polls (see the
`observable::stream` module documentation).

There is no `Single`, `Maybe` or `Completable` either: a one-shot result is a `Future`.

ReactiveX     | Rust
------------- | ----
`Single<T>`   | `Future<Output = Result<T, E>>`
`Maybe<T>`    | `Future<Output = Result<Option<T>, E>>`
`Completable` | `Future<Output = Result<(), E>>`

## Subjects

A subject is both an `Observer` and an `Observable`: push into one end, subscribe to the other.

Subject                                     | Late subscribers get
------------------------------------------- | --------------------
`PublishSubject`                            | Only what is emitted after they subscribe.
`BehaviorSubject`                           | The latest value first, then everything after.
`ReplaySubject` (bounded or unbounded)      | The buffered values, then everything after; the termination is replayed too.
`AsyncSubject`                              | The last value, delivered on completion; nothing on error.
`unicast_subject::{local, shared}`          | A single-consumer pipe that buffers what is sent before the subscription.

The multicast subjects are `Clone`; clone one to keep a sending end after subscribing the other:

```rust
use rx_rust::{
    observable::ObservableExt,
    observer::{Observer, Termination},
    subject::publish_subject::PublishSubject,
};

let mut seen = Vec::new();

let subject = PublishSubject::<i32, std::convert::Infallible, _>::local();
let mut sender = subject.clone();
let subscription = subject.subscribe_with_callback(|value| seen.push(value), |_| {});

let _ = sender.on_next(1);
let _ = sender.on_next(2);
sender.on_termination(Termination::Completed);

drop(subscription);
assert_eq!(seen, [1, 2]);
```

`share()` turns a cold observable into a hot one, subscribed while it has subscribers;
`publish()`, `replay()` and `multicast()` connect manually. See the `operators::connectable`
module documentation.

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
Connectable                 | `multicast`, `publish`, `publish_behavior`, `publish_last`, `replay`, `share`, `share_behavior`, `share_last`, `share_replay`, `ConnectableController::{connect, disconnect, ref_count}`
Async conversion            | `into_future`, `into_try_future`, `into_stream`, `into_stream_with_buffer`, `into_try_stream`, `into_try_stream_with_buffer`
Type conversion             | `into_boxed`, `into_send_boxed`, `into_cloneable_boxed`, `into_send_cloneable_boxed`, `into_shared`, `with_item_type`, `with_error_type`
Debugging                   | `debug`, `debug_to_stdout`, `hook_on_subscription`, `hook_on_next`, `hook_on_termination`

## Contributing

`cargo test --test <operator>` runs the tests of one operator, and `ci/check.sh` runs what CI runs.
See [docs/testing.md](https://github.com/623637646/rx-rust/blob/master/docs/testing.md) for the
test cases every operator is expected to cover,
[docs/decisions/](https://github.com/623637646/rx-rust/tree/master/docs/decisions) for design
decisions, and [AGENTS.md](https://github.com/623637646/rx-rust/blob/master/AGENTS.md) for the
code conventions and the layout of the sources.

## License

MIT. See [LICENSE](https://github.com/623637646/rx-rust/blob/master/LICENSE).
