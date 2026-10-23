# 0008: User code under a lock

Which user code the crate's own code may run while it holds a lock — the lock of a subject, of an
operator's context, of a connectable's controller or ref count.

## `Clone`, `PartialEq`, `Clock::now` and the subject factory are not kept out of a lock

Status: adopted.

The crate may call these while it holds a lock, and is not changed, nor flagged in a review, for
doing so:

- **`Clone`** — of an item, an error (`Termination<E>`), a subject, a waker, or whatever a subject
  factory clones out of its context (the initial value of `publish_behavior`);
- **`PartialEq`** — the comparison `sequence_equal` makes;
- **`Clock::now`** — the scheduler's clock, `SchedulerTypes::clock()`, or the `C: Clock` an
  operator is given;
- **the subject factory** `fn(&mut C) -> S` of a connectable (`multicast`, `publish*`, `share*`,
  `replay`; decision 0007) — called under the controller's lock when a subscriber or a connection
  needs a subject and there is none, and under the `RefCount`'s lock too when a subscriber of the
  `RefCount` needs it — together with whatever it runs.

Where one of them sits now is where it stays. A call that is already outside a lock may stay there,
when that is free: a value every path needs, read or cloned before the lock is taken, which keeps
the lock short. Moving a call out is not worth new code, a second lock or a call that only some
paths need.

The rule that remains for these is a contract on the user's types, in the README ("Item and error
types run under a lock"): their `Clone`, `PartialEq` and `now` must not reach back into the subject
or the pipeline that is running them — no `on_next`, no `subscribe`, no `value()` or
`termination()` of that same subject, no subscription to the same connectable. A subject factory
must not subscribe to, connect or disconnect its own connectable, nor dispose a subscription of its
`RefCount`; the `ConnectableController` documentation says so too. One that does takes a lock the
thread already holds: debug builds panic with a message saying so, release builds panic (`Local`)
or deadlock (`Shared`).

Everything else stays bound by rule 2 of `src/thread_mode/mutable.rs` — nothing that can take the
same lock again runs under it:

- `Drop` of a value the lock guards: replaced, evicted and rejected values, subscriptions, wakers;
- `Observer::on_next` / `on_termination`, `Disposable::dispose`, `subscribe`;
- the operators' callbacks — `retry` and `catch` run theirs in `ReplaceableDisposal::replace_with`'s
  builder, outside the lock;
- `Waker::wake`, which may run the task inline.

### Why

- **The re-entry is contrived.** It takes a `Clone`, `PartialEq` or clock written to emit to, or
  subscribe to, the very subject or pipeline that holds the value. A `Drop` that disposes a
  subscription, or an observer that emits again, is ordinary code; a `Clone` that does is not.
  Neither is a factory that does: it is a function pointer that captures nothing, so it reaches its
  own connectable only through a handle to it stored in its own context.
- **The factory's call is what makes joining atomic.** A missing subject is created and joined in
  the step that finds it missing, so two subscribers that race for it join the same one, and a
  `RefCount` counts and resets in step with the subject it joins (decision 0007).
- **Avoiding it costs more than it saves.** A consistent snapshot outside the lock needs either a
  shared `Arc<T>`, which asks `T: Sync` of items that are only `Send`, or replaying from the
  delivery loop, which still leaves `BehaviorSubject::value()` and `Subject::termination()`
  cloning under a lock, since they answer synchronously. A clock read moved out of the lock is made
  on every value instead of on the one branch that needs it, and the state then has to guard
  against a tick that ran between the read and the lock. `sequence_equal` comparing outside the
  lock would need to count the comparisons in flight, so that `true` is not emitted before a late
  `false`.
- **It is slower.** Moving these out means clocks read and values cloned on paths that do not need
  them, or an allocation per value.
- **A mistake is not silent.** Debug builds report the re-entry at the offending call, rather than
  deadlocking.

### Rejected

- **Snapshotting through `Arc<T>`** (`BehaviorSubject::value()`, replay): needs `T: Sync`.
- **Taking the value out of the lock and putting it back**: a concurrent subscriber misses it, or
  receives it twice.
- **Replaying from the `SerializedMulticast` delivery loop**: turns a re-entrant `subscribe` into
  queueing, but `value()` and `termination()` still clone under the lock.
- **An `Arc<Mutex<T>>` per current value**, so that `value()` clones under the value's own lock
  only: an allocation per `on_next`, and a `Clone` that subscribes to the same subject still needs
  that value's lock again.
- **Reading the clock before the lock in `buffer_with_time_or_count`**, and keeping the deadline
  from moving earlier with a `max` against a concurrent tick: a clock read per value instead of per
  full bundle. Written and reverted.
- **Cloning the waker before the lock in `observable_try_future`**: an executor's waker cannot reach
  the operator's private lock. Written and reverted.
- **Calling the subject factory outside the lock.** The factory needs `&mut C`, so the context
  would have to leave the controller for the call, behind a "creating" state that the other
  subscribers wait on — a wait the `Local` mode has no primitive for — or every racing subscriber
  would call the factory and all but one subject would be thrown away, calling a factory with side
  effects, or an expensive one, more than once per round.

## History

Commit `024321e2` (`docs: say which item and error traits run under a lock`) first wrote the
README contract and left the code as it was, deferring a redesign. A review then asked for these
calls to be moved out of the locks; two of them were, and reverted. This record replaces the
deferral: there is no redesign to come.

The subject factory joined the list afterwards, by the same reasoning. Its call under the lock had
been documented on `ConnectableController` since decision 0007; it is now an exemption rather than
an exception to rule 2. The file was renamed from `0008-clone-eq-and-clock-under-a-lock.md`.
