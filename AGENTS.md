# AGENTS.md

Operational notes for AI coding agents working in this repository. Human-facing docs live in
[README.md](README.md) (usage) and [docs/](docs/): `testing.md` (the test checklist) and
`decisions/` (design decisions that were made or rejected).

## What this is

`rx-rust` is a ReactiveX implementation for Rust: `Observable` / `Observer` / `Disposable` plus a
large set of operators. Edition 2024, MSRV 1.89, `#![forbid(unsafe_code)]`, zero required runtime
dependency — the async runtime is selected by feature flag.

## Testing: run the smallest thing that answers the question

The suite is large (~120 integration test files, ~2400 test functions) and many tests are timing
based, so a full run is slow and wasteful. **Default to the narrowest scope, and widen only when the
change justifies it.**

1. One test — the normal case while iterating:
   ```bash
   cargo test --test map test_completed
   ```
2. One file — after finishing an operator:
   ```bash
   cargo test --test map
   ```
3. A few related files — only when the change is cross-cutting (e.g. a `utils/` change):
   ```bash
   cargo test --test switch --test merge_all --test subscribe_with_context
   ```
4. Full suite — **only when the user explicitly asks for it**, or before a release. Never run it
   "just to be safe" after a local change. It is one build, run with nextest (see
   `.config/nextest.toml`):
   ```bash
   cargo nextest run
   ```

Rules of thumb:

- Compile errors are cheaper to find than test failures: use `cargo check --tests` when you only
  need to know whether it builds.
- Changing `src/operators/<group>/<name>.rs` → run `tests/<name>.rs`. The mapping is one-to-one for
  nearly every operator and subject.
- Do not re-run a test that already passed unless the code under it changed.
- `cargo nextest run` also takes the `--test` filters above, and fails tests that leak; prefer it
  over `cargo test` when a test may hang.
- `RX_TEST_SCHEDULERS=tokio` (see below) runs a test on one scheduler instead of six, which is
  enough while iterating on code that is not scheduler specific.

## Features

The tests need no `--features`: `Cargo.toml` has a dev-dependency on the crate itself that enables
every scheduler feature, so `cargo test` builds them all. The features only matter for the library
(`cargo check --lib --features …`, `cargo hack --each-feature --no-dev-deps check --lib`).

`block_on` in `tests/tests_utils/test_scheduler.rs` runs the test body once on every scheduler of
the crate, in this order: `tokio`, `tokio-local` (`TokioLocalScheduler` on a `LocalSet`), `smol`,
`smol-local`, `thread-pool`, `local-pool`. The body is an `Fn` that gets a `TestScheduler`, called
once per scheduler. A failing run prints
`the test panicked on the <name> scheduler`; `RX_TEST_SCHEDULERS=smol,local-pool` (comma-separated
names from that list) restricts the loop to some of them.

`TestScheduler` declares the `Shared` mode for every variant, the single-threaded ones included,
so the suite is built in `Shared` mode only. The single-threaded schedulers are not `Send`, so
their variants find the scheduler in a thread-local that `block_on` sets on the thread driving
their executor; using one from another thread panics. The `Local` mode is covered by
`tests/local_mode.rs` (non-`Send` items and state on the three single-threaded schedulers) and by
`tests/mutable.rs` (both locks). See `docs/decisions/0003-test-infrastructure.md`.

Single- versus multi-threaded is not a feature but a type: every observable has a
`Mode: ThreadMode` (`thread_mode::Local` or `thread_mode::Shared`). Shared state goes through the
mode — `M::Ptr<T>` (`Rc<RefCell<T>>` / `Arc<Mutex<T>>`), `M::Flag`, and `M::BoxedObserver` of
`ObserverMode` — never
`Rc`/`Arc` directly, and `Send` is required only where a value really crosses threads (the
scheduler impls, the `SendBoxed*` erasures). The test suite runs in the `Shared` mode;
when touching shared state or bounds, run `tests/local_mode.rs` too, which checks that the `Local`
mode still takes values that are not `Send`.

## Other commands

```bash
cargo fmt
cargo clippy --all-targets
cargo doc --open
cargo tarpaulin --out Html
```

## Layout

- `src/observable/`, `src/observer/`, `src/disposable/` — core traits. `ObservableExt` in
  `src/observable/mod.rs` is the fluent API; every operator gets a method there, in alphabetical
  order, except that each `into_*` conversion sits next to its `Send` twin.
- `src/operators/<category>/<name>.rs` — one operator per file, categories mirror reactivex.io.
- `src/subject/` — the subjects, the hot sources.
- `src/thread_mode/` — the bottom layer, depending on nothing else in the crate: `ThreadMode`,
  `Local`, `Shared`, `Joined` (the mode of an operator with several sources), the pointer, weak
  pointer and flag each mode picks, and the lock helpers (`thread_mode::mutable`). The boxed
  observer of a mode is the extension `ObserverMode` in `src/observer/boxed_observer.rs`.
- `src/scheduler/` — `Scheduler<TC, P>` runs a `Task` (a context plus a `fn` handler);
  `SchedulerExt` adds the closure conveniences. `task/` holds the task and how to run it:
  `mod.rs` the task and its `Stepper`, `drive.rs` the async driver (`drive`, `yield_now`), and one
  file per constructor with the state type it names (`once.rs`, `periodic.rs`, …); `runtime/` the
  implementations, one module per runtime and feature, with
  its `Shared` and `Local` scheduler side by side; `virtual_time.rs` the scheduler on a virtual
  clock that tests move forward (`VirtualTime`, always compiled).
- `src/utils/` — shared machinery: `serialized_delivery`, `serialized_multicast`,
  `subscribe_with_context`, `pending_events`. Public modules are for users writing their own
  operators; a module shared inside the crate only is `pub(crate) mod`.
- `tests/<name>.rs` — integration tests, one file per operator; shared helpers in
  `tests/tests_utils/`.

## Code conventions

- No `unsafe`. The crate-level `forbid` makes this a hard error.
- **Locks**: reach a lock (`RefCell` / `Mutex`, or the `M::Ptr` wrapping one) through
  `MutableHelper::with_mut` / `with_ref`, or through the one-shot helpers in `MutableExt`
  (`clone_value`, `replace_value`, `take_value`) — never `lock()` / `borrow_mut()`. The callback
  gets a `&mut T` / `&T`, so a guard cannot escape it; what it must not do is run anything that can
  take the same lock again, dropping an owned value included. Take the value out under the lock and
  act on it afterwards (`take_value().map(...)` for a lock around an `Option<_>`), or return an
  action from the callback and run it after `with_mut` returns. Release the context lock before
  calling `on_next`, and both the context and observer locks before `on_termination`. One
  "transaction" should take the lock once, not repeatedly. Debug builds panic when a thread takes a
  lock it already holds. See the module docs in `src/thread_mode/mutable.rs`. The tests call those
  methods directly too.
- **Subscription helpers**: prefer the existing helpers over hand-rolled state:
  `subscribe_with_context` for an operator with shared state — its context owns the source
  subscription and disposes it as soon as it stops, on whichever thread stops it (decision 0005);
  `subscribe_with_auto_dispose_on_termination` for the simple case. A scheduler task holds a clone
  of the context, like the observer given to the source: when the source lets go of the observer —
  a termination, or a drop that only means it sends nothing more (decision 0004) — the task finishes
  the work already accepted. That forms no cycle, since the context holds only the task's disposal,
  and a disposal still releases the downstream observer at once, through the handle the source
  drops as it is disposed.
- **Implementing an observable** takes two impls: `ObservableTypes` with the associated `Item`,
  `Error`, `Mode` and `Disposal` — which must not mention the observer — and `Observable<OR>`,
  whose bounds name the observer the source is subscribed with
  (`OE: Observable<MapObserver<OR, F>, Item = T0>`). That observer type is `pub`, its fields are
  not: the non-boxed `create`, `hook_on_subscription` and `hook_on_termination` hand the
  downstream observer to a user closure, and Rust rejects user code holding a value of a private
  type, so the observer and every type in its generic arguments must be public. What sits only in
  its fields or in the where-clauses of trait impls — the model, the context, scheduler task and
  mode aliases — stays private: privacy does not check those where-clauses. `Mode` is the source's, `Joined<A, B>` for several sources, or the scheduler's. Ask for
  `OR: IntoBoxedObserver<'a, T, E, M>` only where the observer is boxed (a subject, `create`, a
  hook), box it with `M::boxed(observer)`, and bound a generic mode there by `M: ObserverMode`.
  Inside the bounds of the impl being written, project qualified —
  `<OE as ObservableTypes>::Disposal`, `<OE as ObservableTypes>::Mode` — not `OE::Disposal` (the
  shorthand is a cycle error there). An observer that re-subscribes with itself (`retry`,
  `concat_all`) carries a `utils::resubscribe::Resubscribe` created where the operator is
  subscribed, instead of bounding its own `Observer` impl. See
  `docs/decisions/0002-observable-types-and-thread-mode.md`.
- **The `Disposal`** is one of three: one passed through — the source's or scheduler's
  (`OE::Disposal`), or that of the operator it is built on (`switch_map` takes `switch::Disposal`)
  — a `utils` helper's named disposal used as it is when its parameters are only the operator's
  own (`subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode, OE::Disposal>`), or a
  `Disposal` of the operator's own module. Anything else — a combination of combinators, or a type naming the
  operator's model or another of its private types — is named by `delegate_disposal!`, and the
  types inside stay private. `grep -rn "type Disposal = " src/operators` lists them all.
- Keep `pub` surface minimal; add `Clone`/`Send`/`Sync`/`'static` bounds only where actually needed.
- **Derives** on a generic type go through `#[derive(Educe)]`, not `#[derive(...)]` (which bounds
  every type parameter) and not hand-written impls. A field that cannot or should not be printed
  gets `#[educe(Debug(ignore))]`. Educe infers the bounds from the field types, and leaves out those
  it knows always hold (`PhantomData`, `Arc` / `Rc` for `Clone`, function pointers, a `Vec<T>`
  reduced to `T`), but it matches type names literally: through a type alias (`MarkerType`) or an
  associated type (`M::Ptr<_>`) it emits the whole field type as a bound. Where that bound would
  name a mode pointer or a private type, set it explicitly, `Clone(bound())` or
  `Clone(bound(S: Clone))`, so that the public impl does not list it. Write an impl by hand only
  when it is not structural, e.g. a `Default` that calls a constructor.
- **Time comes from the scheduler**, never from `Instant::now()`: an operator reads
  `SchedulerTypes::now()` at subscription or on an event, and a task handler uses the `now` its step
  is given (`Task::recursive`'s third argument, `schedule_recursively`'s second). An operator that
  measures time without scheduling anything (`throttle`, `timestamp`, `time_interval`) still takes a
  scheduler, for its clock. That is what lets `VirtualTime` drive every operator; CI fails on an
  `Instant::now` in `src/` outside `scheduler/mod.rs` and `scheduler/virtual_time.rs`. Read the
  clock outside a context lock when the value is needed anyway (`let deadline =
  self.scheduler.now() + span;` before `update`).
- Public items get doc comments; operators link to their reactivex.io page.
- `Termination<E>` (`Completed` / `Error`) is the single termination type — do not introduce
  parallel representations.
- **`Flow`**: `Observer::on_next` returns `Flow::Continue` / `Flow::Stop`. `Stop` is a guarantee —
  the observer accepts nothing more and must not be terminated, so the caller stops pushing and
  drops it; `Continue` is only a hint ("no stop seen here"), so a source must still honor its
  disposal. An operator that forwards values returns what its own downstream returned, so that the
  answer reaches the source; an operator that ends its own stream returns `Stop`. A context-based
  operator gets it from `SubscriptionContext::update_flow` / `send_next`; `send_termination` answers
  nothing, since a termination is the last event anyway. The `#[must_use]` is what catches a
  forgotten forward — `let _ =` only where ignoring it is deliberate. User callbacks
  (`subscribe_with_callback`) return `()` or a `Flow`, via `callback_observer::IntoFlow` (a
  diverging `|_| unreachable!()` needs `-> ()`).

## Test conventions

Integration tests (not `#[cfg(test)]` modules). Each file starts with `mod tests_utils;` and builds
sources with `test_channel()` plus a `Checker` observer, asserting on `checker.values()`,
`checker.state()` and the channel state after each event.

Pick the source by what the test needs from it, not by habit:

- One subscription, driven step by step by the test — the default: `test_channel()`. Its sender
  ends the channel with `on_termination`, or with `abandon()`, which drops the observer without a
  termination (`test_abandon_after_next`, decision 0004).
- One observable subscribed several times (several observers, `retry`, `repeat`, a connectable):
  `test_channels()`, one channel per subscription, addressed by its index.
- A source that must emit while it is being subscribed to, then keep going: a `test_channel()`
  with `.start_with([value])`. One that only emits and ends synchronously: `Empty`, `Throw`,
  `Just`.
- A disposal the test controls (`test_lifetime_*`, the `test_*_on_unsub` family, disposal
  ordering), borrowed `&mut` items or errors, or a source that is never subscribed and only needs
  to be `Clone` (`test_clone`): `Create::shared_boxed`.
- One send that must reach several subscriptions of the same source (`*_are_same`,
  `*_same_inner`), or a subject the operator API takes: `PublishSubject::shared()` and the other
  subjects.

Tests build in the `Shared` mode, the mode of every `TestScheduler`, through the library's own
constructors: `Create::shared_boxed`, `PublishSubject::shared()` and the other subjects'
`shared`, `unicast_subject::shared`; erase with `into_send_boxed` / `SendBoxedObserver`. A source
of another mode that must sit among erased `Shared` observables of one type gets `.into_shared()`.
Name the mode (`Shared`, from `rx_rust::thread_mode`) only where inference cannot reach it, and
write `_` in `let` annotations. Keep test state in plain `Arc<Mutex<_>>` / `AtomicBool`, reached
through `MutableHelper` / `MutableBoolHelper`, and bound closures with `Send`. Tests that need real
threads need no gate. A
test that needs the `Local` mode itself goes in `tests/local_mode.rs`.

What must not compile is a `compile_fail` doctest on the item, next to a twin that compiles and
differs in one place, since a `compile_fail` passes on any error; a `Local` type that must not be
`Send` also gets `assert_not_send!` in `tests/local_mode.rs`. No `trybuild` (decision 0003).

A time-based test runs on a virtual clock, not on real sleeps: `let time = VirtualTime::new();`,
`time.scheduler()` for the operator, `time.advance_by(d)` to move the clock, synchronously and
without `block_on`. Assert on exact boundaries (`DURATION_100_MS - DURATION_1_MS`, then
`DURATION_1_MS`), not with a margin. Only `test_async` stays on the real schedulers. A task that
keeps asking to sleep until an instant that has passed makes `advance_by` panic after 10 000 steps
(an off-by-one `<` where `<=` was meant), rather than hang. See `docs/testing.md`.

A test that checks which thread a callback runs on uses `ThreadCheckerScheduler`
(`tests/tests_utils/thread_checker_scheduler.rs`). It does not run a task by itself: `run_task`
only queues it, and the test calls `thread_1.run_until_stalled().await` (the handle it kept) to run
the queued work on that named thread and wait for it — never a `sleep`. The test, not the OS,
decides the interleaving, so assert one exact outcome per step, never "either this or that"; a
panic on that thread fails the test from `run_until_stalled`. Real concurrency between the source's
thread and the delivering one is the job of the `test_race_condition*` tests: a
`ThreadPoolScheduler`, many rounds, assertions on invariants only (order, exactly once, nothing
after a `Stop`), and a wait on a channel with a timeout rather than a sleep.

`docs/testing.md` holds the canonical checklist of case names (`test_completed`, `test_error`,
`test_unsubscribe`, `test_ref`, `test_async`, the hot-observable `test_*_on_next` family, the
lock-related `test_*_on_sub` family, the scheduler-related `test_*_after_next` family, …). When
adding an operator, follow the subset that applies to its shape and name the tests the same way — do
not invent new names for existing cases.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>)!: <subject>`,
the subject lowercase, imperative, without a trailing period. Types: `feat`, `fix`, `perf`,
`refactor`, `test`, `docs`, `build`, `ci`, `chore`. The scope is optional — an operator, `scheduler`,
`subject`, … — and `!` marks a breaking change to the public API (or the MSRV).

Example: `test(scheduler): check that a periodic task catches up after an overrun`

Do not commit or push unless asked.
