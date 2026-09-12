# AGENTS.md

Operational notes for AI coding agents working in this repository. Human-facing docs live in
[README.md](README.md) (usage) and [DEVELOPMENT.md](DEVELOPMENT.md) (TODOs and the review checklist).

## What this is

`rx-rust` is a ReactiveX implementation for Rust: `Observable` / `Observer` / `Disposable` plus a
large set of operators. Edition 2024, MSRV 1.88, `#![forbid(unsafe_code)]`, zero required runtime
dependency — the async runtime is selected by feature flag.

## Testing: run the smallest thing that answers the question

The suite is large (~112 integration test files, ~2000 test functions) and many tests are timing
based, so a full run is slow and wasteful. **Default to the narrowest scope, and widen only when the
change justifies it.**

1. One test — the normal case while iterating:
   ```bash
   cargo test --features tokio-scheduler --test map test_completed
   ```
2. One file — after finishing an operator:
   ```bash
   cargo test --features tokio-scheduler --test map
   ```
3. A few related files — only when the change is cross-cutting (e.g. a `utils/` change):
   ```bash
   cargo test --features tokio-scheduler --test switch --test merge_all --test subscribe_with_context
   ```
4. Full suite — **only when the user explicitly asks for it**, or before a release. Never run it
   "just to be safe" after a local change.

Rules of thumb:

- Compile errors are cheaper to find than test failures: use `cargo check --features tokio-scheduler
  --tests` when you only need to know whether it builds.
- Changing `src/operators/<group>/<name>.rs` → run `tests/<name>.rs`. The mapping is one-to-one for
  nearly every operator and subject.
- Do not re-run a test that already passed unless the code under it changed.
- `cargo nextest run` also works (see `.config/nextest.toml`, which fails tests that leak) and takes
  the same `--features` / filter arguments; prefer it when a test may hang.
- Never add `--all-features`: the scheduler features are mutually exclusive and it will not compile.

## Features

Test code contains `compile_error!("At least one scheduler feature must be enabled")`, so
`cargo test` **must** carry a scheduler feature. Use `tokio-scheduler` unless the change is specific
to another runtime.

`single-threaded` / `local-pool-scheduler` are mutually exclusive with `thread-pool-scheduler`,
`tokio-scheduler`, `async-std-scheduler`, `smol-scheduler`. Code that must work under both
single-threaded and multi-threaded builds goes through `utils::types` (`Shared`, `Mutable`,
`MaybeSend`, `MaybeSync`), never `Rc`/`Arc` or `Send`/`Sync` directly.

Verify single-threaded builds separately when touching shared state:

```bash
cargo check --features local-pool-scheduler --tests
```

## Other commands

```bash
cargo fmt
cargo clippy --features tokio-scheduler --all-targets
cargo doc --open
cargo tarpaulin --out Html --features tokio-scheduler
```

## Layout

- `src/observable/`, `src/observer/`, `src/disposable/` — core traits. `ObservableExt` in
  `src/observable/mod.rs` is the fluent API; every operator gets a method there.
- `src/operators/<category>/<name>.rs` — one operator per file, categories mirror reactivex.io.
- `src/subject/`, `src/scheduler/` — hot sources and runtime adapters.
- `src/utils/` — shared machinery: `mutable`, `types`, `serialized_delivery`,
  `subscribe_with_context`, `pending_events`.
- `tests/<name>.rs` — integration tests, one file per operator; shared helpers in
  `tests/tests_utils/`.

## Code conventions

- No `unsafe`. The crate-level `forbid` makes this a hard error.
- **Locks**: reach a `Mutable` through `MutableHelper::with_mut` / `with_ref`, or through the
  one-shot helpers in `MutableExt` (`clone_value`, `replace_value`, `take_value`) — never
  `lock()` / `borrow_mut()`. The callback gets a `&mut T` / `&T`, so a guard cannot escape it;
  what it must not do is run anything that can take the same lock again, dropping an owned value
  included. Take the value out under the lock and act on it afterwards (`take_value().map(...)`
  for a `Mutable<Option<_>>`), or return an action from the callback and run it after `with_mut`
  returns. Release the context lock before calling `on_next`, and both the context and observer
  locks before `on_termination`. One "transaction" should take the lock once, not repeatedly.
  Debug builds panic when a thread takes a lock it already holds. See the module docs in
  `src/utils/mutable.rs`. The tests call those methods directly too.
- **Subscription helpers**: prefer the existing helpers over hand-rolled state:
  `subscribe_with_context` when the operator can only terminate from inside the source's own
  `on_termination`; `subscribe_with_context_owning_source` when it can terminate while the
  source is still active (notifier, scheduler task, another source);
  `subscribe_with_auto_dispose_on_termination` for the simple case.
- Keep `pub` surface minimal; add `Clone`/`Send`/`Sync`/`'static` bounds only where actually needed.
- Public items get doc comments; operators link to their reactivex.io page.
- `Termination<E>` (`Completed` / `Error`) is the single termination type — do not introduce parallel
  representations.
- **`Flow`**: `Observer::on_next` returns `Flow::Continue` / `Flow::Stop`. `Stop` is a guarantee —
  the observer accepts nothing more and must not be terminated, so the caller stops pushing and
  drops it; `Continue` is only a hint ("no stop seen here"), so a source must still honor its
  disposal. An operator that forwards values returns what its own downstream returned, so that the
  answer reaches the source; an operator that ends its own stream returns `Stop`. A context-based
  operator gets it from `SubscriptionContext::update_flow` / `send_next`; `send_termination` answers
  nothing, since a termination is the last event anyway. The `#[must_use]` is what catches a
  forgotten forward — `let _ =` only where ignoring it is deliberate. User callbacks
  (`subscribe_with_callback`) return `()` or a `Flow`, via `callback_observer::IntoFlow` (a diverging `|_| unreachable!()` needs `-> ()`).

## Test conventions

Integration tests (not `#[cfg(test)]` modules). Each file starts with `mod tests_utils;` and builds
sources with `test_channel()` plus a `Checker` observer, asserting on `checker.values()`,
`checker.state()` and the channel state after each event.

`DEVELOPMENT.md` holds the canonical checklist of case names (`test_completed`, `test_error`,
`test_unsubscribe`, `test_ref`, `test_async`, the hot-observable `test_*_on_next` family, the
lock-related `test_*_on_sub` family, the scheduler-related `test_*_after_next` family, …). When
adding an operator, follow the subset that applies to its shape and name the tests the same way — do
not invent new names for existing cases.

## Commits

Prefix the subject with a category in brackets, matching existing history:
`[Feature]`, `[Improvement]`, `[BugFix]`, `[Refactoring]`, `[Tests]`, `[Miscellaneous]`.

Example: `[Tests] Add drop_probe and serialized_delivery tests.`

Do not commit or push unless asked.
