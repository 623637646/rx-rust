# 0011: The serialized delivery takes its queue in batches

How the delivery loop of `utils::serialized_delivery` — the one under `subscribe_with_context` and
the subjects — takes the values queued behind it, and what that changes for a stop.

## One lock per batch, a flag before each value

Status: adopted.

The loop starts with the value that found the delivery idle, delivered without the queue. Each
time it runs out of values, it takes the lock once and moves every value queued so far into a
batch of its own, then delivers that batch without the lock. `PendingEvents::take_values_into`
swaps the queue's buffer with the loop's empty one, so a loop that keeps taking batches goes back
and forth between two allocations. The loop's buffer is released outside the lock when the loop
ends; neither buffer is kept once the loop parks its observer or terminates it.

Taking a batch, under the lock, finds a stop in the core itself. Between the values of a batch the
loop reads instead the flag of the core's `FlaggedLock`, which every stop but a termination raises
under the lock. Reading the flag takes no lock, even when raised: the loop then drops its batch and
observer without inspecting the core. The raising callback may still be running, so this does not
wait for the stopping thread to finish moving or dropping the resources. A value already selected
for delivery may race a stop; stopping does not wait for an in-flight notification. The
termination stays in the queue, so `is_active`, and the rule that nothing is queued after a
termination, still read the queue alone.

What it saves is one lock per value queued behind a running delivery: values sent by other
threads meanwhile, values sent from inside a notification, the values of an `EventBatch::NextBatch`
after the first (`window`, `delay`). A single producer sending one value at a time saves nothing:
its queue is empty, and a value costs the core lock that queues it and the one that parks the
observer, plus the observer cell's lock to take and return it, as before. No benchmark measured
the contended case before this was adopted.

Preserved: arrival order (a value queued during the batch queues up behind it), a stop taking
effect between any two values, the resources released before the observer.

## What changes: a stop leaves the loop its batch

A stop from outside the loop — a `DeliveryStop`, from inside the notification or from another
thread — cannot reach the values the loop has taken. It drops what the queue still holds and the
resources; the loop drops the batch, with the observer, when it reads the flag, before selecting
another value. Those values are dropped by the loop rather than inside the stop: after the resources
when stopping from inside the notification, or possibly while the stopping thread still releases
them when stopping concurrently.
`test_stop_from_on_next_drops_the_values_that_are_still_queued` pins the new order.

A stop of the loop's own — its observer answering `Flow::Stop`, or unwinding — drops in the same
order: it stops the core, which drops the queue and the resources, then drops its batch and its
observer (the unwind does, after a panic). One order for both stops, rather than the loop dropping
its batch first, is what lets every stop be the one `stop_core`, and the panic guard nothing but a
call to it. `test_an_observer_that_stops_drops_the_values_that_are_still_queued` and
`test_a_panic_during_a_batch_drops_the_remaining_values_outside_the_lock` pin it.

Nothing in the crate depended on the old order: a value's drop that sends to the delivery is
rejected either way, since the batch is dropped after the stop, and sending takes the core lock and
finds `Stopped`. The loop still drops its batch before its observer. The one oddity is that the
values still queued, the later ones, are dropped before the batch, the earlier ones.

## Two stops, neither touching the observer's cell

Status: adopted.

A delivery is stopped by a `DeliveryStop`, from outside, or by its loop, as the observer it holds
answers `Flow::Stop` or unwinds, both through `stop_core`. The first cannot name the observer's
type, and the second holds the observer, so neither takes it out of its cell, and `stop_core` only
swaps the core for `Stopped` and drops what it held once the lock is released. A stop of the
delivery's own that took a parked observer out of the cell used to exist too, but only the loop
called it, where the cell is always empty: the tests that drove it on an idle delivery checked a
path no caller reached, and now drive a `DeliveryStop` instead.
