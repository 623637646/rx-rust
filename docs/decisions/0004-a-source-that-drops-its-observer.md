# 0004: A source that drops its observer

What an operator does when its source drops the observer without a termination: the source has
only stopped sending, so the work the operator has already accepted runs its course.

## The source falls silent; it does not dispose

Status: adopted. `test_abandon_after_next` and `test_unsub_after_abandon`, in the files of the
six operators listed below, specify it, and `timeout::test_timeout_never` the case of `Never`.

### Before

Six operators keep work of their own going after the source has sent a value: `observe_on`,
`delay`, `debounce`, `timeout`, `buffer_with_time` and `buffer_with_time_or_count`. Each runs that
work in a scheduler task, and the task holds the operator's context weakly
(`PromotableWeakContext`, or a plain `WeakSubscriptionContext`). The disposal holds only a stop
handle. Until the source terminates and the operator promotes the handle, the one strong reference
to the context is the observer the source holds.

So when the source dropped that observer without a termination, the context went with it: the
values still queued, delayed or pending, the timers, and the downstream observer, which was
dropped without a termination. The operator treated it as a disposal.

That was a side effect of the weak handle, not a decision. The handle is weak so that disposing the
subscription while the source is still active releases the downstream observer synchronously: the
disposal stops the context, the source is then disposed and drops its observer, and with it the
last strong reference. Nobody had chosen what should happen when the source drops the observer on
its own.

### Where it happens

A source drops its observer without terminating it:

- after it was disposed, or after `on_next` answered `Flow::Stop`. The context has already stopped
  then, so nothing changes in these cases;
- when it never sends anything: `Never` drops its observer as soon as it is subscribed;
- when the producer goes away: the last handle on a subject is dropped, a callback registration
  that a `Create` adapts is torn down, or the thread that holds the emitter unwinds from a panic.

The last case is the one that matters, and it is not rare. It is also the case in which a `timeout`
is most needed, and in which it stayed silent.

### Why it was wrong

- **Asynchronous operators disagreed with synchronous ones.** A source sends `1, 2, 3` and drops
  its observer. Through `map`, the observer gets all three values. Through `observe_on`, `delay` or
  `debounce`, it got none.
- **The outcome depended on timing.** With a scheduler on another thread, the task may have
  delivered some of the values before the drop: the observer got any prefix of `1, 2, 3`. That is
  the race the test suite was made deterministic to rule out
  (see [AGENTS.md](../../AGENTS.md), `ThreadCheckerScheduler`), handed to the users instead.
- **It contradicted the crate's own reading of a drop.** `Never` drops its observer at once, so
  dropping an observer means "nothing more will come", not "cancel". The disposal and `Flow::Stop`
  already stop the context before the observer is dropped; neither needs the drop to say anything.
- **`timeout` could not report the failure it exists for.** A source that falls silent while it
  holds the observer times out. A source that dies, and drops the observer as it does, never did,
  and neither did `Never.timeout(…)`.
- **ReactiveX does not do it.** In RxJava and RxJS, `observeOn` drains its queue unless the
  subscription is disposed, and a `timeout` fires on a source that has gone quiet, whatever the
  reason.

### Decision

A source that drops its observer without a termination has stopped sending, nothing more. It is
not a disposal and not a termination.

- **The subscription lives until it terminates or is disposed**, as it does when the source is
  silent but still holds the observer. The work the operator has already accepted runs its course:

  | Operator | After the source drops its observer |
  |---|---|
  | `observe_on` | the queued values are delivered |
  | `delay` | the delayed values are delivered when they are due |
  | `debounce` | the pending value is emitted when the quiet period ends, as it is when the source falls silent; a completion would flush it at once, but there is none |
  | `timeout` | the timer fires and the stream fails with `Error::Timeout` |
  | `buffer_with_time`, `buffer_with_time_or_count` | the open buffer is emitted at its tick, and the ticks go on, with empty buffers, until the subscription is disposed |

- **The downstream observer is dropped, never terminated, once nothing more can come**: when the
  queue is drained, the delayed values are out, the pending value is emitted. That is what `map`
  and `Never` do with a source that has dropped its observer, a step later. An operator with no
  work left when the source drops the observer drops downstream at once, as before.
- **Disposing still cancels everything**, before or after the source dropped its observer. While
  the source still holds its observer, the downstream observer is released before `dispose`
  returns, as before. After the source dropped it, the downstream observer is released when the
  runtime drops the cancelled task, as it already was when the subscription was disposed after the
  source terminated (`test_unsub_after_completed`). See the mechanism below.
- **Nothing is invented.** No termination is sent on the source's behalf: a dropped observer does
  not say whether the stream succeeded.

`buffer_with_time` keeps ticking because its buffers are cut by time, not by the source, and the
subscription is still held: it ends when the user disposes it, as it does in ReactiveX. A
subscription that is no longer wanted is dropped, and dropping it disposes it, so the ticks cannot
outlive their user.

### Mechanism

The scheduler task of each of the six operators holds the context strongly: a clone of the
`SubscriptionContext`, like the one the observer of the source holds. When the source lets go of
its observer, terminated or not, the tasks keep the context until their work is done, and the last
of them releases it, and the downstream observer with it. An operator without a task alive at that
moment holds nothing, so the context is released at once. Nothing has to be promoted, and no
operator can get it wrong by holding the wrong kind of handle: there is only one.

What the weak handle did — releasing the downstream observer before `dispose` returns, although
another handle may still be alive — is done by the `Drop` of `SerializedDelivery`, the state every
handle shares:

- A disposal stops the context through its `DeliveryStop`, which cannot name the observer (decision
  0002) and leaves it parked in its cell. The source is disposed next, and drops its observer, and
  with it its handle on the context. A handle dropped while the context is stopped takes the parked
  observer out and drops it, whoever else still holds a handle. So the downstream observer is
  released before `dispose` returns, as before, although a task still holds the context. The
  handle learns that a `DeliveryStop` stopped it from a flag the stop sets, read without the lock:
  dropping a handle of a context no `DeliveryStop` has stopped takes no lock at all, so it stays
  as safe as before anywhere, under the context's own lock included, and costs nothing on the
  common path. The same
  happens for a context that owns its source subscription (`timeout`, the two buffers): stopping it
  disposes the source, which drops its handle.
- A `Flow::Stop` stops the context from inside the delivery loop, which holds the observer and
  drops it itself, as before.
- A disposal after the source has already let go of its observer — after `abandon`, or after a
  termination that left events for the task — finds no handle of the source left to drop. The
  observer is then released when the runtime drops the cancelled task. That was so before for the
  termination (`test_unsub_after_completed`), and is the one case where the release is later than
  `dispose`.

No cycle is formed: the context holds the disposals of its tasks, which cancel them, never the tasks
themselves, which their runtime owns. A scheduler whose disposal owned its task would form one,
broken only when the context stops.

### Rejected

- **Abandonment as a disposal, documented** — the behavior from before. It costs nothing, but it
  keeps the disagreement with synchronous operators, the timing-dependent prefix and the silent
  `timeout`, and writes them into the contract.
- **Abandonment as an implicit completion**, as a Rust channel ends when its last sender is
  dropped. A dropped observer does not say whether the stream succeeded, and a producer that
  panicked would look like one that finished. `Termination` stays explicit.
- **A promoting handle.** The observer of the source held its context through a
  `PromotingContext`, whose `Drop` handed a `PromotableWeakContext` to the tasks — made strong —
  unless the context had stopped; the tasks held that weak handle, a plain
  `WeakSubscriptionContext` before. It gives the same guarantees as the mechanism above, at the
  cost of three handle types, a promotion that has to happen in the right place, and a race between
  a disposal and a drop on another thread. It was the first implementation of this decision.
- **A disposal that reaches the observer through an erased pointer.** The `DeliveryStop` held a
  weak pointer to the observer cell erased to `dyn` — a new `WeakCell` of `ThreadMode`, through a
  per-mode trait like `IntoBoxedObserver` — so that a stop released the observer itself, even after
  the source had let go of it. It is the only way to cover that last case, and cost a change to the
  bottom layer of the crate, a `'static` bound on the observer, and a `Send` bound wherever a
  `Shared` source meets a single-threaded scheduler, which nothing asked for before. Not worth it
  for a case where the subscription has no source left anyway.
- **Tasks that hold the context strongly, and nothing else.** Without the `Drop` above, the
  disposal cannot reach the observer — its type names neither the observer nor a lifetime
  (decision 0002) — so a disposed subscription would release its downstream observer only when the
  runtime drops the cancelled task: later, on the runtime's thread, after the sources, and only if
  the executor is still driven. A `timeout` or a `buffer_with_time` always has a task, so every
  disposal of one would do so.
