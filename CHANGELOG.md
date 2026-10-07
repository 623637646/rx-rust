# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `utils::lazy_subscription::LazySubscription`, the subscription of a future or stream adapter:
  made on the first poll, released when the adapter is over.
- `ObservableExt::into_shared` (`operators::others::into_shared::IntoShared`) declares an
  observable `Shared`, so that a `Local` source such as `Just` erases into the same type as a
  `Shared` one, and a chain of `Local` sources can be erased into a `Send` box.

### Changed

- **Breaking:** the MSRV is 1.89 (was 1.88), for educe 0.8 (was 0.6). educe 0.6 bounded a derived
  `Eq` by `PartialEq` only, which made `Termination<f64>`, `Event<f64, _>`, `EventBatch`,
  `DebugEvent` and `timeout::Error` `Eq` over a type that is not; they now require `Eq`.
- The derived `Debug` of erased types, subjects and internal handles prints the type's name only,
  instead of `std::any::type_name` with its path and type arguments.
- **Breaking:** `Observable<'or, T, E>` is split into `ObservableTypes`, with the associated types
  `Item`, `Error`, `Mode` and `Disposal`, and `Observable<OR>`, whose `subscribe` takes the
  observer type as a trait parameter. There is no `'or` lifetime anymore, and the disposal never
  names the observer. A custom observable implements both traits and names, in its bounds, the
  observer it subscribes its source with. See
  `docs/decisions/0002-observable-types-and-thread-mode.md`.
- **Breaking:** the thread mode is part of the type instead of a build setting. Every observable
  declares `Mode = thread_mode::Local` (`Rc<RefCell<_>>`, nothing needs to be `Send`) or
  `thread_mode::Shared` (`Arc<Mutex<_>>`); sources that can be either have `local` / `shared`
  constructors (`Create`, the subjects), and operators over several sources take the joined mode.
  `Send` is only required where a value crosses threads.
- **Breaking:** the `single-threaded` feature and `utils::types` (`Shared`, `Mutable`, `MaybeSend`,
  `MaybeSync`) are removed. All features are additive: the scheduler features can be enabled
  together, and `local-pool-scheduler` no longer excludes the others.
- **Breaking:** `Scheduler` is now `Scheduler<TC, P>` over `SchedulerTypes { Mode, Disposal }` and
  runs a nameable `Task` (a context, an optional future or stream, and a `fn` handler); the
  closure-based `schedule`, `schedule_periodically`, `spawn_future`, … moved to `SchedulerExt`,
  which every scheduler implements.
- **Breaking:** type erasure is explicit: `into_boxed` / `into_cloneable_boxed` erase without
  `Send`, `into_send_boxed` / `into_send_cloneable_boxed` (and `SendBoxedObserver`,
  `SendBoxedDisposal`) keep it, and `into_boxed_for` / `into_send_boxed_for` fix the observer type.
- **Breaking:** `Create::local` / `Create::shared` hand the builder an `Emitter`, the unboxed
  downstream observer (no allocation, static dispatch); such a `Create` subscribes one observer
  type only. The former behavior, a boxed observer and a `Create` that subscribes any observer, is
  `Create::local_boxed` / `Create::shared_boxed`. Both forms are one type,
  `Create<T, E, D, F, M, const BOXED: bool = false>`.
- **Breaking:** the lock helpers moved from `utils::mutable` to `thread_mode::mutable`, next to the
  modes that pick the locks; `thread_mode` depends on nothing else in the crate. The boxed observer
  of a mode is `ObserverMode::BoxedObserver`, and `IntoBoxedObserver` lives with it in
  `observer::boxed_observer`; box with `M::boxed(observer)` (was `observer.into_mode_boxed()`).
- **Breaking:** Tokio's multi-threaded scheduler is `TokioScheduler` instead of
  `tokio::runtime::Handle` itself: `TokioScheduler::current()` (or `Default`) takes the current
  runtime's handle, `try_current()` returns `None` outside of a runtime, and
  `from_handle(handle)` wraps a given one.
- **Breaking:** `SmolScheduler` is no longer a unit struct: `SmolScheduler::global()` (or
  `Default`) is the former value, and `SmolScheduler::from_executor(&executor)` runs on an
  `Executor` of your own.
- **Breaking:** the scheduler implementations moved under `scheduler::runtime`, one module per
  runtime: `scheduler::tokio_scheduler` is `scheduler::runtime::tokio`, and likewise `smol` and
  `async_std`. The `thread-pool-scheduler` and `local-pool-scheduler` features are merged into
  `futures-scheduler`, whose executors are wrapped like the others:
  `ThreadPoolScheduler::from_pool(pool)` and `LocalPoolScheduler::from_spawner(spawner)` instead of
  `ThreadPool` and `LocalSpawner` themselves.
- **Breaking:** each scheduler's disposal is a type of the crate: `TokioDisposal`, `SmolDisposal`,
  and `FuturesDisposal` for both `futures` schedulers (was `ThreadPoolDisposal` and
  `LocalSpawnerDisposal`), instead of `tokio::task::JoinHandle<()>` and `smol::Task<()>`;
  `Disposable` is no longer implemented for those two runtime types.
- New single-threaded (`Local`) schedulers, whose tasks need not be `Send`:
  `TokioLocalScheduler` — `ambient()` for the `LocalSet` the calling thread runs,
  `from_local_set(&local_set)` for a given one, which also accepts tasks before it runs —,
  and `SmolLocalScheduler::from_executor(&executor)` over a `LocalExecutor`. async-std has none:
  its `spawn_local` needs its `unstable` feature. A scheduler built from an executor holds it
  weakly, so a pending task does not keep its executor alive; running a task after the executor is
  dropped panics.
- **Breaking:** `publish`, `replay`, `share`, … multicast through a subject of the source's mode;
  `share*` require `Item: Clone` and `Error: Clone`.
- After a dispose, a context-based operator releases its observer as the source drops its handle on
  the context, which is in practice before `dispose` returns, even while a scheduler task still
  holds another; nothing is delivered after the dispose either way. `delay` and `observe_on` keep
  delivering the values already scheduled when their source terminates.
- A source that drops its observer without a termination has stopped sending; it no longer
  disposes the subscription. The work the operator already accepted runs its course, and the
  downstream observer is then dropped, never terminated: `observe_on` and `delay` deliver the
  values they hold, `debounce` emits its pending value when the quiet period ends, `timeout` fails
  with `Error::Timeout` (so does `Never.timeout(…)`, which never fired before), and
  `buffer_with_time` / `buffer_with_time_or_count` keep cutting buffers until the subscription is
  disposed. Before, all of that was dropped, and what reached the observer depended on timing. See
  `docs/decisions/0004-a-source-that-drops-its-observer.md`.
- **Breaking:** `utils::subscribe_with_context::WeakSubscriptionContext`,
  `SubscriptionContext::downgrade`, `utils::serialized_delivery::WeakSerializedDelivery` and
  `SerializedDelivery::downgrade` are removed. A scheduler task holds a clone of the context, which
  no longer keeps a disposed observer alive: a handle dropped after a disposal releases it. Nothing
  else in the crate needed a weak handle; a reference cycle through a delivery is broken when it
  stops, as the one through a source subscription always was.
- A `Scheduler` should drop a disposed task promptly, since the task may hold an operator's context
  and with it the downstream observer; see `SchedulerTypes::Disposal`. The built-in schedulers do.
- **Breaking:** `utils::subscribe_with_context` has one entry point: the context always owns the
  source subscription and disposes it as soon as it stops. `subscribe_with_context_owning_source`
  is renamed `subscribe_with_context`, which replaces the non-owning one, and `ContextDisposal` is
  renamed `Disposal`, which replaces the old chained one; `SubscriptionContext` has no default for
  its source disposal any more. `observe_on`, `delay` and `debounce` now dispose their source when
  downstream answers `Flow::Stop` from their scheduler task, instead of when the source next sends
  — and so on that task's thread, as after delivering a termination. In exchange, their source's
  disposal must be `Send + 'static` where the scheduler's tasks must be, as `timeout`'s already
  was. See `docs/decisions/0005-the-context-owns-its-source.md`.

## [1.0.1] - 2026-09-14

### Added

- GitHub Actions: `ci.yml` (rustfmt, clippy, `cargo hack` check of every feature, MSRV, docs and
  doctests, nextest per scheduler feature) and `release.yml` (publishes a pushed version tag to
  crates.io through Trusted Publishing and creates the GitHub Release from this file).
- A `ci` nextest profile that retries timing-sensitive tests and reports flaky ones.

### Changed

- `Cargo.toml` metadata: `license = "MIT"` (was `license-file`), plus `keywords` and `categories`
  for crates.io.
- The published crate now contains only `src/`, `Cargo.toml`, `LICENSE` and `README.md`
  (`include` whitelist); tests, editor settings and contributor docs stay in the repository.
- `educe` and `futures` are required at their actual 0.x minor (`0.6` / `0.3`) instead of any
  `0.*`. educe stays on 0.6: 0.7+ needs rustc 1.89, above this crate's MSRV.
- `DebugEvent<'a, T, E>` spells out the `T: 'a, E: 'a` bounds that its reference fields already
  implied. No caller is affected.
- The `paste` dev-dependency (archived upstream) is replaced with the drop-in `pastey`.

### Fixed

- README install snippet said `0.3`; it now matches the released `1.0`.
- README `LICENSE` links were relative and resolved nowhere on docs.rs / crates.io; they now point
  at the file on GitHub.

## [1.0.0] - 2026-09-14

First stable release. Most of the crate was reworked between 0.3 and 1.0, so this entry lists the
themes rather than every change.

### Added

- `Observer::on_next` returns `Flow` (`Continue` / `Stop`), letting an observer stop its source
  synchronously; every operator forwards the answer back to the source.
- `Stream` / `Future` bridges: `from_try_future`, `from_try_stream`, `observable_future`,
  `observable_try_future`, `observable_try_stream`, and `into_stream_with` with the `StreamBuffer`
  strategies (`Latest`, `Bounded`, or your own) that bound what a `Stream` keeps between two polls.
- `smol` runtime support (`smol-scheduler` feature).
- `unicast_subject()` / `unicast_subject_with_capacity()`: a single-consumer channel-like subject
  (`UnicastSender` + `UnicastObservable`).
- Operators: `collect`, `map_err`, `with_error_type`, `with_item_type`.
- `EitherDisposal`; `delegate_disposal!` usable outside the crate.
- Operator-building helpers `subscribe_with_context`, `subscribe_with_context_owning_source` and
  `subscribe_with_auto_dispose_on_termination`, with `SerializedDelivery` / `SerializedMulticast`
  underneath; nearly every stateful operator and all subjects are built on them.

### Changed

- `Subscription` is typed by its disposal (`Subscription<D>`) instead of lifetime-erased.
- `NecessarySend` / `NecessarySendSync` are now `MaybeSend` / `MaybeSync` in `utils::types`, next
  to `Shared` / `Mutable`, so the same code compiles in single-threaded and multi-threaded builds.
- `ConnectableObservable` uses a typestate and can be reconnected; `RefCount` connects outside its
  lock.
- `Scheduler` reworked: an associated disposal type (`Scheduler::D`), cooperative yielding,
  correct Tokio runtime context.
- `MutableHelper` lost the `safe_lock_*` family; locks are reached through `with_mut` / `with_ref`
  and the one-shot `MutableExt` helpers.

### Removed

- `on_backpressure`, `on_backpressure_buffer`, `on_backpressure_latest`. Backpressure is handled on
  the `Stream` side via `into_stream_with` (see the README's "Backpressure" section).

### Fixed

- Race conditions in `switch`, `catch`, `unicast_subject`, `subscribe_with_context`,
  `ConnectableObservable` / `RefCount`, and around `MutableBool`.
- `skip_until` / `take_until` now complete when the notifier completes without emitting.
- `concat` disposes its second source on unsubscription; `concat_all` / `merge_all` / `switch`
  carry the right error type from `from_iter`.
- `retry` with a synchronously throwing source; `replay_subject` replays before an error;
  `from_stream` stops polling once the observer stops.

## [0.3.0] - 2025-12-17

### Added

- `CloneableBoxedObservable`.

### Changed

- `Subscription` performance improvements.
- `NecessarySend` renamed back to `NecessarySendSync`, with `Sync` included again.

## [0.2.2] - 2025-12-13

### Changed

- `NecessarySendSync` renamed to `NecessarySend`; the `Sync` bound was dropped.

### Fixed

- Compile error in `debug.rs`.

## [0.2.1] - 2025-11-28

### Added

- `Debug` implementations for public structs.

## [0.2.0] - 2025-11-13

Initial public release on crates.io.

[Unreleased]: https://github.com/623637646/rx-rust/compare/1.0.1...HEAD
[1.0.1]: https://github.com/623637646/rx-rust/compare/1.0.0...1.0.1
[1.0.0]: https://github.com/623637646/rx-rust/compare/0.3.0...1.0.0
[0.3.0]: https://github.com/623637646/rx-rust/compare/0.2.2...0.3.0
[0.2.2]: https://github.com/623637646/rx-rust/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/623637646/rx-rust/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/623637646/rx-rust/releases/tag/0.2.0
