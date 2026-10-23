# Design decisions

Designs that were discussed and adopted, rejected or deferred, recorded so the argument does not
have to be repeated. One file per topic, numbered in the order the topics came up; a file may hold
several decisions, each with its own status.

A record says what holds now, and why. When a decision changes, rewrite the record to the new
conclusion and move the old one to a "History" section, keeping what it ruled out and why: that is
what stops the old design from being proposed again.

| # | Topic |
|---|---|
| [0001](0001-no-borrowed-items.md) | No borrowed items (`type Item<'a>` GAT) — deferred |
| [0002](0002-observable-types-and-thread-mode.md) | Observable types and the thread mode: the observer is a type parameter, the thread mode is a type, schedulers run nameable tasks, erasure is explicit |
| [0003](0003-test-infrastructure.md) | The test infrastructure: one `Shared` test build on every scheduler; what must not compile, `compile_fail` doctests with a twin that compiles, not `trybuild`; a virtual-time scheduler, the time taken from the scheduler |
| [0004](0004-a-source-that-drops-its-observer.md) | A source that drops its observer without a termination has fallen silent, not disposed: the operator's own work runs its course |
| [0005](0005-the-context-owns-its-source.md) | The context of `subscribe_with_context` owns the source subscription and disposes it as soon as it stops; the non-owning helper is gone |
| [0006](0006-observe-on-task-owns-its-observer.md) | The task of `observe_on` owns the downstream observer, which never leaves the scheduler's thread and need not be `Send`; a disposal releases it promptly, not at once |
| [0007](0007-a-subject-per-round.md) | A factory, a context and a `fn` pointer, creates the subject of a connectable, and the subject is reset after each round: on termination, on `disconnect`, when a ref count drops to zero — each configurable |
| [0008](0008-user-code-under-a-lock.md) | `Clone`, `PartialEq`, `Clock::now` and a connectable's subject factory may run under a lock; their re-entry is a contract on the user's code, not something the crate works around |
