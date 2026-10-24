# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `distinct_until_changed` and `distinct_until_changed_with_key_selector` compare their keys with
  `PartialEq` instead of requiring `Eq`, so a key such as an `f64` works. A key that is not equal
  to itself, such as `f64::NAN`, never counts as a repeat.
- **Breaking:** `skip_until` answers its notifier `Flow::Stop` on its first item, as ReactiveX
  unsubscribes from it there: what the notifier sends afterwards, an error included, no longer
  reaches the result. A notifier that failed after opening the gate used to fail the result.

## [2.0.0] - 2026-10-21

### Added

- `ConnectableController::reset_on_error(bool)` / `reset_on_complete(bool)` and
  `RefCount::reset_on_ref_count_zero(bool)` choose when the subject of a connectable is reset: when
  its source fails, when it completes, and when the last subscriber of a ref count leaves. All
  three reset by default; `reset_on_complete(false)` keeps a completed subject, so that a late
  subscriber still gets what it replays and the source is not connected again — nor kept alive by
  that subscriber, which holds only the subject — and `reset_on_ref_count_zero(false)` keeps the
  source connected without subscribers. See
  `docs/decisions/0007-a-subject-per-round.md`.
- `ObservableExt::publish_behavior(initial_value)` and
  `ObservableExt::share_behavior(initial_value)` multicast through a `BehaviorSubject`, so every
  subscriber receives the current value — the
  initial one until the source emits — before what follows it. They complete the set next to
  `publish` / `share` (`PublishSubject`), `publish_last` / `share_last` (`AsyncSubject`) and
  `replay` / `share_replay` (`ReplaySubject`), and
  `subject::behavior_subject::BehaviorSubjectOf<'or, OE>` names the subject they build, as the
  other three subjects' `…Of` aliases do.
- `Debug` for `disposable::chain_disposal::ChainDisposal`, for the disposal types
  `delegate_disposal!` generates, whenever the disposal inside them is `Debug`, and for the
  schedulers' own disposals (`TokioDisposal`, `SmolDisposal`, `FuturesDisposal`,
  `VirtualTimeDisposal`), so that the `DisposeOnDrop` a `subscribe` or a `run_task` returns can be
  printed whatever it is built of. `ChainDisposal` is `Clone` as well, when both of its sides are.
- `scheduler::TaskHandler<TC, P>`, the `fn` type of a task's handler, is public: it is what
  `Task::new` and `Task::with_pinned` take, and its documentation states what the handler must do
  with the waker in `cx` and that the `Instant` is the scheduler's time, not `Instant::now()`.
- `observable::JoinedModeOf<A, B>`, the mode of an operator with two sources. It is an alias, so
  every type it names is unchanged.
- The other aliases for the shapes the crate's operators spell out are public as well, so that a
  caller can write the name a signature uses, and an operator written outside the crate the same
  shorthand: `observable::ChainDisposalOf<A, B>`, the disposals of the two sources `A` and `B`
  chained, `A`'s disposed first, as `merge`, `zip` and the other two-source operators chain them;
  `observable::JoinedSchedulerModeOf<OE, S>`, the `Mode` of `delay`, `debounce`, `timeout`,
  `subscribe_on`, `buffer_with_time` and `buffer_with_time_or_count`;
  `observable::ChainSchedulerDisposalOf<S, OE>`, the disposal of a task on `S` chained before that
  of the source `OE`, as the context of `delay` and `debounce` owns it;
  `observable::BoxedObserverOf<'a, OE>`, the observer the callback of
  `hook_on_subscription_boxed` gets; `utils::subscribe_with_auto_dispose_on_termination::ObserverOf`
  and `DisposalOf`, the observer and the `Disposal` of an operator built on that helper, as `take`
  and `first` are; and `operators::others::debug::DebugPrintCallback<C, T, E>`, the callback type
  of `debug_to_stdout`. They are aliases, so every type they name is unchanged.
- `scheduler::Clock`, `SystemClock` and `virtual_time::VirtualClock`: a handle on a scheduler's
  clock that can go where the scheduler cannot, such as a background producer feeding a local
  scheduler.
- `Next::PendingUntil(instant)`: a task waits for its waker or for the instant, whichever
  comes first. `drive` and `VirtualTime` support it.
- `Task::wakeable(state, step)` returns a task and the `TaskWaker` that wakes it from outside,
  from another thread than the scheduler's in the `Shared` mode: the task registers its waker
  before each step, so no wake is lost, and dropping the `TaskWaker` tells the task that nothing
  will wake it anymore. `WakeableContext` is its state type.
- `scheduler::virtual_time`: `VirtualTime`, a clock that a test moves forward with `advance_by`,
  and its `VirtualTimeScheduler`, which queues the tasks and runs them, on the calling thread, at
  the instants they are due; `VirtualTime::clock` is the virtual clock alone. Time-based tests
  become exact, synchronous and instant.
- `SchedulerTypes::clock`, the scheduler's `Clock` (`SystemClock` for the built-in schedulers).
  Every operator that measures time reads it there, so that a scheduler with a clock of its own,
  such as `VirtualTimeScheduler` (`VirtualClock`), drives the deadlines and timestamps of the
  operators it is given.
- `ObservableExt::into_shared` (`operators::others::into_shared::IntoShared`) declares an
  observable `Shared`, so that a `Local` source such as `Just` erases into the same type as a
  `Shared` one.
- `Debug` for the public types that lacked it: `thread_mode::{Local, Shared}`,
  `observer::emitter::Emitter`, `scheduler::{Task, Stepper}`, the `UnicastSender`,
  `UnicastObservable`, `BoxedUnicastObservable` and `Disposal` of `subject::unicast_subject`, the
  disposals of `amb` and `debug`, and `utils::serialized_delivery::{DropUndecided, DropDecided}`.
  With the disposals, the `DisposeOnDrop` a `subscribe` returns can be printed for those operators
  too.

### Changed

- **Breaking:** a connectable creates a new subject for each round instead of keeping one for its
  whole life. `ConnectableController::new` and `ObservableExt::multicast` take a context and a
  `fn(&mut C) -> S` subject factory instead of a subject — what the factory needs goes in the
  context, which is the third type parameter of `ConnectableController<OE, S, C = ()>` and
  `RefCount<OE, S, C = ()>` — and `observable()` returns a `MulticastObservable`, which joins the
  current subject, instead of a `SubjectObservable` pinned to one. `publish`, `publish_behavior`,
  `publish_last` and `replay` now require `Item` and `Error` to be `Clone`, since the controller
  stores the subject's mode pointer. A terminated source no longer leaves later subscribers with
  only the termination — they start a new round, with a new connection — and the current value of
  `share_behavior` or the buffer of `share_replay` no longer leaks into the next connection, so
  `share_replay` and `share_last` start over once the source terminates. The `reset_on_*` options
  above keep the previous behavior where it was wanted.
- **Breaking:** `observer::emitter::Emitter<OR, M>` is now `Emitter<M, OR>`, the mode first, like
  every other type that takes one (`ReplaceableDisposal<M, D>`, the operators' observers). The
  callbacks of `Create::local` / `shared` and of `hook_on_subscription` / `hook_on_termination`
  infer it, so only code naming the type itself changes.
- **Breaking:** disposal and adapter APIs now name their behavior explicitly:
  `disposable::shared_disposal::SharedDisposal` becomes
  `disposable::replaceable_disposal::ReplaceableDisposal`, with `replace_with` instead of
  `replace`. Its `D` is the inner disposal: the slot holds the `DisposeOnDrop<D>` that
  `replace_with`'s builder returns, as `subscribe` and `run_task` do, so dropping its last clone
  disposes it; `DisposeOnDrop::map_into` becomes `map_inner_into`. `Subject::terminated` becomes
  `termination`. The stream conversions are
  `into_stream_with_buffer` and `into_try_stream_with_buffer`. Debug printing uses
  `Debug::with_stdout` and `ObservableExt::debug_to_stdout` instead of
  `DefaultPrintType`, `new_default_print` and `debug_default_print`. The crate-private
  `utils::subscription_slot::SubscriptionSlot` becomes `utils::reservation_slot::ReservationSlot`.
  The stdout debug printer labels subscription events `Subscribed`.
- **Breaking:** `disposable::bound_drop_disposal::BoundDropDisposal` is now
  `disposable::dispose_on_drop::DisposeOnDrop`, and the `observable::Subscription` alias is
  removed. Observable and scheduler APIs return `DisposeOnDrop<D>` directly.
  `DisposableExt::into_subscription` becomes `into_dispose_on_drop`, and
  `preceded_by_bound` becomes `preceded_by_wrapped`; disposal behavior is unchanged.
- **Breaking:** `Interval` emits its first value one period after the subscription, as ReactiveX
  does, instead of at once. `Interval::new(period, scheduler)` replaces
  `Interval::new(period, scheduler, delay)`, and `Interval::with_initial_delay(initial_delay,
  period, scheduler)` takes an initial delay of its own. The former behavior is
  `Interval::with_initial_delay(Duration::ZERO, period, scheduler)` for `None`, and
  `with_initial_delay(delay, period, scheduler)` for `Some(delay)`. The old three-argument call no
  longer compiles, so no caller changes timing silently.
- **Breaking:** `buffer_with_time` and `buffer_with_time_or_count` (and `BufferWithTime::new`,
  `BufferWithTimeOrCount::new`) lose their `delay: Option<Duration>` argument and time their first
  bundle one `time_span` after the subscription, as ReactiveX does. With `None`, the first bundle
  used to be emitted at once, and so empty.
- **Breaking:** `throttle`, `timestamp` and `time_interval` take a `Clock`, not a scheduler, since
  they schedule nothing: `.throttle(span, scheduler.clock())`, `.timestamp(SystemClock)`,
  `.time_interval(time.clock())` on a `VirtualTime`. The operator is then `Send` whatever
  scheduler the clock comes from, and needs no runtime feature. The observer of `timeout` holds
  only the scheduler's `Clock`, so a local scheduler can time a background source.
- **Breaking:** `SchedulerTypes` requires `type Clock: Clock` and `fn clock(&self)`, the only
  source of its time: there is no `now` method on the scheduler, `scheduler.clock().now()` reads
  it. A custom scheduler on the system's clock adds `type Clock = SystemClock` and
  `fn clock(&self) -> SystemClock { SystemClock }`.
- **Breaking:** a task handler returns a `Next` (formerly `TaskState`) rather than a
  `Poll<TaskState>`, and so does `Stepper::step`: `Poll::Pending` is `Next::Pending`, and `Next::PendingUntil` is a
  `Pending` with a deadline. A handler that forwards a poll matches on it
  (`let Poll::Ready(output) = future.poll(cx) else { return Next::Pending };`). A scheduler
  that steps tasks itself handles the two new variants.
- **Breaking:** a task step is given the scheduler's time: `TaskHandler` takes a fourth argument,
  `now: Instant`, and so does `Stepper::step`. A `Next::SleepUntil` is an instant on that
  clock. `drive` takes the clock of the scheduler
  it runs for (`drive(task, delay, scheduler.clock(), sleep)`), whose time it passes to every
  step.
- A `delay` given to `Scheduler::run_task` that is too long for an `Instant` to represent never
  ends: the task is never started and is kept until it is disposed. Tokio's timer used to fire it
  after some thirty years.
- **Breaking:** `utils::pending_events::EventBatch` moved to `observer::EventBatch`, next to
  `Event` and `Termination`.
- **Breaking:** the crate-internal parts of `utils` are no longer public:
  `utils::id_generator`, `utils::on_panic`, `utils::pending_events` (`PendingEvents`),
  `utils::subscription_slot`, `utils::serialized_multicast` (`SerializedMulticast`, `Admission`,
  `MulticastDisposal`) and `utils::resubscribe` (`Resubscribe`). They appear in no public
  signature: the multicast is reached through the subjects it is made of, and the re-subscription
  through `retry` and `concat_all`. The helpers for writing an operator
  (`subscribe_with_context`, `subscribe_with_auto_dispose_on_termination`,
  `serialized_delivery`) stay public.
- The observer type each operator subscribes its source with (`MapObserver`, `FilterObserver`,
  `TakeObserver`, … — 75 of them) is `#[doc(hidden)]`. They remain `pub`, because the non-boxed
  `create`, `hook_on_subscription` and `hook_on_termination` hand the downstream observer to a
  user closure whose signature names the type, so this is a documentation change, not an API one:
  nothing that compiled before stops compiling. The crate now denies `missing_docs`, so a public
  item is either documented or hidden.
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
  runs a nameable `Task` (a context, an optional future or stream, and a `fn` handler), built with
  `Task::new`, `with_pinned`, `once`, `periodic`, `wakeable`, `from_future_then` or
  `from_stream_then`. The closure-based `schedule`, `schedule_recursively`,
  `schedule_periodically`, `spawn_future` and `schedule_stream` are removed: the task's state
  carries what the closure captured (`scheduler.run_task(Task::once(state, |state| …), delay)`,
  `Task::periodic`, or `Task::new` for a step that decides when it runs next), and code that holds
  a concrete runtime's scheduler can spawn on that runtime directly. `sleep` is removed too: an
  operator schedules a task, and a test on `VirtualTime` advances its clock.
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
  runtime: `scheduler::tokio_scheduler` is `scheduler::runtime::tokio`, and likewise `smol`. The `thread-pool-scheduler` and `local-pool-scheduler` features are merged into
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
  and `SmolLocalScheduler::from_executor(&executor)` over a `LocalExecutor`. A scheduler built from an executor holds it
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
  and with it the downstream observer, and its disposal must be a handle to the task that does not
  keep the task's state alive; see `SchedulerTypes::Disposal`. The built-in schedulers do both.
- **Breaking:** `utils::subscribe_with_context` has one entry point: the context always owns the
  source subscription and disposes it as soon as it stops. `subscribe_with_context_owning_source`
  is renamed `subscribe_with_context`, which replaces the non-owning one, and `ContextDisposal` is
  renamed `Disposal`, which replaces the old chained one; `SubscriptionContext` has no default for
  its source disposal any more. `observe_on`, `delay` and `debounce` now dispose their source when
  downstream answers `Flow::Stop` from their scheduler task, instead of when the source next sends
  — and so on that task's thread, as after delivering a termination. In exchange, their source's
  disposal must be `Send + 'static` where the scheduler's tasks must be, as `timeout`'s already
  was. See `docs/decisions/0005-the-context-owns-its-source.md`.
- **Breaking:** the observers that `delay`, `debounce`, `observe_on` and
  `buffer_with_time_or_count` give their source no longer hold the scheduler, so a `Shared`
  source, which may send from another thread, builds with a single-threaded scheduler:
  `observe_on(TokioLocalScheduler::ambient())` moves the events of a background source onto a
  `LocalSet`. `delay`, `debounce` and `observe_on` run one task for the whole subscription, started
  when they are subscribed and parked on `Next::Pending` / `Next::PendingUntil` while nothing is
  due, instead of one per burst of events; the source's observer wakes it (`Task::wakeable`). A
  custom scheduler must step a task again once its waker is woken, as the built-in ones do.
  `DelayObserver`, `DebounceObserver` and `BufferWithTimeOrCountObserver` take the clock's type
  instead of the scheduler's, and `ObserveOnObserver` takes neither.
- **Breaking:** the task of `observe_on` owns the downstream observer, and the observer it gives
  its source holds only the queued events and the task's waker: `ObserveOnObserver<M, T, E>` names
  neither the downstream observer nor the source's disposal. The downstream observer never leaves
  the scheduler's thread, so with a single-threaded scheduler it need not be `Send`, even behind a
  `Shared` source: a background source observed on a `LocalSet` by callbacks that hold `Rc`s
  builds. A disposal still stops the delivery at once, but releases that observer only when the
  scheduler drops the cancelled task, on the scheduler's thread. A task dropped by its runtime
  before it finished now disposes the source. See
  `docs/decisions/0006-observe-on-task-owns-its-observer.md`.
- **Breaking:** behind a source that requires a `Send` observer — any in the `Shared` mode —
  `retry` requires its error type and its source's disposal to be `Send`. An error that arrives on
  another thread while a subscription is being made waits for it to return and is retried by the
  thread that made it, and the source's subscription is disposed by the first retry, on whichever
  thread runs it. The `Local` mode has neither requirement.
- Fewer allocations in the buffering operators. `buffer_with_count` prepares each bundle with
  room for `count` items, as `buffer_with_time_or_count` does, so a full bundle is built with a
  single allocation. `buffer`, `buffer_with_time` and `buffer_with_time_or_count` emit an empty
  bundle as a new `Vec`, which allocates nothing, and keep the room already prepared for the next
  one. `observe_on` leaves its queue, each time it takes the values out to deliver them, with as
  much room as they took, so the values queued meanwhile do not grow it from nothing.
- `debug_to_stdout` prints a disposal as `Disposed`, the name of its `DebugEvent`, instead of
  `Dispose`.

### Fixed

- `switch` drops errors from stale inner observables outside its context lock, so an error's
  destructor can re-enter the operator without panicking or deadlocking.

- `retry` no longer overflows the stack when its sources keep failing synchronously: an observable
  that fails while it is being subscribed is retried once its `subscribe` has returned, in a loop,
  instead of from inside it, so each retry used to nest one subscription deeper. The subscription
  of the observable that failed is now disposed before the next one is subscribed, rather than
  after — and so is the source's, which used to be kept until the whole subscription was disposed.

- A `RefCount` whose source panics while it is connected or disconnected — the subject factory,
  cloning the source, subscribing to it or disposing that subscription — keeps working once the
  panic is caught: its controller is given back disconnected and the subscriber that was
  connecting is counted out, so the next subscriber connects again. The controller used to be
  lost, and the `RefCount` never connected again. A thread that unwinds never connects the source
  — a panic there would abort the process — so subscribers still waiting for a connection get it
  from the next subscription or disposal.

- A subject that delivers from a `Drop` run by an unwinding panic — the disposal of a subscription
  the unwind drops, say — is no longer killed as if one of its observers had panicked: the guards
  that undo a panicking callback mistook the unwind already under way for one of their own.

- `observe_on` delivers an error before the values still queued when it was sent, even when it
  comes while the task is delivering a batch of earlier values: the rest of that batch used to be
  delivered first.

- A delay, period, time span or timeout too long for an `Instant` to represent no longer panics on
  overflow; its deadline is never reached instead. `delay` never delivers the value, nor anything
  after it, `timeout` never fires, `debounce` never emits the pending value, and `interval` and
  the `buffer_with_time*` timers stop.

### Removed

- **Breaking:** the `disposable::empty_disposal` module. It only held `impl Disposable for ()`,
  which is a trait implementation and stays in effect; nothing could be named through the path.
- **Breaking:** the `async-std-scheduler` feature and `AsyncStdScheduler`. async-std is
  discontinued upstream, which points to smol instead: use `smol-scheduler` and
  `SmolScheduler::global()`.

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

[Unreleased]: https://github.com/623637646/rx-rust/compare/2.0.0...HEAD
[2.0.0]: https://github.com/623637646/rx-rust/compare/1.0.1...2.0.0
[1.0.1]: https://github.com/623637646/rx-rust/compare/1.0.0...1.0.1
[1.0.0]: https://github.com/623637646/rx-rust/compare/0.3.0...1.0.0
[0.3.0]: https://github.com/623637646/rx-rust/compare/0.2.2...0.3.0
[0.2.2]: https://github.com/623637646/rx-rust/compare/0.2.1...0.2.2
[0.2.1]: https://github.com/623637646/rx-rust/compare/0.2.0...0.2.1
[0.2.0]: https://github.com/623637646/rx-rust/releases/tag/0.2.0
