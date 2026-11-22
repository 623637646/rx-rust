# 0011: The serialized delivery takes its queue in batches

How the delivery loop of `utils::serialized_delivery` — the one under `subscribe_with_context` and
the subjects — takes the values queued behind it, and what that changes for a stop.

## One lock per batch, a flag before each value

Status: adopted.

The loop starts with the value that found the delivery idle, delivered without the queue. Each
time it runs out of values, it takes the lock once and moves every value queued so far into a
batch of its own, then delivers that batch without the lock. `Core::refill_or_end` swaps the queue's
buffer with the loop's empty one, so a loop that keeps taking batches goes back and forth between
two allocations. The loop's buffer is released outside the lock when the loop
ends; neither buffer is kept once the loop parks its observer or terminates it.

Taking a batch, under the lock, finds a stop in the core itself. Between the values of a batch the
loop reads instead the flag of the core's `FlaggedLock`, which every stop but a termination raises
under the lock. Reading the flag takes no lock, even when raised. The raising callback may still be
running, but the loop, which then ends, takes the core's lock to stop it again, a no-op, so it waits
for that thread to finish moving the resources, not to drop them. A value already selected for
delivery may race a stop; stopping does not wait for an in-flight notification. The
termination stays with the core, so `is_active`, and the rule that nothing is queued after a
termination, still read the core alone.

What it saves is one lock per value queued behind a running delivery: values sent by other
threads meanwhile, values sent from inside a notification, the values of an `EventBatch::NextBatch`
after the first (`window`, `delay`). A single producer sending one value at a time saves nothing:
its queue is empty, and a value costs the core lock that queues it and the one that parks the
observer, plus the observer cell's lock to take and return it, as before. No benchmark measured
the contended case before this was adopted. That single producer's cost is what
[the loop notifies its observer in its cell](#the-loop-notifies-its-observer-in-its-cell) cuts.

Preserved: arrival order (a value queued during the batch queues up behind it), a stop taking
effect between any two values.

## What changes: a stop leaves the loop its batch

A stop from outside the loop — a `DeliveryStop`, from inside the notification or from another
thread — cannot reach the values the loop has taken. It drops what the queue still holds and the
resources; the loop drops the batch itself when it sees the stop, before selecting another value:
after the resources when stopping from inside the notification, or possibly while the stopping
thread still releases them when stopping concurrently.

A stop of the loop's own — its observer answering `Flow::Break(())`, or unwinding — leaves the batch
to the loop too, rather than dropping it first, which is what lets every stop be the one
`stop_core`. The order everything is dropped in, and why, is
[below](#what-is-dropped-around-the-observer).

Nothing in the crate depended on the batch being dropped inside the stop: a value's drop that sends
to the delivery is rejected either way, since the batch is dropped after the stop, and sending takes
the core lock and finds `Stopped`. The one oddity is that the values still queued, the later ones,
are dropped before the batch, the earlier ones.

## Two stops, neither touching the observer's cell

Status: adopted.

A delivery is stopped by a `DeliveryStop`, from outside, or by its loop, as the observer it is
notifying answers `Flow::Break(())` or unwinds, both through `stop_core`. The first cannot name the
observer's type, and the second has the cell to itself and takes the observer out on its own, to
drop it once the core has stopped, so `stop_core` never touches the cell: it only swaps the core for
`Stopped` and drops what it held once the lock is released. A stop of the delivery's own that took a
parked observer out of the cell used to exist too, but only the loop called it, where the cell was
always empty: the tests that drove it on an idle delivery checked a path no caller reached, and now
drive a `DeliveryStop` instead.

## The loop notifies its observer in its cell

Status: adopted.

The loop used to take the observer out of its cell to notify it, and put it back as it parked: a
single value sent to an idle delivery took four locks — the core's to start the loop, with the
cell's nested inside to take the observer, then the core's again to park, with the cell's nested
inside to put it back — and moved the observer four or five times between the cell, the
`EnqueueAction`, the loop and the `Step`. A non-context operator inlines its downstream observer
(`MapObserver<FilterObserver<…>>`), so the bytes moved per value grew with the chain behind a
context.

The observer now stays in its cell for the life of the delivery, and the core says who may lock
the cell:

- Starting a delivery only turns the core `Delivering`; the loop then locks the cell, outside the
  core's lock, and holds it for as long as it runs, notifying the observer through `&mut` in place.
  It takes the core's lock inside the cell's, only to take the next batch or to park.
- Parking turns the core `Idle` under both locks; the loop then releases the cell with nothing run
  in between. Whoever finds the core idle next, and starts a delivery, may wait on the cell for
  that moment, while the previous loop waits on nothing.
- Every other end of the loop — a termination, a stop found, an observer answering
  `Flow::Break(())` — takes the observer out of the cell once the loop has released it, so that
  `on_termination` and every drop run with no lock held. The core says nobody else may lock the
  cell by then, so leaving it first races nobody, and the loop itself only borrows the observer.
- A stop records in `Core::Stopped { observer_parked }` whether it left the observer in its cell
  with no loop to release it: only when it found the core idle. An event, an update or a handle
  dropped that finds the flag set clears it under the core's lock, which makes it the only one left
  to lock the cell, and takes the observer out after releasing the core's lock.

A value sent to an idle delivery takes three locks — the core's, the cell's, the core's — and moves
the observer not at all. The cell's lock is never taken under the core's, so the two locks have one
order. `on_next` runs under the cell's lock, the one lock in the crate held across user code by
design (decision 0008): nothing the observer reaches takes it, since every path back into the
delivery either queues behind the loop or finds a stop that left nothing parked. A re-entry that
did take it would fail the re-entry check of debug builds.

An `on_next` that unwinds leaves the observer in its cell. The loop arms a guard for its whole run,
outside the cell's lock: the unwind releases the cell, then the guard ends the delivery as an
observer answering `Flow::Break(())` does, with the same function. It carries no state and is
disarmed as the loop returns: a guard holding the batch, as first written, made a `Local` delivery
about a tenth slower per value, more than one lock fewer saves under a `RefCell`.

Measured with `benches/serialized_delivery.rs` and `benches/subject.rs` against the code before,
both run alternately on one machine. Under a `Mutex` the gain grows with the observer the context
inlines: `take_until/shared` about 16% faster, with eight thin `map`s about 25%, with four 1 KiB
`map`s about 75% (four times the throughput); `publish/1_subscriber/shared` about 9%,
`publish/4_threads` about 16%. Under a `RefCell`, where a lock costs little, `take_until/local` is
about 12% faster and `publish/1_subscriber/local` about 4%.

`test_what_a_stopped_observer_sends_or_drops_leaves_its_cell_to_the_loop` pins that a stopped
observer re-entering the delivery never takes the cell; the concurrent-stop and panic tests pin
the rest, unchanged.

### Rejected

- **Claiming a parked observer under the core's lock**, as the old code took it out there. A loop
  holds the cell and then takes the core's lock; a thread that held the core's lock and then waited
  on the cell would deadlock with it — a handle dropped as a disposal on another thread drops the
  source, while the loop finishes a notification. Clearing the flag under the core's lock and
  taking the observer after it is released leaves one lock order.
- **A `Stopped` that only says whether a loop was running.** It answers the same question for
  whoever finds the stop first, but every event and handle dropped after it would lock the cell
  again to find it empty. Recording whether an observer is left parked, and clearing it as it is
  claimed, makes the release happen once.
- **Stopping the core from inside the cell's lock** when the observer answers `Flow::Break(())`:
  `stop_core` drops the resources, disposing sources and running their callbacks, which would then
  run under the cell's lock. A stop the observer makes from inside `on_next` does run them there,
  but that is the observer's own call; the loop's stop leaves the cell first.
- **A disposal that names the observer through an erased pointer**, so that the observer could live
  in the core and a value take two locks: already rejected in
  [0004](0004-a-source-that-drops-its-observer.md#rejected), for the bounds it adds.

## What is dropped around the observer

Status: adopted.

Whoever releases an observer drops, in this order: the resources, with what is queued for it, then
the observer, then the values it will never see — the loop's batch, the event or update a stopped
delivery rejected, what is left of a replay. The documentation of `utils::serialized_delivery`
("Drop order") says what that comes to for a delivery. The rule is the crate's, for code that owns
an observer and lets go of it; code that only borrows one releases nothing, so the values it holds
may go first (the step of `observe_on`'s task, whose observer the task owns).

A resource is what a disposal of the subscription releases: the source's subscription, the stream a
task drives, the resource of `Using`. An iterator that `FromIter` or `StartWith` sends is not one.
It lives inside the `subscribe` that sends it, whose disposal does not exist yet, so what is left
in it are values the observer will never see. One that holds more than values, such as a file, is
still dropped before the completion, or before the source of `StartWith` is subscribed, since it is
exhausted by then. A stop in between drops the observer first; `Using` is for a resource that must
go before it.

**Before the values**, because the observer is the one thing with a single owner responsible for it:
the loop that has the cell to itself, or the call that claimed a parked one, leaves nobody else to
take it out. Released first, it cannot be lost to a value whose drop unwinds, and the values need no
care in turn, since they are locals that the unwind drops. The order adopted first, the batch before
the observer, took a tuple of the two, a second panic guard around the batch, and a rejection that
took the observer out of its cell before dropping what it rejected and dropped it after.

**After the resources**, because a downstream that ends its own stream is released once its source is
disposed, so that its release tells it the source is gone: `AutoDisposeOnTerminationObserver` and
`observe_on` (`observe_on::test_stop_on_next_releases_observer_last`) do the same. The loop takes the
observer out of its cell before it stops the core, so that a resource whose drop unwinds does not
lose it either. Only a termination is the other way round, as the observer is told of the end before
its source is disposed.

`test_stop_from_on_next_drops_the_values_that_are_still_queued`,
`test_an_observer_that_stops_drops_the_values_that_are_still_queued` and
`test_a_panic_during_a_batch_drops_the_remaining_values_outside_the_lock` pin the whole order.

What else follows it, mostly by the order of a declaration:

- the replay of a `SerializedMulticast` and of the unicast subject, and `emit_then_terminate`, whose
  observer is declared after the values so that it is dropped before them, `FromIter` through it,
  and `StartWith`;
- the observers that hold values, which declare their downstream observer first;
- `Using`, whose resource is dropped before the downstream observer as it is before a termination;
- a `Task`, whose pinned state — the stream or the future that is the source — is dropped before its
  context, which owns the observer, whether or not the task ever ran, and on `VirtualTime` as under
  `drive`.

What does not, on purpose: `Hold` and `DoFinally` release and run what they hold once downstream is
done, which is what they are for.

### Rejected

- **The observer before the resources.** A `DeliveryStop` cannot name the observer, so a disposal
  drops the resources first whatever the loop does: a downstream that stops would be released in
  another order than one that is disposed. It saves nothing more than the order above, and needs the
  stop of the core split from the drop of what it held, as an observer dropped while the core still
  accepts events could send to it.

## The unicast subject's replay takes its queue the same way

Status: adopted.

The unicast subject is not built on the serialized delivery: its replay, which runs once as the
observable end is subscribed, used to take the lock of its pipe once per buffered value. It now
takes the values queued so far with `PendingEvents::take_values_into`, which swaps the buffers in
the same way, and delivers them without the lock; values the sender queues meanwhile wait behind
the batch, and the termination stays in the queue.
No disposal exists during the replay, so only the observer can stop it between two values of a
batch, by answering `Flow::Break(())` or unwinding. The pipe is then closed, which drops what is still
queued, and the replay drops the observer, then the rest of its batch: the same order as above.
`test_stop_on_next_while_replaying_releases_observer_before_values` and
`test_panicking_replay_releases_observer_before_values` pin it. A sender that finds its parked
observer disposed drops it before the event it was sending, too.

Once the sender has taken the observer, it never looks at the pipe's state again, so it now lets
go of its pointer to it: the buffer's allocation is freed once `subscribe` has returned too, rather
than kept until the sender is dropped. The sender keeps the flag, which the disposal raises.
