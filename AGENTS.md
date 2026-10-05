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
- `RX_TEST_SCHEDULERS=tokio` (see below) runs a test on one scheduler instead of seven, which is
  enough while iterating on code that is not scheduler specific.

## Features

The tests need no `--features`: `Cargo.toml` has a dev-dependency on the crate itself that enables
every scheduler feature, so `cargo test` builds them all. The features only matter for the library
(`cargo check --lib --features …`, `cargo hack --each-feature --no-dev-deps check --lib`).

`block_on` in `tests/tests_utils/test_scheduler.rs` runs the test body once on every scheduler of
the crate, in this order: `tokio`, `tokio-local` (`TokioLocalScheduler` on a `LocalSet`),
`async-std`, `smol`, `smol-local`, `thread-pool`, `local-pool`. The body is an `Fn` that gets a
`TestScheduler`, called once per scheduler. A failing run prints
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
  its `Shared` and `Local` scheduler side by side.
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
  `subscribe_with_context` when the operator can only terminate from inside the source's own
  `on_termination`; `subscribe_with_context_owning_source` when it can terminate while the
  source is still active (notifier, scheduler task, another source);
  `subscribe_with_auto_dispose_on_termination` for the simple case. A scheduled task that must
  still deliver after the source has terminated holds a `PromotableWeakContext`, which the
  operator promotes on termination, rather than a plain weak context.
- **Implementing an observable** takes two impls: `ObservableTypes` with the associated `Item`,
  `Error`, `Mode` and disposal `D` — which must not mention the observer — and `Observable<OR>`,
  whose bounds name the observer the source is subscribed with
  (`OE: Observable<MapObserver<OR, F>, Item = T0>`). That observer type is `pub`, its fields are
  not: the non-boxed `create`, `hook_on_subscription` and `hook_on_termination` hand the
  downstream observer to a user closure, and Rust rejects user code holding a value of a private
  type, so the observer and every type in its generic arguments must be public. What sits only in
  its fields or in the where-clauses of trait impls — the model, the context, scheduler task and
  mode aliases — stays private: privacy does not check those where-clauses. `Mode` is the source's, `Joined<A, B>` for several sources, or the scheduler's. Ask for
  `OR: IntoBoxedObserver<'a, T, E, M>` only where the observer is boxed (a subject, `create`, a
  hook), box it with `M::boxed(observer)`, and bound a generic mode there by `M: ObserverMode`.
  Inside the bounds of the impl being written, project qualified — `<OE as ObservableTypes>::D`,
  `<OE as ObservableTypes>::Mode` — not `OE::D` (the shorthand is a cycle error there). An observer
  that re-subscribes with itself (`retry`, `concat_all`) carries a `utils::resubscribe::Resubscribe`
  created where the operator is subscribed, instead of bounding its own `Observer` impl. See
  `docs/decisions/0002-observable-types-and-thread-mode.md`.
- **The disposal `D`** is one of three: one passed through — the source's or scheduler's
  (`OE::D`), or that of the operator it is built on (`switch_map` takes `switch::Disposal`) — a
  `utils` helper's named disposal used as it is when its parameters are only the operator's own
  (`subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode, OE::D>`), or a `Disposal` of
  the operator's own module. Anything else — a combination of combinators, or a type naming the
  operator's model or another of its private types — is named by `delegate_disposal!`, and the
  types inside stay private. `grep -rn "type D = " src/operators` lists them all.
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

- One subscription, driven step by step by the test — the default: `test_channel()`.
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

`docs/testing.md` holds the canonical checklist of case names (`test_completed`, `test_error`,
`test_unsubscribe`, `test_ref`, `test_async`, the hot-observable `test_*_on_next` family, the
lock-related `test_*_on_sub` family, the scheduler-related `test_*_after_next` family, …). When
adding an operator, follow the subset that applies to its shape and name the tests the same way — do
not invent new names for existing cases.

## Commits

Prefix the subject with a category in brackets, matching existing history:
`[Feature]`, `[Improvement]`, `[BugFix]`, `[Refactoring]`, `[Tests]`, `[Miscellaneous]`.

Example: `[Tests] Add drop_probe and serialized_delivery tests.`

Do not commit or push unless asked.
