# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
- `educe` and `futures` are required at their actual 0.x minor (`0.8` / `0.3`) instead of any
  `0.*`.
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

[Unreleased]: https://github.com/623637646/rx-rust/compare/1.0.0...HEAD
[1.0.0]: https://github.com/623637646/rx-rust/compare/0.3.0...1.0.0
[0.3.0]: https://github.com/623637646/rx-rust/compare/0.2.2...0.3.0
[0.2.2]: https://github.com/623637646/rx-rust/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/623637646/rx-rust/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/623637646/rx-rust/releases/tag/0.2.0
