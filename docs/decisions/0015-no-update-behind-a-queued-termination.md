# 0015 — Queueing a termination freezes the model

## Status

Adopted.

## Context

An inner observable can fail while `MergeAll` or `Switch` is delivering another inner observable's
value. The error waits for that delivery. Previously, an outer value arriving in that interval
could still reserve a slot and start an inner subscription whose events would all be rejected.
In `Switch` it could also replace and dispose the current inner subscription.

The same interval let an update run against a model that had already ended its stream. `ForkJoin`
ends the stream when a source completes without a value; the completion of the last other source,
racing it, could still bring the count of sources left to zero and take every last value out,
panicking on the empty one. Each operator that ends its stream from an update had to guard
against updates after that point, and none of those guards was checked.

## Decision

`SerializedDelivery::update` and `SubscriptionContext::update` run their callbacks only while
the delivery accepts events. The check and the callback run under the same lock that queues the
termination. After a termination is queued or the delivery stops, every update returns
`Updated::Ended` and drops the unused callback and its captures outside the lock. Previously
accepted events can still reach the observer in order. The variant was named `Stopped` while it
meant only a stopped delivery; it was renamed as it came to cover a queued termination too.

The delivery's core names that state: `Terminating` holds the values queued before the
termination, the termination and the resources, and offers no access to the resources, so only
`Idle` and `Delivering` accept events and updates.

Values arriving after this point cannot reserve slots, and completions cannot release a slot
to hand it to another source. Work outside the lock can still race with termination. `FlatMap`
runs its selector before attempting to reserve a slot or queue the inner observable. An inner
subscription whose slot was previously reserved can still start or finish after termination
is queued.

## Disposal order

- A subscription returned by in-flight work after a termination is queued is disposed by the
  update that tried to store it. This can happen before downstream sees the termination.
- A subscription already stored in the model stays there even if its source completes while a
  termination is queued. If the termination is delivered, the context drops its resources after
  the notification. A disposal or downstream `Flow::Break` can stop delivery earlier.

`SerializedMulticast` sends its own termination as an ordinary action. Its underlying delivery
stays available to serve late subscribers.

## Rejected alternatives

- Checking `is_active` before the update leaves a race between the check and the update.
- An update dedicated to storing late subscriptions would add another operation under the core
  lock and require callers to preserve the model decisions that led to termination.
