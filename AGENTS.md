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
  nearly every operator and subject. Changing a crate-private module such as
  `src/utils/positional/slots.rs` → run its unit tests, `cargo test --lib utils::positional::slots`,
  plus the integration tests of the operators that use it.
- Do not re-run a test that already passed unless the code under it changed.
- `cargo nextest run` also takes the `--test` filters above, and fails tests that leak; prefer it
  over `cargo test` when a test may hang.
- `RX_TEST_SCHEDULERS=tokio` (see below) runs a test on one scheduler instead of six, which is
  enough while iterating on code that is not scheduler specific.

## Features

The tests need no `--features`: `Cargo.toml` has a dev-dependency on the crate itself that enables
every scheduler feature, so `cargo test` builds them all. The features only matter for the library
(`cargo check --lib --features …`; `ci/check.sh lint dev` lints every combination of them).

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
mode — `M::Ptr<T>` (`Rc<RefCell<T>>` / `Arc<Mutex<T>>`), `M::Flag` (a `RaiseOnlyFlag`: once
raised it stays raised; a bool that goes back is a field under the lock), and `M::BoxedObserver` of
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

CI is `ci/check.sh`: each job of `.github/workflows/ci.yml` runs one of its subcommands (`static`,
`lint dev|release`, `doc`, `test dev|release`, `msrv`), so a local run of one checks exactly what
the job does, warnings denied. `ci/check.sh all` runs them all, the full test suite twice: before
a release, not after a local change. CI builds every configuration the code compiles differently
in — each feature combination, both profiles, the MSRV — and `ci/cfg_coverage.py` (in `static`)
fails on a `cfg` key that `ci/cfg-dimensions.toml` does not list: a new key, such as a
`target_os`, is a configuration CI does not build yet, so extend the matrix before listing it.
`.github/workflows/scheduled.yml` runs the beta toolchain and macOS weekly, and opens an issue
when they fail.

## Layout

- `src/observable/`, `src/observer/`, `src/disposable/` — core traits. `ObservableExt` in
  `src/observable/mod.rs` is the fluent API; every operator gets a method there, in alphabetical
  order, except that each `into_*` conversion sits next to its `Send` twin.
  `observable::sources` is the crate-private machinery for source sets (decision 0012):
  `SourceSet` and `SameItemSources` cover tuples and the `IterSources<I>` adapter. `SourceSet`
  holds `Item`, `Error` and `Mode` — `Item` is a `Combination`, a type selection that asks
  nothing of the items (no `Clone`, no `Send`) — and selects, hidden, the
  resources operators keep for the sources: `Cursor`, `CollectedCursor` and `SubscriptionSlots`, the
  one store of subscriptions — types only; the operations returning them are crate-private. Typed positions keep
  tuple access static and list access dynamic; storage capabilities let both forms share operator
  state machines. The source-independent structure lives in `utils::positional`. In
  `combination`, the public, sealed `Combination` — a tuple of items or a `Vec` — only selects
  types: `Latest` and `Queues` storage, and `After<T>`, the output after a primary item; its
  operations are in the crate-private `LatestOps`, `QueueOps`, `PrependOps` and `CloneLatest`
  (cloning only). `CombineLatest`, `WithLatestFrom` and `ForkJoin` keep the raw `Latest`
  storage, replacing a position through `AtPosition`: the first two clone a combination out with
  `CloneLatest::clone_all`, which answers `None`, cloning nothing, until every position has a
  value; `ForkJoin`, which takes its one combination as its stream ends, moves it out with
  `LatestOps::take_all`. An operator projects
  what it selects from a source set in place (`<S::Item as Combination>::Queues`), with no
  public alias for it. A struct generic over
  the combination itself (`Model<T>`) bounds it by `Combination`; operators require the `*Ops`
  traits where they operate on it, and `CloneLatest` only when subscribing needs a clone.
  Completion and termination policy stay in each operator.
  Zip shares one model and observer across tuple and iterator sources; its model owns the raw
  queues, empty-queue count and completed-source combination budget. `QueueOps::pop_all`
  removes one complete combination and counts the queues left empty in the same pass.
  Subscribing the sources has two levels, and an operator names only the higher one that fits
  (decision 0012, "Two levels"). The lower, `sources::subscribe`: every
  operator subscribes its sources through a cursor and a `SourceSubscriber`, whose steps
  `SubscribeEach` (`subscribe_each`, resumable) runs in order for each
  source — `still_running`, `observer` (`None` skips the source: `When`'s), the subscription,
  `store` — and keeps each subscription in the slot of its position. `SubscribeEach` is
  implemented directly by each cursor, a tuple of `Option`s (per arity) and an `Enumerate`, both
  running the steps of one source in the private `subscribe_one`. `Merge`, amb and `When` write
  a subscriber. On top,
  `sources::batch`: crate-private `SubscribeBatch` is the shared
  case, a batch, for the collecting operators (`CombineLatest`, `Zip`, `ForkJoin`,
  `WithLatestFrom`): one impl, over `CollectedCursor`, whose subscriber (`batch::Batch`)
  subscribes each source with the observer `ObserverFamily::at` gives for its position while
  `ObserverFamily::is_active` holds, and returns the guarded slots. A subscriber has one impl over
  `K: Position`. The batch's cursor is collected, so advancing it runs no user code — in its
  type: `into_collected_cursor()` returns a `CollectedCursor<C>`, which only the `sources`
  backends make, and a cursor from `into_cursor()` has no `subscribe_batch`. The `Enumerate`
  impl of `SubscribeEach` asks the subscriber's `still_running`, a required method, after each
  `Iterator::next`, user code, before subscribing the source it returned; the tuple impl takes its
  sources directly and never asks. The crate-private `SourceOps` holds the operations needing no observer:
  `into_collected_cursor()` collects every source and returns a cursor over them with their
  position count (decision 0012), and `into_cursor()` prepares the cursor of sources not collected. The collected
  cursor is subscribed by `SubscribeBatch`; the operator creates its storage for the count
  (`LatestOps::empty(n)`, `QueueOps::empty(n)`); Zip's count of empty queues starts at that
  count.
  `ReservationSlots<K>` and its generic `TupleSlots` and `ListSlots` implementations live in
  `utils::positional::slots`: values need no disposal trait, and those left are dropped in reverse
  position order. The source backends select these stores with `DisposeOnDrop` values. What goes
  from a tuple's last element to the first reverses the tuple with `IntoReversed`, from
  `utils::positional::tuple`, and drops the result, whose drop glue keeps that order during
  unwinding: `TupleSlots`, the tuple `Disposable` (each disposal wrapped in a `DisposeOnDrop`
  first) and `When`'s released inputs.
  The single-position `ReservationSlot` is in the same module, with three states: `Reserved`,
  `Filled` and `Released`, which is terminal and hands back whatever is filled afterwards.
  `When` needs only the cursor; `Merge` and amb need both the cursor and slots. `Merge` is one
  state machine, the `Driver` of the crate-private `combining/internal/merge.rs`, with or without
  a limit: with none there is always room, so the
  cursor never waits in the model, whose type then names `Infallible` in its place, and neither
  the observers nor the disposal name the sources. Both forms release the subscription of a
  source as it completes. `ConcatEager` is that driver over its sources made `IndexedSource`s
  (`IndexSources`, in `combining/internal/reorder.rs`), which send each value with their
  position, and `ReorderObserver` as the merge's observer: the driver knows nothing of positions.
  None of them requires batch resources. The three public strategy/resource traits are hidden;
  observer bounds stay on subscription capabilities. All public collection traits, aliases and
  adapters are re-exported by `observable`. Amb's winner swaps the slots for fresh ones and moves
  its own subscription across at its typed position.
  The tuple `Disposable` implementations live in the private `disposable::tuple_disposal` module.
  Every tuple impl, these and `When`'s included, is generated over the arities of
  `utils::positional::tuple::for_each_arity!`. A cursor — a tuple of `Option`s or an
  enumerated iterator — hands each source, with its position, to the subscriber and retains
  the remaining sources when it breaks. A subscriber that cannot break, its break
  type `Infallible`, is handed every source by one `subscribe_each` (`When`'s). A subscriber is
  borrowed `&mut`, so what it gathers is a plain field (the batch's slots, amb's `subscribed`),
  with no `RefCell` or `Cell`; one that is a shared handle is implemented for `&Handle`
  (`Merge`'s `&Driver`). The positions,
  `TupleIndex` at compile time and `usize` at run time (both a `Position`), and `AtPosition`, the
  typed access to an element of a tuple or a `Vec` at one, live in `utils::positional::position`.
- `src/operators/<category>/<name>.rs` — one operator per file, categories mirror reactivex.io.
  Every file directly in a category is a public operator; what is not — the crate-private
  machinery several operators share — lives in `<category>/internal/`, without a `_core` suffix:
  `combining/internal/` (`flatten`, `merge`, `join`, `reorder`), `filtering/internal/`
  (`deadline`, the pending value and timer of `debounce` and `audit`, which differ only in whether
  an item moves the deadline: a change to it runs `--test debounce --test audit`) and
  `transforming/internal/` (`count`, `time_shift`, `time_or_count`, `boundary`, `group`,
  `bundle`, `window`, `overlap`, `timetable`). The twelve buffer and window operators are four state machines, one per way their
  groups open and close — `count`, `time_shift`, `time_or_count`, `boundary` — each generic over
  the `Groups` of `group.rs`: the `Bundles` of `bundle.rs`, kept under the machine's lock, or the
  `Windows` of `window.rs`, every change an `Action` for its `DelegateObserver` outside it
  (decision 0014). `group.rs` holds only what the machines share; each kind of group is a module
  of its own.
  `buffer_with_count`, `window_with_count`, `buffer_with_time` and `window_with_time` are the
  `*_and_skip` / `*_and_shift` operators with the step equal to the length and `no_overlap` as the
  duplicate, so that they do not ask for `Clone`. A change to that machinery runs its unit tests,
  `cargo test --lib transforming::internal` and the integration tests of the twelve operators.
  The operators that flatten an observable of observables — `merge_all`, `concat_all`,
  `switch`, `exhaust_all` and their `*_map` forms (`flat_map` for `merge_all`) — are one state
  machine, the crate-private `combining/internal/flatten.rs`, set apart only by the store of the
  inner observables that find no place (decision 0013); `concat_eager_all` and
  `concat_eager_map` are that machine with an observer that puts the values back in order. A
  change to it runs its unit tests, `cargo test --lib combining::internal::flatten`, and the
  integration tests of those ten operators.
  The operators that merge a tuple or a list — `merge`, `concat_eager`, and `concat` over a list,
  a `Merge` of one at a time — are the `Driver` of `combining/internal/merge.rs` (decision
  0012); a change to it runs `--test merge --test concat --test concat_eager`. The observer that
  puts values back in order, with the observer that sends each source's position and the
  subscription of an observable of observables over `flatten` (`subscribe_eager`), is
  `combining/internal/reorder.rs`, shared by `concat_eager_all`, `concat_eager_map` and
  `concat_eager`; a change to it runs
  `--test concat_eager --test concat_eager_all --test concat_eager_map`.
  A limit is a method on the operator, not a constructor: `merge_all().max_concurrency(n)`,
  `Merge::new(sources).max_concurrency(n)`.
- `src/subject/` — the subjects, the hot sources.
- `src/thread_mode/` — the bottom layer, depending on nothing else in the crate: `ThreadMode`,
  `Local`, `Shared`, `Joined` (the mode of an operator with several sources), the pointer, weak
  pointer and flag each mode picks, and the lock helpers (`thread_mode::mutable`). The boxed
  observer of a mode is the extension `ObserverMode` in `src/observer/boxed_observer.rs`.
- `src/scheduler/` — `Scheduler<TC, P>` runs a `Task` (a context plus a `fn` handler); there
  are no closure conveniences, except the tests' `tests/tests_utils/scheduler_ext.rs`. `task/`
  holds the task: `mod.rs` the task, what
  its handler answers (`Next`) and its `Stepper`, and one file per constructor with the state type
  it names (`once.rs`, `periodic.rs`, …); `drive.rs` the async driver (`drive`, `yield_now`), which
  runs a task through its `Stepper` on an executor; `runtime/` the
  implementations, one module per runtime and feature, built on `drive`, with
  its `Shared` and `Local` scheduler side by side; `virtual_time.rs` the scheduler on a virtual
  clock that tests move forward (`VirtualTime`, always compiled).
- `src/utils/` — shared machinery. Public modules are for users writing their own operators:
  `subscribe_with_context`, `subscribe_with_auto_dispose_on_termination`.
  A module shared inside the crate only is `pub(crate) mod`: `serialized_delivery`,
  `serialized_multicast`, `flagged_lock`, `subscribe_fn`, `pending_events`,
  `reservation_map`, `id_generator`, `on_panic`, `lazy_subscription`, `positional`,
  `emit_then_terminate` (the values an operator sends as its stream completes — an aggregate's
  result, a default, the last items kept — then the termination, unless one of them stopped). A
  module stays public only when a public signature names its items or a user writing an operator
  needs them; otherwise it is `pub(crate)`, and so is everything in it — the crate's
  `#![warn(unreachable_pub)]` is what catches a `pub` left behind in one, and CI denies warnings.
  The few items of such a module that a public signature does name stay `pub` and are re-exported
  by the public module naming them (`serialized_delivery::UpdateOutcome` from
  `subscribe_with_context`).
- `tests/<name>.rs` — integration tests, one file per operator; shared helpers in
  `tests/tests_utils/`. Crate-private modules carry their own unit tests (see below).

## Code conventions

- No `unsafe`. The crate-level `forbid` makes this a hard error.
- **Debug-only code**: `cfg(debug_assertions)` gates whole items — a check, the helpers only it
  uses, their imports (the `thread_mode::mutable::reentrancy` module,
  `is_auto_dispose_on_termination_observer`) — and never selects between two implementations:
  no `cfg(not(debug_assertions))` twin, no function whose body or return type differs between the
  builds. A release build then compiles a subset of the debug build, not code of its own. CI lints
  and tests both profiles anyway.
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
  methods directly too. `Clone`, `PartialEq`, `Clock::now` and a connectable's subject factory
  are exempt: they may run under a lock, and do not get moved out of one (decision 0008).
- **Subscription helpers**: prefer the existing helpers over hand-rolled state:
  `subscribe_with_context` for an operator with shared state — its context owns the source
  subscription and disposes it as soon as it stops, on whichever thread stops it (decision 0005);
  `subscribe_with_auto_dispose_on_termination` for the simple case. A scheduler task holds a clone
  of the context, like the observer given to the source: when the source lets go of the observer —
  a termination, or a drop that only means it sends nothing more (decision 0004) — the task finishes
  the work already accepted. That forms no cycle, since the context holds only the task's disposal,
  and a disposal still releases the downstream observer at once, through the handle the source
  drops as it is disposed. An operator whose scheduler delivers what its source sends (`delay`,
  `debounce`, `observe_on`) starts one `Task::wakeable` when it is subscribed, and gives the
  source's observer the `TaskWaker`, never the scheduler: the source may send from a thread a
  `Local` scheduler cannot reach. The observer changes the model, then wakes the task.
  `observe_on` goes further and is not built on the context: its task owns the downstream
  observer, and the source's observer holds only an inbox, so that observer need not be `Send`
  with a `Local` scheduler; a disposal releases it when the scheduler drops the task, not at once
  (decision 0006). An operator that subscribes something *after* it has handed out its own
  subscription keeps a `ReplaceableDisposal` instead (`retry`, `catch`, `concat`), and does that
  work — the user's callback included — inside `replace_with`'s builder, which does not run once
  the slot is disposed: an event that arrives after a disposal is then dropped with the observer,
  rather than running the callback and delivering to an observer that is already released.
- **Implementing an observable** takes two impls: `ObservableTypes` with the associated `Item`,
  `Error`, `Mode` and `Disposal` — which must not mention the observer, and whose where-clause
  carries only what those four types need — and `Observable<OR>`,
  whose bounds name the observer the source is subscribed with
  (`OE: Observable<MapObserver<OR, F>, Item = T0>`). That observer type is `pub` but
  `#[doc(hidden)]`, and its fields are not public: the non-boxed `create`,
  `hook_on_subscription` and `hook_on_termination` hand the downstream observer to a user closure,
  and Rust rejects user code holding a value of a private type, so the observer and every type in
  its generic arguments must be public — yet no user ever writes its name, so it stays out of the
  documentation and needs no doc comment (the crate denies `missing_docs`, which is what lists the
  ones that lack both). What sits only in
  its fields or in the where-clauses of trait impls — the model, the context, scheduler task and
  mode aliases — stays private: privacy does not check those where-clauses. `Mode` is the source's, `JoinedModeOf<A, B>` for two sources, `<S as SourceSet>::Mode` for a tuple or a list of them (decision 0012), `JoinedSchedulerModeOf<OE, S>` for a source and a scheduler, or the scheduler's alone (`timer`, `observe_on`). Ask for
  `OR: IntoBoxedObserver<'a, T, E, M>` only where the observer is boxed (a subject, `create`, a
  hook), box it with `M::boxed(observer)`, and bound a generic mode there by `M: ObserverMode`.
  Inside the bounds of the impl being written, project qualified —
  `<OE as ObservableTypes>::Disposal`, `<OE as ObservableTypes>::Mode` — not `OE::Disposal` (the
  shorthand is a cycle error there). An observer that re-subscribes with itself (`retry`,
  `repeat_source`) carries a `utils::subscribe_fn::SubscribeFn` created where the operator is
  subscribed, instead of bounding its own `Observer` impl; the inner observers of the flattening
  operators carry the same capture as a plain function pointer, typed by what they queue
  (`Infallible` for a store that queues nothing). See
  `docs/decisions/0002-observable-types-and-thread-mode.md`.
- **The `Disposal`** is one of three: one passed through — the source's or scheduler's
  (`OE::Disposal`), or that of the operator it is built on (`switch_map` takes `switch::Disposal`)
  — a `utils` helper's named disposal used as it is when its parameters are only the operator's
  own (`subscribe_with_auto_dispose_on_termination::DisposalOf<OE>`), or a
  `Disposal` of the operator's own module. Anything else — a combination of combinators, or a type naming the
  operator's model or another of its private types — is named by `delegate_disposal!`, and the
  types inside stay private. `grep -rn "type Disposal = " src/operators` lists them all.
- **Type aliases for repeated shapes**: a helper names the types an operator built on it spells
  out, from the operator's source and scheduler types: `XxxOf<OE, …>` in the helper's module
  (`subscribe_with_auto_dispose_on_termination::{ObserverOf, DisposalOf}`), `JoinedModeOf<A, B>` and
  `ChainDisposalOf<A, B>` in `observable` for an operator with two sources, `JoinedSchedulerModeOf<OE, S>`
  and `ChainSchedulerDisposalOf<S, OE>` (the task's disposal, then the source's) for one with a
  source and a scheduler, `BoxedObserverOf<'a, OE>` for the boxed observer of a source's mode, `BoxedObservableOf` /
  `CloneableBoxedObservableOf` for what `into_boxed` / `into_cloneable_boxed` erase a source into. All
  of them are `pub`, so that an operator written outside the crate uses the same shorthand. Use
  them rather than spelling the projections out. Such an alias is written with qualified projections
  (`<OE as ObservableTypes>::Mode`), so it can appear in the bounds of `OE` itself, and it can
  name only public types: a private model still goes through `delegate_disposal!`.
- **An alias a public signature names is `pub`**: a return type, an associated type of a public
  impl (`type Mode =`, `type Disposal =`), the parameter or where-clause of a public fn. Rustdoc
  expands an alias in place, so a crate-private one leaves the docs without its name or its doc
  comment, and the privacy lints do not see through aliases, so nothing else catches it. Only the
  where-clauses of trait impls, which privacy does not check either, may name a crate-private one
  (an operator's own task alias, `DelayTask` in the bounds of `delay`). An alias in a private
  module is re-exported where its module's public items are (`scheduler::TaskHandler`).
- Keep `pub` surface minimal; add `Clone`/`Send`/`Sync`/`'static` bounds only where actually needed.
  On a constructor (`new`, a fluent method), "needed" means one of two things. The first is type
  inference: a closure's signature (`F: FnMut(T0) -> T`), a type parameter that appears in no
  argument and must be tied to the source (`OE: ObservableTypes<Item = T0, …>` for `Map`'s `T0`,
  `Debug`'s `T, E`), or two sources whose `Item` / `Error` must unify, so that one with a free
  parameter (a subject, `with_item_type`) is inferred. The second is the body or the struct's
  fields (`S::Mode::ptr`). A bound that only repeats what the `ObservableTypes` / `Observable` impl
  checks anyway (`Repeat`'s `T: Clone`, `S: Subject` on `ConnectableController::new`) is left out:
  it moves the error earlier and nothing more. A callback's `Fn*` bound stays even with no
  arguments (`F: FnOnce()`), as the only place its signature is written.
- **A bound an `ObservableTypes` impl does not need is not neutral**: the same rule as for a
  constructor, with a cost of its own. `Scan`'s `T: Clone` or `Filter`'s `F: FnMut(&T) -> bool`
  says nothing about `Item`, `Error`, `Mode` or `Disposal`, so leaving it there only moves the
  error earlier — and, since `ObservableTypes` is the trait one asks for a type alone
  (`JoinedModeOf`, `ChainDisposalOf`, every `OE: ObservableTypes` where-clause), it propagates:
  a caller that merely reads `OE::Item` then has to prove `T: Clone` too. The `Observable<OR>`
  impl and the fluent method carry those bounds, which is where they belong. What the where-clause
  does need is what the associated types name (`OE: ObservableTypes<Item = T, Error = E>`,
  `<OE as ObservableTypes>::Mode: ObserverMode` for an `Item` built on the mode, `F: Fn(…)` when
  the `Disposal` names `F`).
- **Derives** on a generic type go through `#[derive(Educe)]`, not `#[derive(...)]` (which bounds
  every type parameter) and not hand-written impls. A field that cannot or should not be printed
  gets `#[educe(Debug(ignore))]`. Educe infers the bounds from the field types, and leaves out those
  it knows always hold (`PhantomData`, `Arc` / `Rc` for `Clone`, function pointers, a `Vec<T>`
  reduced to `T`), but it matches type names literally: through a type alias (`MarkerType`) or an
  associated type (`M::Ptr<_>`) it emits the whole field type as a bound. Where that bound would
  name a mode pointer or a private type, set it explicitly, `Clone(bound())` or
  `Clone(bound(S: Clone))`, so that the public impl does not list it. Write an impl by hand only
  when it is not structural, e.g. a `Default` that calls a constructor.
- **Time comes from the scheduler's clock**, never from `Instant::now()`: an operator reads
  `scheduler.clock().now()` at subscription or on an event, and the observer given to the source
  holds the `Clock` (`SchedulerTypes::clock()`), a `Send` handle, rather than a scheduler that may
  be `Local`; a task handler uses the `now` its step
  is given (the fourth argument of a `TaskHandler`). An operator that
  measures time without scheduling anything (`throttle`, `timestamp`, `time_interval`) takes a
  `C: Clock`, not a scheduler: `scheduler.clock()`, `SystemClock`, or `time.clock()` in a test.
  That is what lets `VirtualTime` drive every operator; CI fails on an
  `Instant::now` in `src/` outside `scheduler/mod.rs` and `scheduler/virtual_time.rs`. The clock
  may be read under a context lock (decision 0008); a value every path needs is read before it
  (`let deadline = self.clock.now() + span;` before `update`), which keeps the lock short.
- Public items get doc comments; operators link to their reactivex.io page. `#![deny(missing_docs)]`
  enforces it, so an item that is `pub` only because the type system demands it — an operator's
  observer — takes `#[doc(hidden)]` instead of a doc comment.
- **Turbofish**: write only the type arguments inference cannot reach — in the sources, the doc
  examples, the README and `examples/`, for every operator, subject and method call
  (`.collect::<…>()`, `Err::<…>`), not only constructors. Drop the turbofish when the code still
  builds; otherwise turn every argument that can be inferred into `_`:
  `PublishSubject::<_, Infallible, _>::local()`, not `PublishSubject::<&str, Infallible, _>`. A
  subject's error type is the usual exception, since nothing but a termination names it, unless
  the subject is joined to a source that fixes it
  (`Just::new(1).merge(PublishSubject::local())`).
  Check each change by running the doctest or example it is in — and see that the doctest filter
  matched some test, since one that matches none passes. A `compile_fail` example changes together
  with its twin, and must still fail for its own reason, not for a type annotation now missing.
- `Termination<E>` (`Completed` / `Error`) is the single termination type — do not introduce
  parallel representations.
- **`Flow`**: `Observer::on_next` returns `Flow::Continue` / `Flow::Stop`. `Stop` is a guarantee —
  the observer accepts nothing more and must not be terminated, so the caller stops pushing and
  drops it; `Continue` is only a hint ("no stop seen here"), so a source must still honor its
  disposal. An operator that forwards values returns what its own downstream returned, so that the
  answer reaches the source; an operator that ends its own stream returns `Stop`. A context-based
  operator gets it from the `Updated` an `update` answers (`.flow()`) or from `send_next`;
  `send_termination` answers nothing, since a termination is the last event anyway. The model
  stops its own caller with `UpdateOutcome::answering_stop` (a replaced `switch` inner), never
  with a `Flow` in the update's result. An `on_next` that does more work after its first update —
  subscribing an inner observable or a duration, as the flattening operators and `join` do — takes
  only the result of the earlier ones (`.result()`, or a match on `Updated::Ran { result, flow }` to
  skip the work once the flow says `Stop`), and answers what its *last* context call says, the
  `.flow()` of an `update` or a `send_next`: that work may have ended the stream, and a source still
  emitting from inside its own `subscribe` has no disposal yet, so only the flow stops it. An
  earlier flow, or a `let _ =` on that last update, answers `Continue` for a stream that is over. A
  termination queued behind a delivery running on another thread still lets the
  context's updates run, so such an `on_next` may still start that work — call the selector,
  subscribe the inner observable — for a value racing that termination; the flow of its last
  update then stops the source, and the context disposes what it stored once the stream has
  ended. No `is_active` check in front of that work: it costs a second lock on every value, and
  narrows a window it cannot close, since the work runs outside the lock anyway. The
  `#[must_use]` is what catches a
  forgotten forward — `let _ =` only where ignoring it is deliberate. A disposal does not pass
  through the operators — it goes from the subscription to the source — so an operator that sends
  several notifications for one event (`take_last`, `buffer_with_count`) is stopped between two of
  them by the flow alone, and carries no flag of its own for it: the explicit `stop_on_disposal`
  operator turns a disposal into a `Stop` at the end of a chain that needs it. User callbacks
  (`subscribe_with_callback`) return `()` or a `Flow`, via `observer::IntoFlow` (a
  diverging `|_| unreachable!()` needs `-> ()`).

## Test conventions

The public API is tested by integration tests, in `tests/`. Each file starts with
`mod tests_utils;` and builds sources with `test_channel()` plus a `Checker` observer, asserting on
`checker.values()`, `checker.state()` and the channel state after each event.

What is `pub(crate)` cannot be reached from `tests/`, nor from a doctest, so it is tested by a
`#[cfg(test)] mod tests` at the bottom of its own file (`cargo test --lib`), with `use super::…`.
Those tests cannot use `tests/tests_utils/` either: keep the few helpers they need local to the
module, and keep them small — a crate-private item is a building block, tested on its own state
machine, while its effect on the operators stays covered by the operators' integration tests. Do
not make an item public only to test it from `tests/`.

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
`shared`, `unicast_subject::shared`; erase with `into_boxed` / `SendBoxedObserver`. A source
of another mode that must sit among erased `Shared` observables of one type gets `.into_shared()`.
Name the mode (`Shared`, from `rx_rust::thread_mode`) only where inference cannot reach it, and
write `_` in `let` annotations. Keep test state in plain `Arc<Mutex<_>>`, reached through
`MutableHelper`, or in atomics (`AtomicBool`, `AtomicUsize`) with their own `load` / `store` and
`Ordering::SeqCst`, and bound closures with `Send`. Tests that need real
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

A test that checks which thread a callback runs on owns a `ThreadCheckerExecutor`
(`tests/tests_utils/thread_checker_scheduler.rs`) and gives the operator its
`thread_1.scheduler()`, a `ThreadCheckerScheduler` that does not keep the executor alive.
`run_task` only queues a task; the test calls `thread_1.run_until_stalled().await` on the executor
to run the queued work on that named thread and wait for it — never a `sleep`. Dropping the
executor stops its thread, which drops its tasks there, even if scheduler clones or task disposals
remain alive. It waits for the thread to end unless dropped on that same thread,
where the event loop finishes the shutdown after the current task returns. The test, not the OS,
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

## Agent skills

### Issue tracker

Issues and specs live as Markdown files under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the five canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: root `GLOSSARY.md`, with design decisions in `docs/decisions/`. See `docs/agents/domain.md`.
