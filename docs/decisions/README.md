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
| [0002](0002-observable-types-and-thread-mode.md) | Observable types and the thread mode: the observer is a type parameter, the thread mode is a type, schedulers run nameable tasks, erasure picks its box from the mode |
| [0003](0003-test-infrastructure.md) | The test infrastructure: one `Shared` test build on every scheduler; what must not compile, `compile_fail` doctests with a twin that compiles, not `trybuild`; a virtual-time scheduler, the time taken from the scheduler |
| [0004](0004-a-source-that-drops-its-observer.md) | A source that drops its observer without a termination has fallen silent, not disposed: the operator's own work runs its course |
| [0005](0005-the-context-owns-its-source.md) | The context of `subscribe_with_context` owns the source subscription and disposes it as soon as it stops; the non-owning helper is gone |
| [0006](0006-observe-on-task-owns-its-observer.md) | The task of `observe_on` owns the downstream observer, which never leaves the scheduler's thread and need not be `Send`; a disposal releases it promptly, not at once |
| [0007](0007-a-subject-per-round.md) | A factory, a context and a `fn` pointer, creates the subject of a connectable, and the subject is reset after each round: on termination, on `disconnect`, when a ref count drops to zero — each configurable |
| [0008](0008-user-code-under-a-lock.md) | `Clone`, `PartialEq`, `Clock::now` and a connectable's subject factory may run under a lock; their re-entry is a contract on the user's code, not something the crate works around |
| [0009](0009-keep-educe.md) | educe, the one required dependency, stays: the standard derives bound every type parameter and cannot leave a field out, and it costs only a compile-time `syn` |
| [0010](0010-acknowledged-stream-sources.md) | A stream source that sends a `StreamAck` with each item and polls only while it holds a permit; dropping a `StreamAck` acknowledges it, and `hold` keeps it until the inner that handles the item ends. `concat_map_acked` was replaced by `hold` |
| [0011](0011-serialized-delivery-in-batches.md) | The serialized delivery under `subscribe_with_context` and the subjects takes the values queued behind it in one lock per batch and reads a flag before each value; a stop from outside the loop leaves it the batch, dropped after the resources |
| [0012](0012-source-tuples.md) | Fixed heterogeneous sources are a tuple, runtime homogeneous sources a list (`new_from_iter`); a source set selects its metadata and the resources operators keep for it, while subscription strategies and value readiness are separate, sharing operator state machines without erasure; collecting operators get their sources and value storage from one call. `Concat` keeps tuple pairs or limited-list Merge; cursor-based Merge and `When` retain their own resources |
| [0013](0013-one-flattening-machine.md) | `MergeAll`, `ConcatAll`, `Switch`, `ExhaustAll` and their `*_map` forms are one state machine, `internal::flatten`, set apart by what becomes of an inner observable that finds no place; the store is a type, which decides what a subscription names; the operators outside the Rx vocabulary (`KeepLatestAll`, `max_waiting`) are not kept, and `ConcatEagerAll` is the machine with an observer that puts the values back in order |
| [0014](0014-overlapping-buffers-and-windows.md) | Each way buffers and windows open and close — a count and a skip, a time and a shift, a time or a count, a boundary — is one state machine generic over `Groups`, with bundles under the lock and windows outside it; `*_with_count` and `*_with_time` are the operators with a skip or a shift equal to the length and a duplicate that is never called, so they do not ask for `Clone`, and each pair stays two public types rather than one with a parameter |
| [0015](0015-no-update-behind-a-queued-termination.md) | Queueing a termination freezes the model. Later updates return `Updated::Ended` and drop their captures outside the lock |
