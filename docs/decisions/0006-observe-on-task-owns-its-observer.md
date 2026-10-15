# 0006: The task of `observe_on` owns its observer

Where the downstream observer of `observe_on` lives: in its scheduler task alone, not in a context
the source's observer shares.

## The task owns the observer

Status: adopted. `local_mode::test_observe_on_from_another_thread` specifies it with an observer
that holds `Rc`s, and the `compile_fail` doctest on `ObserveOn` with its twin.

`observe_on` is not built on `subscribe_with_context`. Its source's observer and its task share an
inbox — the values and the termination not delivered yet, and whether the delivery stopped — in
the mode of the source and the scheduler joined. The downstream observer sits in the task, which
alone calls it and drops it. The source's subscription sits in a `ReplaceableDisposal` that the
task disposes as it stops, so the context-owns-its-source rule (decision 0005) still holds.

The disposal stops the inbox first, then cancels the task, then disposes the source. A step of
the task takes the events queued so far and delivers them; what comes meanwhile waits for the next
step, so a step is as long as what was queued when it began, however fast the source keeps
sending. Before every value it delivers, the task checks the inbox, so nothing is delivered after
a disposal, even one made by the observer itself in the middle of a batch, or from another thread
while the task runs, and an error queued meanwhile preempts the values left in the batch, as it
preempts those still queued when the step begins. A task dropped before it stopped — cancelled,
unwound by a panic of the observer, or dropped with its runtime — stops the inbox and disposes the
source from its `Drop`.

### Why

- **The observer never leaves the scheduler's thread.** Only the events cross from the source's
  thread, so the source's observer is `Send` as soon as the items and the error are. With a `Local`
  scheduler the downstream observer need not be `Send`: a background source observed on the UI
  thread by an observer holding UI handles, the case `observe_on(main)` exists for. Before, the
  context the source's observer held contained the downstream observer, behind a `Shared` pointer
  as soon as the source was `Shared`, which required it to be `Send`.
- **Nothing is shared that does not have to be.** The observer is called by the task only, so it
  needs no lock and no serialized delivery; the inbox is one lock, taken once per batch and once per
  value.
- **The disposal stays `Send`** with a `Local` scheduler: it holds the inbox, the scheduler's task
  handle and the source's slot, none of which names the observer.
- **A dropped task stops the source.** A context held by a task that its runtime dropped stayed
  alive for the source, which kept queueing into it.

### Cost

- **A disposal releases the observer promptly, not at once.** The observer is released when the
  scheduler drops the cancelled task, on the scheduler's thread — for Tokio, the next time its
  runtime polls the task — while every other operator releases it before `dispose` returns. Nothing
  is delivered after the disposal either way; only the release of what the observer holds is late.
  The same was already true of a subscription disposed after its source had let go of its observer
  (decision 0004).
- **`observe_on` does not use the helper** that the other operators with a scheduler task use
  (`subscribe_with_context`), so its stopping, its source ownership and its panic handling are its
  own: the inbox flag, the slot, and the `Drop` of the task's state.

## History

Rejected: keeping `subscribe_with_context` with the context in the scheduler's mode and only the
queue in the joined mode. The observer would then sit in a pointer that the task and the disposal
share, which releases it at once, but a `Local` scheduler would make the disposal `!Send`, and the
operator would keep a serialized delivery for an observer that only one task ever calls.

Deferred: the same for `delay` and `debounce`. `delay` sends an error, and `debounce` a completion
with its pending value, from the source's thread at once; owning the observer in the task would
move them through the scheduler, a change of timing.
