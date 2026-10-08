# 0003: The test infrastructure

Three decisions about the test suite: how it handles threads (adopted), what must not compile
(adopted, with `trybuild` rejected for now), and time (adopted: a virtual-time scheduler, with the
time taken from the scheduler).

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

Status: adopted. The time comes from the scheduler; a thread-local clock was designed first and
rejected (see History).

### Before

Every time-based test slept for real: sleep `DURATION_100_MS - DURATION_30_MS`, check that nothing
happened, then sleep `DURATION_30_MS * 2` and check that it did. The margins were there because a
real sleep may oversleep. The tests were slow, the margins hid off-by-a-few-ms bugs, and they still
flaked: the `ci` profile of `.config/nextest.toml` retries every test twice for that reason.

### Decision

- **The time comes from the scheduler.** `SchedulerTypes::now(&self) -> Instant` is a default
  method (the system clock), so a custom scheduler is not broken by it. Outside a task an operator
  reads `scheduler.now()` — at subscription, on an event. Inside a task the handler gets the time of
  the step as an argument: `TaskHandler` and `Stepper::step` take a `now: Instant`, and so do the
  steps of `Task::recursive` and the closure of `schedule_recursively`. A `TaskState::SleepUntil` is
  an instant on that clock. `Task::periodic` without an anchor anchors on its first step's `now`.
- **`drive` takes the scheduler** (`drive(task, delay, scheduler, sleep)`), steps with
  `scheduler.now()` and measures a `SleepUntil` against it. What is left to the implementer is that
  `sleep` runs on the same clock.
- **`throttle`, `timestamp` and `time_interval` take a scheduler**, for its clock only: they
  schedule nothing. A breaking change, made with the others of 2.0.
- **`scheduler::virtual_time`**, always compiled, no feature:
  - `VirtualTime` owns the clock and the queued tasks; the test holds it and calls `advance_by(d)`,
    `now()` and `pending_tasks()`. `advance_by(Duration::ZERO)` runs what is due now.
  - `VirtualTimeScheduler` is the handle the operators get, `Shared`, `Send + Sync`. It holds the
    `VirtualTime` weakly: a task often holds its own scheduler (an `interval` upstream of a
    `debounce`), and a strong handle would form a cycle through the queue. Dropping the
    `VirtualTime` drops the queued tasks.
  - `run_task` only queues, a task without delay too. `advance_by` runs, on the calling thread, each
    task due by the target in order of due time, then of queueing, after setting the clock to the
    instant it was due, so that each step of an `interval` advanced by ten periods sees its own
    time. A task queued meanwhile and due by the target runs in the same call.
  - A task that returns `Pending` waits for its waker, and runs at the next `advance_by`.
  - Panics: `advance_by` from inside a task or from two threads at once; a scheduler used after its
    `VirtualTime` is gone; and a task that asks 10 000 times in a row to sleep until an instant that
    has passed. That last one would otherwise hang: a real clock moves while such a task waits, the
    virtual one moves only between due tasks. It is what an off-by-one deadline check (`<` for
    `<=`) does, and a test then fails at once instead of hanging. A `Yield` has no such limit, since
    a stream task yields after every element.
- **The time-based tests run on it**, synchronously, without `block_on`, and assert on exact
  boundaries: nothing after `advance_by(DURATION_100_MS - DURATION_1_MS)`, the event after
  `advance_by(DURATION_1_MS)`. Changing the `<=` of `delay`, `debounce`, `timeout` or `throttle` to
  `<` fails 4 to 13 tests of the operator's file. `test_async` stays on the real schedulers, as the
  case about them. The scheduler's own tests are `tests/virtual_time_scheduler.rs`.
- **CI** fails on an `Instant::now` in `src/` outside `scheduler/mod.rs` (the default `now`) and
  `scheduler/virtual_time.rs` (the origin of a virtual clock).
- **The `ci` profile keeps its retries**: `test_async`, the `test_race_condition*` tests and
  `tests/scheduler.rs` still run on real time.

### Compared with other Rx libraries

- RxJava's `TestScheduler` (`advanceTimeBy`, `triggerActions`) and Rx.NET's (`AdvanceBy`, `Start`)
  are the same design: the clock is the scheduler's, every time-based operator takes one, and the
  test advances it synchronously. Rx.NET adds a recording observer (each event with its virtual
  time, asserted as one timeline), and RxJS marble tests (`'-a--b-|'`). Neither is adopted: the
  suite asserts step by step with `Checker`, and a virtual clock fits that as it is.
- RxJS's `run()` mode swaps the default time source globally, the thread-local design of the
  History below; Tokio's `time::pause` is per runtime and only for Tokio.

### History: a thread-local clock (designed, rejected)

The first design kept `Instant` out of the scheduler API: a `scheduler::now()` behind a
`test-scheduler` feature, reading a thread-local virtual clock registered by the virtual-time
scheduler and falling back to `Instant::now()`. The operators would only have swapped
`Instant::now()` for `now()`. Alternatives considered then were a `Clock` enum from a default method
(rejected as too much plumbing), an associated type `SchedulerTypes::Clock` (breaks every custom
scheduler: stable Rust has no associated type defaults), and a process-wide clock (the tests of a
binary run on parallel threads under `cargo test`).

It was rejected for what the thread-local cost elsewhere:

- The time depended on which thread asked, not on the pipeline's scheduler. A timestamp taken on a
  thread without the clock — a test pushing from another thread, an `observe_on(tokio)` — was a
  real `Instant`, a few milliseconds from the virtual ones: a silent error.
- Excluding that meant a `Local`-only, `!Send` virtual scheduler, which the `Shared` helpers
  (`test_channel()`, `SendBoxedObserver`) cannot hold: the tests would have needed `Local` twins of
  the helpers.
- Production and tests would have read the clock through different code, behind a feature.

Taking the time from the scheduler removes all three: the clock goes with the scheduler, so the
virtual one can be `Shared` and work with the existing helpers, with no feature. The plumbing that
was feared stayed small: the operators already hold their scheduler, and a handler gets `now` from
its step.
