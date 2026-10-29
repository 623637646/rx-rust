# rx-rust and rxRust

[rxRust](https://github.com/rxRust/rxRust) is the older ReactiveX library for Rust. The two share
their central idea: single- or multi-threaded is chosen in the type, `Local` (`Rc<RefCell<_>>`) or
`Shared` (`Arc<Mutex<_>>`), so a pipeline that stays on its thread pays for no lock and asks
nothing to be `Send`. They differ in where a scheduler comes from, how a consumer stops a source,
what an item may be, and which operators exist.

This page compares rx-rust 2.0 with rxRust 1.0.0-rc.6 (October 2026).

## At a glance

Aspect            | rx-rust | rxRust
----------------- | ------- | ------
Thread mode       | a `Mode` on every observable; an operator with several sources is `Shared` as soon as one of them is | a `Local` or `Shared` context the chain starts from (`Local::from_iter(…)`)
Schedulers        | passed by value to each time-based operator; Tokio, smol or the `futures` executors, each behind a feature, none by default | part of the context's type, built with `Default` and spawned on the ambient Tokio runtime (an `_with` variant takes one); browser timers on wasm, or your own
Stopping a source | `on_next` returns `Flow::Stop`                     | the source asks `observer.is_closed()`
Borrowed items    | items are owned                                    | `&mut T` can be lent to each subscriber in turn (`multicast_mut_ref`)
Virtual time      | `VirtualTime`, a value: any number per test        | `TestScheduler`, one clock per thread
`unsafe`          | `#![forbid(unsafe_code)]`                          | a few `Pin` projections
Platforms         | no wasm target                                     | wasm out of the box

## Thread mode

Both libraries pick `Rc` or `Arc` from a type-level marker, and differ in where the marker sits.

In rxRust the marker is the **context** a chain starts from: `Local::from_iter(0..10)` or
`Shared::from_iter(0..10)`, and every operator after it inherits that context. In rx-rust it is
the **`Mode` of each observable**, chosen by its source (`Create::local` / `Create::shared`,
`PublishSubject::local()` / `PublishSubject::shared()`; plain values such as `Just` are `Local`).
An operator with several sources joins their modes, and a scheduler fixes the mode of what runs on
it, so a `Local` value source merged with a `Shared` subject is a `Shared` pipeline without being
written so. See [decision 0002](decisions/0002-observable-types-and-thread-mode.md).

## Schedulers

This is the difference a user meets first. In rxRust a timer takes no scheduler:

```rust
// rxRust
Local::timer(Duration::from_millis(100)).subscribe(|_| println!("tick"));
```

The scheduler is still there, in the type. `Local<T>` is an alias of `LocalCtx<T, LocalScheduler>`,
the context requires `Scheduler: Clone + Default`, and `timer` builds one with
`Self::Scheduler::default()`. `LocalScheduler` is a unit struct that owns no runtime: it calls
`tokio::task::spawn_local` and `tokio::time::sleep`, and `SharedScheduler` calls `tokio::spawn`.
The runtime is whichever one the calling thread is in, which is why rxRust's examples run inside
`#[tokio::main(flavor = "local")]`; outside of it, Tokio panics when the task is spawned. Another
runtime takes `timer_with(delay, scheduler)` at each call, or a context alias with a scheduler of
one's own, `LocalCtx<T, MyScheduler>`.

rx-rust asks for the scheduler at every time-based operator:

```rust
// rx-rust
let _subscription = Interval::new(Duration::from_millis(100), TokioScheduler::current())
    .subscribe_with_callback(|_| println!("tick"), |_| {});
```

That is one argument more, and it buys three things:

- **Nothing is ambient.** A pipeline runs where its scheduler sends it, so one program can drive
  pipelines on different runtimes, and code that builds a pipeline names its runtime dependency
  in its signature instead of panicking when called from the wrong thread.
- **Every pipeline can run on virtual time.** Code that takes a scheduler can be given
  `time.scheduler()` in a test (see [Virtual time](#virtual-time)). Code that reaches for an
  ambient default has no such seam.
- **The runtime is a feature, not a default.** No feature pulls in a runtime unless asked for:
  `tokio-scheduler`, `smol-scheduler` and `futures-scheduler` can be combined, and enabling one
  more changes no code that already compiles.

### Why rx-rust has no default scheduler

A default scheduler chosen by the enabled features — Tokio when `tokio-scheduler` is the only
scheduler feature, a compile error asking to pick one when there are several — looks like the best
of both, and was rejected:

- **Cargo features are unified across the dependency graph.** If a user's code relied on the
  default, a new dependency that enables `rx-rust/smol-scheduler` would break it, with the error
  in the user's code and nothing pointing at the dependency. Features must be additive: enabling
  one more must not make code stop compiling.
- **One feature still has two schedulers.** `tokio-scheduler` holds `TokioScheduler` (`Shared`)
  and `TokioLocalScheduler` (`Local`). A default would have to follow the source's mode, so `Range`
  and `Just`, which are `Local`, would land on `TokioLocalScheduler`, and panic outside a
  `LocalSet`.
- **It doubles the API.** Rust has neither overloading nor default arguments, so each of the
  thirteen places that take a scheduler — `delay`, `debounce`, `timeout`, `buffer_with_time`,
  `buffer_with_time_or_count`, `observe_on`, `subscribe_on`, `Interval`, `Timer`, `FromFuture`,
  `FromTryFuture`, `FromStream`, `FromTryStream` — would need a second form.
- **The saving is small.** It is one argument, `TokioScheduler::current()`, and `TokioScheduler`
  already implements `Default`.

If a shorter form is ever added, it will be chosen by an import rather than by a feature: a module
of each runtime offering, say, `interval(period)` and a `delay_tokio(span)` method. Importing two of
them conflicts in nothing, so it stays additive; what it gives up, the virtual-time seam, is
then visible at the import.

## Stopping a source

An infinite synchronous source, such as a range with no end, never returns from `subscribe`, so
there is no subscription to drop yet. Each library lets the observer stop it.

In rxRust the source asks: `Observer::is_closed()` returns `true` once the observer takes nothing
more, and `from_iter` checks it before each item. In rx-rust the observer answers:
`Observer::on_next` returns `Flow::Stop`, a promise that it accepts nothing more, not even a
termination, and the source stops pushing and drops it. An operator forwards its downstream's
answer, so `take`, `first` and `take_until` stop their source the same way, and the
`#[must_use]` on `Flow` catches an operator that forgets to forward it.

## Borrowed items

rxRust's item type is a generic associated type with a lifetime, so an observable can lend data it
owns as `&mut T` instead of handing it over, and a subject can lend the same `&mut` to each
subscriber in turn (`subject_mut_ref`, `multicast_mut_ref`). That saves a clone of large items.

rx-rust items are owned: a subject clones each item for every subscriber, so a large item goes
behind an `Arc`, whose clone is a reference count, and one that subscribers mutate goes behind an
`Arc<Mutex<_>>`. Lending was left out because only synchronous pass-through operators could take a
borrowed item — anything that stores one (`buffer`, `replay`, `zip`, `combine_latest`, `delay`)
or crosses a scheduler cannot — while every trait, boxed observer and multicast operator would
need a second, borrowing form. See [decision 0001](decisions/0001-no-borrowed-items.md).

## Virtual time

Both libraries test time-based pipelines on a clock that moves only when told to. rxRust's
`TestScheduler` is one clock per thread, driven by associated functions (`TestScheduler::init()`,
`TestScheduler::advance_by`). rx-rust's `VirtualTime` is a value: a test holds as many as it
needs, gives `time.scheduler()` to the operators, and calls `time.advance_by`, synchronously and
without a runtime. Every operator reads time from its scheduler's clock and never from
`Instant::now()`, which CI checks, so the virtual clock drives all of them.

## Operators

Both cover most of the [ReactiveX operators](https://reactivex.io/documentation/operators.html).

- **rx-rust only**: `catch`, `timeout`, `window`, `window_with_count`, `amb_with`,
  `sequence_equal`, `materialize`, `dematerialize`, `timestamp`, `time_interval`, `replay`,
  `share_replay`, `publish_last`, `ReplaySubject`, `AsyncSubject`.
- **rxRust only**: `with_latest_from`, `pairwise`, `filter_map`, `delay_subscription`.

## Which one to use

Situation                                                                 | Use
------------------------------------------------------------------------- | -------
A WebAssembly target                                                      | rxRust
Large items lent to several subscribers without a clone                   | rxRust
Pipelines on smol or the `futures` executors, or several runtimes at once | rx-rust
Error recovery and replay: `catch`, `timeout`, `replay`, `ReplaySubject`  | rx-rust
Time-based logic tested on several independent virtual clocks             | rx-rust
No `unsafe` in the dependency                                             | rx-rust
