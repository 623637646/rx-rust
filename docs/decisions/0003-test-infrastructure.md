# 0003: The test infrastructure

Three decisions about the test suite: how it handles threads (adopted), what must not compile
(adopted, with `trybuild` rejected for now), and time (deferred).

## One test build, `Shared`, on every scheduler

Status: adopted.

### Before

The suite was two builds, picked by `#[cfg(feature = "local-pool-scheduler")]` all over
`tests/tests_utils/`:

- `cargo test-shared`: `TestMode = Shared`, and `block_on` ran each async test on Tokio, async-std,
  smol and the thread pool.
- `cargo test-local`: `TestMode = Local`, and `block_on` ran each async test on the local pool and
  on the `Local` schedulers of Tokio and smol.

It took two `block_on`s, two `TestRuntime`s, two cargo aliases and a CI matrix. A test that needed
real threads was gated off the `Local` build. A plain `cargo test` failed on a `compile_error!`
until it was given a scheduler feature.

### Decision

- **One `block_on`**, in `tests/tests_utils/test_scheduler.rs`, runs the body once on each of the
  six schedulers of the crate: `tokio`, `tokio-local`, `smol`, `smol-local`, `thread-pool`,
  `local-pool`. The body gets a `TestScheduler`, one enum with a variant per
  scheduler.
- **`TestScheduler` is `Shared` for every variant**, the single-threaded ones included. That is
  sound: declaring `Shared` only makes the operators downstream pick the thread-safe pointers.
- **The single-threaded schedulers are not `Send`** (each holds an `Rc` / `Weak` of its executor),
  and a `Shared` scheduler must be, since operators store it in observers that cross threads. So
  their variants hold no scheduler. They find it in a thread-local that `block_on` sets on the
  thread driving the executor. Using one from another thread panics, which is the `Local` contract
  anyway.
- **The tests enable every scheduler feature themselves**, through a dev-dependency of the crate
  on itself. `cargo test` needs no `--features`, and the tests contain no scheduler-feature `cfg`.
- **The tests use the library's own constructors.** With one build, `TestMode` named nothing that
  could change, so it went, together with the wrappers in `tests/tests_utils/modes.rs`. The tests
  call `Create::shared_boxed`, `PublishSubject::shared()` and the other `shared` constructors
  directly, erase with `into_send_boxed`, and name `Shared` only where inference cannot reach it.
  The tests' `AsTestMode` became `ObservableExt::into_shared` in the library, since a user erasing
  a `Local` source next to a `Shared` one meets the same need. The tests' own state is plain
  `Arc`, `Mutex`, `AtomicBool` and `Send`.

### Why not keep the `Local` mode in the main suite

The body of `block_on` is a closure, and a closure cannot be generic. Running one body against
schedulers of both modes therefore means one of these:

- **A generic test function per test.** Each `async fn body<S: TestScheduler>(s: S)` has to spell
  out every bound the library needs for an abstract `S::Mode`: the task type of each scheduler-based
  operator, and the observer type each source must accept. On top of that, "`Send` only in `Shared`"
  cannot be stated generically. That is a `where` clause per test, growing with every operator in
  the chain, over ~500 tests.
- **A macro that pastes the body once per mode.** Each copy is concrete, so inference works. But
  every helper in `tests_utils` and every test's own state has to follow a mode chosen by the
  macro, all call sites become macro calls, and the suite compiles twice.

Neither was worth it. The operators are generic over the mode, and the modes differ only in the
pointer, weak pointer and flag they pick. Debug builds check lock re-entry the same way for both.
The synchronous sources (`just`, `from_iter`, `range`, …) are `Local`, so the main suite still runs
`Local` pipelines whenever no `Shared` source or scheduler joins them.

### What covers the `Local` mode

What the main suite no longer checks is the `Local` mode's promise: nothing it is given needs to be
`Send`. A `Send` bound slipped into an operator would compile in a `Shared`-only suite. Two files
cover that instead:

- `tests/local_mode.rs`: pipelines whose items, callbacks and state hold `Rc`, built from the
  `Local` sources and subjects, and run on the three single-threaded schedulers directly (one test
  per scheduler, through a small macro). There is one representative per kind of operator:
  synchronous, time-based, higher-order, multi-source, the four subjects, `create` and erasure.
- `tests/mutable.rs`: every lock test runs on both locks, the `RefCell` and the `Mutex`.

A change to the bounds of an operator, or to the shared state of one, should run
`tests/local_mode.rs` as well as the operator's own file.

## What must not compile

Status: adopted. `trybuild` rejected for now.

### The problem

The design rejects at compile time what would race or dangle: a `Shared` subject or scheduler takes
no `Rc`, a `Local` pipeline cannot cross threads, an erased observable cannot outlive what it
borrows, `ThreadMode` is sealed. A loosened bound keeps every ordinary test green, so each of these
needs a check that fails when the code starts to compile.

A `compile_fail` doctest is the obvious tool, and it has one flaw: it passes on *any* compile error.
A typo, an import gone stale or a renamed API keeps it green while it no longer checks anything.
`compile_fail,E0277` does not help: rustdoc checks the error code on nightly only, and most of
these are E0277 anyway.

### Decision

- **A `compile_fail` doctest sits on the item that makes the guarantee, next to a twin that
  compiles and differs from it in one place** (`shared()` / `local()`, `TokioScheduler` /
  `TokioLocalScheduler`, `Rc` / `Arc`, `into_send_boxed` / `into_boxed`). The twin proves the rest
  of the example is current, so the one difference is the reason it fails. A twin that cannot run
  outside a runtime is `no_run`: it is still compiled. The examples also tell users what is
  refused and what to write instead, which is why they stay in the docs.
- **The only example without a twin is the sealing of `ThreadMode`**, since no mode can be added
  outside the crate. It implements every item of the trait, so that the sealing is its only error,
  and a comment above it says it must follow the trait.
- **"Not `Send`" is also asserted in `tests/local_mode.rs`**, as constant items checked when the
  file compiles: `assert_not_send!` for what the `Local` mode picks (the mode, its pointer, its
  boxed observer, `Emitter<_, Local>`, a local subject, the single-threaded schedulers) and
  `assert_send!` for each `Shared` twin. `assert_not_send!` is the ambiguity trick of the
  `static_assertions` crate, written out in a few lines rather than added as a dependency. It
  cannot pass for a wrong reason: a typo fails the build.

### Why not `trybuild`

`trybuild` compiles each case of `tests/ui/` on its own and compares the output with a `.stderr`
snapshot, so it pins *why* a case fails, which a doctest cannot. Against that:

- The snapshots follow the compiler's wording, which changes between releases: they would have to
  be checked on one pinned toolchain, not on the MSRV and stable both, and regenerated
  (`TRYBUILD=overwrite`) on each upgrade.
- It is one more dev-dependency and a slow target of its own, each case a separate crate.
- With the twins above, the only case left without a guard is the sealing, one example.

**The `test_lifetime_*` tests are no reason to adopt it either.** About two hundred of them
(`test_lifetime_sub`, `_or`, `_or_sub`, …, see `docs/testing.md`) each keep the order that must not
compile as a comment under `// Error`, unchecked. `trybuild` could check them, but what they would
pin is the borrow checker's, not the library's: the crate forbids `unsafe`, so the reversed order
can only start to compile when the subscription or observer no longer borrows the value, or no
longer has drop glue that may use it, which is sound either way, and the second would be caught by
the disposal tests. The library's own promise is the other half, that an operator does not demand
`'static`, and that half already compiles in the suite. Two hundred borrow-checker snapshots would
be the costliest part of a `trybuild` target and the least useful.

Reconsider when a guarantee with no possible twin is added, or when the wording of an error becomes
part of the design (a `#[diagnostic::on_unimplemented]` message, or `Emitter`'s promise that the
error names `Emitter<_, Local>`): only a snapshot pins that.

## A virtual-time scheduler

Status: deferred — designed but not implemented. This records where the design got to, what is
hard about it and what it would cost, so that a later attempt starts from here. The name
`TestScheduler` belongs to the test helper above, so this one needs another, such as
`VirtualTimeScheduler`.

### Motivation

Every time-based test sleeps for real. The usual shape is to sleep
`DURATION_100_MS - DURATION_30_MS`, check that nothing happened yet, then sleep `DURATION_30_MS * 2`
and check that it did. The margins are there because a real sleep may oversleep. The tests are slow,
the margins hide off-by-a-few-ms bugs, and they still flake: the `ci` profile of
`.config/nextest.toml` retries every test twice for that reason.

A virtual-time scheduler (RxJava's `TestScheduler`, RxJS's `VirtualTimeScheduler`) would make
`sleep(d)` exact and free:

- `run_task(task, delay)` only queues the task, due at `now + delay`.
- `advance_by(d)` / `advance_to(at)` move a virtual clock to each due time in turn and run the tasks
  due there, synchronously, on the calling thread.
- A test then asserts on exact boundaries — nothing after `advance_by(99ms)`, the value after one
  more `advance_by(1ms)` — and takes no real time.

### The scheduler itself (the easy part)

It drives tasks with `Task::split` / `Stepper::step`, not with `drive`.

- **Queue:** a `BinaryHeap` of `(due, sequence, id)` plus a map `id -> slot`. Entries for cancelled
  tasks are skipped when they come up.
- **Loop:** under the lock, pop the earliest entry with `due <= target`, set the clock to `due` and
  take the task out. Release the lock, then step it. Take the lock again and act on the answer:
  - `Finished`: drop the task, outside the lock.
  - `Yield`, or `SleepUntil(at)` with `at <= now`: requeue at `now` with a new sequence number, so
    the other tasks due now get their turn.
  - `SleepUntil(at)`: requeue at `at`.
  - `Pending`: park the task. Its waker, a `std::task::Wake`, pushes the id onto a woken list,
    which the next loop requeues at `now`.

  After the loop, set the clock to `target`.
- **Disposal:** holds a weak handle and the id. Disposing takes a queued or parked task out and
  drops it outside the lock; a task that is running is marked cancelled, so it is not requeued.
- **Edge cases:** `advance` called from inside a task panics; `advance_to` an instant in the past
  panics; a task that yields forever hangs `advance`, as it would spin on a real executor.

### Difficulty 1: the operators read the clock themselves

A scheduler only decides when a task runs. The operators compute and compare their deadlines with
`Instant::now()`, so virtual sleeps alone break them. The calls are in these files:

- Deadlines and anchors, at subscription or in `on_next`: `utility/delay.rs`,
  `filtering/debounce.rs`, `utility/timeout.rs`, `creating/interval.rs`,
  `transforming/buffer_with_time.rs`, `transforming/buffer_with_time_or_count.rs`.
- Inside the `fn` task handlers, which cannot reach the scheduler: `delay`, `debounce`, `timeout`.
- Timestamps, with no scheduler parameter at all: `filtering/throttle.rs`, `utility/timestamp.rs`,
  `utility/time_interval.rs`.
- In `scheduler/`: the anchor of `SchedulerExt::schedule_periodically`, and `Task::periodic`'s
  fallback when no anchor is given.

The alternatives considered:

1. **A `Clock` value, from a default method `SchedulerTypes::clock()`.** `Clock` would be an enum,
   `System | Virtual(Arc<…>)`, so the change is not breaking. It is explicit and correct across
   threads. But every task context that compares times has to carry the clock, which changes the
   task type aliases (`DelayTask`, …). And `throttle` / `timestamp` / `time_interval` need new APIs,
   such as `throttle_with_clock(span, clock)`. Rejected as too much plumbing.
2. **An associated type `SchedulerTypes::Clock`.** It breaks every custom scheduler, since stable
   Rust has no defaults for associated types. Rejected.
3. **A process-wide virtual clock.** `cargo test` runs the tests of a binary on parallel threads,
   which would advance each other's clock; it is safe only under nextest's process per test.
   Rejected.
4. **Chosen: `rx_rust::scheduler::now()`, behind a `test-scheduler` feature.**
   - It reads a thread-local virtual clock, registered by the virtual-time scheduler the thread
     created, and falls back to `Instant::now()` when there is none.
   - Without the feature it compiles to `Instant::now()`, so production pays nothing.
   - The operators only swap `Instant::now()` for `now()`; no signature or type changes.
   - `throttle` / `timestamp` / `time_interval` get virtual time for free.
   - The thread-local holds a `Weak`, so the registration ends with the last clone, wherever it is
     dropped. A second live virtual-time scheduler on the same thread panics.
   - `drive` must keep `Instant::now()`: it sleeps in real time, for the real schedulers.

### Difficulty 2: timestamps taken on two threads

With a thread-local clock, a value that takes a timestamp on one thread and another on a different
thread mixes virtual and real `Instant`s. That happens when a test pushes events from another
thread, or when the virtual-time scheduler is combined with a real one (`observe_on(tokio)`). The
error is silent: the virtual origin starts at the real `Instant::now()`, so the two clocks differ by
only a few milliseconds at first.

Considered:
- Carrying the clock with the object, which is alternative 1 again.
- Making the mix loud at run time: put the virtual origin a day ahead, and panic when a task is run
  or the clock is advanced from another thread.

**Conclusion: the virtual-time scheduler has only the `Local` mode.**
- It holds `Rc<RefCell<_>>` and `Rc<Cell<Instant>>`, so it is `!Send`.
- Every observer, context and disposal that holds it is `!Send` too, so moving such a pipeline to
  another thread does not compile.
- The cross-thread case is then excluded at compile time, and the implementation is simpler: one
  type, no `+ Send` task box, no atomics.
- Tests that are about a real scheduler (`test_async`, `tests/scheduler.rs`, `observe_on`,
  `subscribe_on`, `from_future`, `from_stream`, …) keep running on the real schedulers.

A `Shared` variant was the earlier plan. Its clock has to be `Send + Sync` (`Arc` plus an
`AtomicU64` offset), because a `Shared` subject boxes the observer holding the scheduler into a
`SendBoxedObserver`. Even so, it never reads the clock from another thread meaningfully.

### Difficulty 3: the test helpers are `Shared`

`test_channel()` and the other helpers in `tests/tests_utils/` build in the `Shared` mode, which is
the whole suite's (see above), and box the downstream observer into a `SendBoxedObserver`. A
`delay` observer holding a `!Send` virtual-time scheduler then does not compile. The virtual-time
tests therefore need `Local` helpers, along the lines of `tests/local_mode.rs`: make `test_channel`
generic over the mode, or add `Local` twins of the helpers they use. (A second option, gating the
virtual-time tests on a `Local` build, went away with the one-build suite.)

### Cost

- **Library:** the new scheduler, `scheduler::now()`, and swapping `Instant::now()` for it in nine
  operator files plus `schedule_periodically` and `Task::periodic`. Document the limits: one
  thread; do not mix with a real scheduler.
- **Feature wiring:**
  - `test-scheduler = []` in `Cargo.toml`, and in the features of the crate's dev-dependency on
    itself, so that `cargo test` keeps needing no `--features`.
  - CI: the `cargo hack --each-feature --no-dev-deps check --lib` step covers the new feature by
    itself and proves the library builds without it.
- **Tests:**
  - A new test file covering exact boundaries, ordering, disposal, periodic rate, `Pending` and
    waking, the panics, and the fallback to the system clock.
  - Migrating `delay`, `debounce`, `timeout`, `interval`, `timer`, `buffer_with_time`,
    `buffer_with_time_or_count`, `sample` (the `Interval` sampler cases), `throttle`, `timestamp`
    and `time_interval`, about 9.4k lines.
  - Each test drops `block_on`, uses `advance_by` with exact boundaries, and keeps its name and
    other assertions. `test_async` stays on the real runtimes.
  - The `Local` helpers of Difficulty 3.
- **Docs:** `docs/testing.md` (timing cases use the virtual-time scheduler), `AGENTS.md`, and this
  record updated to adopted.
