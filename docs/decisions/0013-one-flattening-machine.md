# 0013 — The operators that flatten an observable of observables are one state machine

## Status

Adopted.

## Context

`MergeAll`, `ConcatAll`, `Switch` and `ExhaustAll` each had a state machine of its own, and
`FlatMap`, `ConcatMap` and `SwitchMap` were a `Map` in front of one, while `ExhaustMap` shared a
function with `ExhaustAll`. The four machines did the same things — reserve a place before an
inner observable is subscribed with the lock released, hand the place over between its
subscription and its completion, complete once the source and every inner observable are done —
and had drifted apart where they need not differ:

- `ConcatAll`, which is `MergeAll` with a `max_concurrency` of one, had no guard against a panic
  while an inner observable is subscribed, which the three others had: its slot stayed reserved,
  every later inner observable waited in the queue, and the stream never completed.
- `ConcatAll` subscribed a queued observable after a completion without looking at the flow of
  the update that took it out of the queue, which `MergeAll` does. With one inner observable at a
  time no update runs behind a queued termination, so nothing came of it; it was one more thing
  to keep in step.
- `Switch` panicked on an inner observable that arrived while the one before it was still being
  subscribed (`ReservationSlot::reserve_replacing`), relying on its source to serialize that.
- The observers and the disposals were named and parameterized four ways.

## Decision

One machine, `operators::combining::internal::flatten`, a private module as
`combining::internal::join` and `transforming::internal::window` are. It keeps the inner
observables subscribed to, never more than a `max_concurrency`, in a `ReservationMap`, and what
sets the operators apart is what becomes of an inner observable that arrives while every
place is taken. The `Store` type selects that policy and the type of any queued observables:

| Operator        | `max_concurrency` | Store        | An arrival that finds no place              |
|-----------------|-------------------|--------------|---------------------------------------------|
| `MergeAll`      | any               | `Enqueue`    | waits in line                               |
| `ConcatAll`     | one               | `Enqueue`    | waits in line                               |
| `ExhaustAll`    | one               | `Ignore`     | is dropped                                  |
| `Switch`        | one               | `Replace`    | takes the place of the running one          |

`FlatMap`, `ConcatMap`, `ExhaustMap` and `SwitchMap` are the same machine with the callback as
its selector; a `*_all` passes the identity. Each operator stays a public type of its own, with
its documentation and its constructors, whose `subscribe` is one call to
`internal::flatten::subscribe`.

A place stays reserved until the inner observable has completed and its subscription has
returned. The second of those events either releases the place or reserves it for the oldest
queued observable in the same update. `Switch` can also release a place by replacing its inner
observable. A subscription that returns after it was replaced is disposed immediately.

The names stay those of ReactiveX: `Switch` and `FlatMap`, not `SwitchAll` and `MergeMap`.
An operator reactivex.io has no page for takes the name of the Rx that has it: exhaust is only in
RxJS, which renamed `exhaust` to `exhaustAll` in version 7, so it is `ExhaustAll`, not `Exhaust`.

### Only the Rx vocabulary

The line of an `Enqueue` has no bound, as in every Rx: `max_concurrency` bounds the inner
observables subscribed to, not those that wait. For a while the line took a bound,
`max_waiting(n, QueueOverflow::DropNewest | DropOldest)` on `MergeAll`, `ConcatAll`, `MergeMap`
and `ConcatMap`, and a `KeepLatestAll` / `KeepLatestMap` — the `ConcatAll` whose line has one
place and drops its oldest — ran the one under way to its end, then the newest. Neither is an
operator of ReactiveX, RxJS, RxJava or Rx.NET, and both were removed before a release; see
"Rejected".

The limits are methods on the operator rather than constructors — `merge_all().max_concurrency(n)`
— so that each new one does not double the constructors and the fluent methods; `Merge` over a
tuple or a list takes its `max_concurrency` the same way.

### `ConcatEagerAll` is this machine and an observer that puts the values back in order

`ConcatEagerAll` and `ConcatEagerMap` subscribe the inner observables as `MergeAll` does, a
`max_concurrency` included, and emit as `ConcatAll` does: the values of an inner observable behind
the first are kept, without a bound, until every one before it has completed. They are not a
sixth store. Each inner observable is given its position by the selector that makes it — the
callback, or the identity, wrapped in one that counts, so that the machine knows nothing of
positions — and sends it with every value and once more as it completes, the machine merges those, and the observer the merge is
subscribed with emits the values of the first position and keeps the others by position. The
merge delivers one event at a time, so that observer needs no lock; it emits several values for
the one event that ends the first inner observable, and checks the `StopView` of the context
before each, as `join` does. Those observers, and `subscribe_eager`, which subscribes both
operators through them, are `combining/internal/reorder.rs`, which `ConcatEager` shares over the
merge of a source set (decision 0012).

What the model does is tested in the module (`cargo test --lib combining::internal::flatten`);
the operators keep their integration tests, which were what checked the move.

### The store is a type, and decides what the subscription names

A store that keeps observables (`Enqueue<OE1>`) names their type in the disposal and observers.
The inner observer takes the next one out of the store and subscribes it with an observer of its
own type, through a function pointer made when the operator is subscribed, for the reason
`SubscribeFn` gives. The observers' parameter is the store's `Queued` type, what that function
receives, and the function pointer's type names the observer that holds it, so it needs no
wrapper to name a recursive type.

A store that keeps none (`Ignore`, `Replace`) has no type parameter. Its `Queued` type is
`Infallible`, and its function pointer is one that is never called. The inner observers and the
disposal name only the inner observable's disposal type. A subscription whose type names the
inner observable is tied to what that observable borrows, and so is the observer it is given. An
inner observable that keeps the observer it is subscribed with in a local it borrows — the
`test_race_condition_*` tests of `switch` — then borrows that local for as long as its own
type lives, which does not compile. `Switch` and `ExhaustAll` allowed it, and still do.

This is why the overflow is a type rather than three values in the model, and why the observers
have a parameter for the type of the observables that wait.

Each operator names its store and, where its items are the observables themselves, its selector,
inside aliases of its own: `merge_all::MergeAllObserver` and `MergeAllInnerObserver` for the
`Enqueue` pair, `switch::SwitchObserver` for `Replace`, `exhaust_all::ExhaustAllObserver` for
`Ignore`, and a `*ObserverOf<.., F>` beside each for the `*_map` form, whose selector is the user's
callback. An operator built on another takes those aliases as it takes its `Disposal`: `ConcatAll`
is `MergeAll`'s pair, `ConcatMap` is `MergeAllObserverOf`. So no `Enqueue`, `Replace`, `Ignore` or
`Identity` appears in an operator's where-clause, and none of them needs a path outside its
module — the aliases are `pub(crate)`, as `flatten::SourceObserverOf` is, so the where-clauses of
the public `Observable` impls name them without widening the public API.

### The selector runs on the source's thread

A store keeps the observables the selector made, not the source's values: turning a value into
an observable as its place comes would run the selector on the thread of whichever inner
observable completes, several at once with a `max_concurrency` above one, so an `FnMut` would
need a lock of its own, around user code (decision 0008 lists what may run under a lock, and why
that list is short).

So under `Enqueue` and `Replace`, where every value has an observable made, the selector runs as
the value arrives, before the lock is taken. Under `Ignore` the place is reserved first, and the
selector runs only for a value that found one: no work is started for an ignored value, as
`ExhaustMap` already promised. A panic in the selector leaves the stream as it was in either
order: in the second, where a place is already reserved, the place is given back.

## What changed in behavior

- `ConcatAll` gives the stream up when the subscription of an inner observable panics, or the
  disposal of the one that completed, as `MergeAll` does.
- `Switch` no longer relies on its source to hand it one inner observable at a time. One that
  arrives while another is still being subscribed replaces it, and the builder of the replaced
  one gets its subscription back to drop, where `ReservationSlot::reserve_replacing` panicked.
  A source that honors the `&mut self` of `on_next` never got there, and a subject queues a value
  sent from within its own delivery, so this is a guarantee the machine no longer needs rather
  than a case a test of the public API reaches.
- `SwitchMap` and `ConcatMap` name the observers of the machine instead of a `MapObserver`
  around those of `Switch` and `ConcatAll`; the callback still runs as each value arrives.

## Rejected

- **A queue of source values, with the selector run as each place comes**, as RxJS's
  `mergeInternals` does on its one thread. See above: a lock around user code. Rust's futures
  being lazy, an observable made early usually starts nothing until it is subscribed, which the
  documentation of `FlatMap` says.
- **The overflow as a value in the model** (`max_concurrency`, a queue capacity, which one to
  drop). The three are not independent — the capacity and the victim are the store — and a
  model with a queue it never uses would put the inner observable's type in the disposal of
  `Switch` and `ExhaustAll`.
- **One public, configurable `Flatten` operator.** The policies are a closed set with names
  users already know; a public knob for each parameter would be an API to keep for
  combinations nobody asked for. `exhaust` and `switch` with a `max_concurrency` above one are a constructor away, and are not
  added until someone needs one.
- **A bound on the line of an `Enqueue`, and `KeepLatestAll`.** Outside the Rx vocabulary, and
  the second is the first with one place that drops its oldest: two names for one knob nobody
  had asked for. Without them the line is a `VecDeque` and nothing more, and an arrival is never
  handed back to drop.
- **`SwitchAll` and `MergeMap`**, so that every flattening is `x_all` and `x_map`. The names a
  user looks for are those of reactivex.io, `Switch` and `FlatMap`.
- **`Exhaust`**, to match `Switch`. reactivex.io has no exhaust, and the one Rx that has it calls
  it `exhaustAll` now, `exhaust` being its deprecated name.
- **`Switch` left out of the machine**, the fallback had it needed more than its own
  admission and its check that an inner observable is still the running one. It needs exactly
  those two, both decided by a constant of the store, so the three others pay nothing for it.
- **`ReservationSlot` for the stores of one place.** A `ReservationMap` of one entry costs a
  `BTreeMap` insertion per inner observable, and `Switch` a lookup in it per value, where the
  slot compared two integers. One machine was worth more than that; a second kind of storage
  can come back as an associated type of the store if a measurement asks for it.
- **`Merge` over a tuple or a list on this machine.** Those are pulled, not pushed: `Merge`
  takes the next source only once there is room, so it has no store at all (decision 0012).
  Likewise `Concat`, which hands its observer from one source of a tuple to the next without a
  lock.

## History

Until this decision the four operators were `merge_all.rs`, `concat_all.rs`, `switch.rs` and
`exhaust_all.rs`, each with a model of its own: a `ReservationMap` and a queue in `MergeAll`, a
`ReservationSlot` and a queue in `ConcatAll`, a `ReservationSlot` in `ExhaustAll`, and a
`ReservationSlot` with an `IdGenerator`, whose latest id told a superseded inner observable from
the running one, in `Switch`. `ReservationSlot::reserve_replacing` and `reserve_if_idle`, and
`IdGenerator::latest`, existed for them alone and were removed; the slot remains for `Merge` with
a limit, whose slots are reserved once, as they are created.
