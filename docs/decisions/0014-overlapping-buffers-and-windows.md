# 0014 — Buffers and windows: one state machine per trigger, the operators without a skip or a shift built on those with one

## Status

Adopted.

## Context

`BufferWithCountAndSkip` and `WindowWithCountAndSkip` start a bundle or a window every `skip`
items, each for `count` items: with a `skip` below `count` several are open at once, and every
item goes to all of them, so the items must be `Clone`. ReactiveX has the same with time — a
bundle or a window every `time_shift`, each for `time_span` (`Buffer(timeSpan, timeShift)` in
Rx.NET, `buffer(timespan, timeskip)` in RxJava, `bufferTime(span, creationInterval)` in RxJS) —
which the crate lacked.

`BufferWithCount` is `BufferWithCountAndSkip` with `skip` equal to `count`, `BufferWithTime`
`BufferWithTimeAndShift` with `time_shift` equal to `time_span`, and the same for the windows:
the oldest closes as the next opens, so one is open at a time. The two forms differed only where
they need not: a state machine each, and the `T: Clone` of the general one, which a skip or a
shift equal to the length never uses.

## Decision

### The operators without a skip or a shift are the ones with it

Each operator stays a public type of its own, in a module of its own, with its documentation and
its constructors; "Rejected" says why the two of a pair are not one type with a parameter.
`BufferWithCount`, `WindowWithCount`, `BufferWithTime` and `WindowWithTime` subscribe the
observer of the general operator, given the length as the step and `no_overlap` as the duplicate:

| Operator          | Built on                 | Step        |
|-------------------|--------------------------|-------------|
| `BufferWithCount` | `BufferWithCountAndSkip` | `count`     |
| `WindowWithCount` | `WindowWithCountAndSkip` | `count`     |
| `BufferWithTime`  | `BufferWithTimeAndShift` | `time_span` |
| `WindowWithTime`  | `WindowWithTimeAndShift` | `time_span` |

Where the general operator has a `Disposal` of its own, the one without a skip or a shift
re-exports it from its own module, so that the type keeps its path there.

### Cloning is a function pointer, not a bound

The `Overlap` holding the open sinks — in `Bundles` for a buffer, in the `DelegateObserver` for a
window — holds a `duplicate: fn(&T) -> T`, which clones an item for every one open but the newest.
The general operator passes `T::clone`, and asks `T: Clone` in its `Observable` impl and its
fluent method only. The operator without a skip or a shift passes `no_overlap`, an
`unreachable!`: with one open at a time, the loop over "all but the newest" never runs, so it is
never called, and nothing asks `T: Clone`.

That one open at a time is a property of the step, not of a type: a mistake there panics rather
than failing to compile. A type could carry it only by keeping the single sink in storage of its
own (a `Vec` beside a `VecDeque<Vec<T>>`, one sender beside a queue of them), which is the whole
state machine twice, and what this replaces. The flattening operators make the same trade with a
store that queues nothing (decision 0013).

### One state machine per trigger, generic over the groups

A buffer and a window opened and closed the same way differ only in what happens to their groups
— the bundles of a buffer, the windows of a window — as they open, take an item, close and end:

|              | A bundle                         | A window                                  |
|--------------|----------------------------------|-------------------------------------------|
| opens        | an empty `Vec`, emitting nothing | a new window, **emitted**                 |
| takes an item| pushed                           | sent to the window                        |
| closes       | **emitted**                      | completed                                 |
| completion   | the ones not empty emitted       | each completed                            |
| error        | discarded                        | each errored                              |

So each of the four triggers is one state machine in `transforming/internal/`, generic over the
crate-private `Groups` (`group.rs`), and each of the twelve operators is one call to it:

| Machine             | Buffers                                         | Windows                                         |
|---------------------|-------------------------------------------------|-------------------------------------------------|
| `count`             | `buffer_with_count`, `buffer_with_count_and_skip` | `window_with_count`, `window_with_count_and_skip` |
| `time_shift`        | `buffer_with_time`, `buffer_with_time_and_shift` | `window_with_time`, `window_with_time_and_shift` |
| `time_or_count`     | `buffer_with_time_or_count`                     | `window_with_time_or_count`                     |
| `boundary`          | `buffer`                                        | `window`                                        |

What sets an operator apart beyond its groups stays in it: the step and the duplicate it passes,
the room of its first bundles, and, for `boundary`, what a completed boundary does: a buffer
ends, a window only stops rotating, as before. That one is fixed at compile time, a
`const ENDS_STREAM: bool` of the boundary observer that each operator's `BoundaryObserver` alias
sets, rather than a value the observer carries and matches at run time.

### Where the groups live

The two kinds differ in one thing that matters to a machine with a lock: putting an item into a
bundle runs no user code, putting one into a window does (its subscriber). So:

- `Bundles` (`bundle.rs`) live in the state the machine keeps under its lock, and an item is
  pushed there, in the one update the machine takes anyway. Only a finished bundle is an event.
- `Windows` keep nothing under the lock. Every change is an `Action` for the `DelegateObserver`
  (`window.rs`), which holds the senders and acts outside the lock; an item is an action of its
  own, queued by the same update.

Every hook emits to an observer it gets by reference, `put` included, so that a machine treats
both kinds alike: one `update_groups` per change, which runs the hooks under the context's lock and
queues what they emitted. That observer is an `Events`, which holds one event without a `Vec`, or,
for `count`, which has no lock, the observer itself. Each hook answers the flow of the last event
it emitted and emits nothing after a `Stop`, so that `count` needs no adapter to keep the observer
from being sent anything once it stopped; the `Events` answer `Continue`, and the context applies
the stop as it delivers them.

### The bar: no new lock, no dynamic dispatch

The shared machinery adds no lock to the path of an operator that existed before — one update per
item, per tick, per boundary — and dispatches nothing dynamically: the groups, the machine and the
delegate are all generic. Heap allocations are allowed where they keep the code simpler:

- **A window's item goes through an update**, rather than a `send_next` that skips it: both take
  the one lock, and the windows then need no path of their own.
- **A bundle is allocated as it opens, before the one before it is freed**, as
  `take_leaving_room` did, with room for as many items as the last bundle emitted had (before
  the first, the room the operator gives it: `count` where there is one). A count operator's
  bundles all close full, so their room stays `count`; `buffer_with_time_or_count`'s follows its
  last bundle too, so an idle one allocates nothing per tick, and a full bundle after short ones
  grows its `Vec` as it fills.
- **`Overlap` is one `VecDeque`, the newest sink at its back**: its buffer is allocated as the
  first group opens, one allocation per subscription. It is never shrunk, so a group opening or
  closing allocates nothing after that, and `send` takes the newest from the back of the
  iterator, so with one group open it clones nothing and checks no index.

### What is shared below the machines: the open sinks and the timetable

`Overlap<S, T>` in `transforming/internal/overlap.rs` holds the open bundles or window senders in
opening order. They have the same length, so the oldest is always the next to close. The caller
owns each value until the copies for the older sinks have been made. A timed buffer therefore
drops the original outside its context lock when no bundle is open or cloning panics.

`Timetable` in `transforming/internal/timetable.rs` keeps two deadlines. The first timed group
opens at subscription and closes after `time_span`. The next opening is at `time_shift`, and
each deadline advances by `time_shift` when it is reached. When a close and an open coincide,
one step closes the oldest group and then opens the next one. A late timer handles one deadline
per step, with a yield between overdue steps. A deadline that does not fit in an `Instant` never
comes, and the timer finishes once it has no deadline left: at its first step when neither
initial deadline fits, otherwise after its last representable one. Nothing more comes from a
finished timer, so it no longer holds the context (decision 0004); the groups still open are
ended by the termination of the source.

## What changed in behavior

Existing operators retain their emission and termination behavior: their integration tests,
which were what checked the move, pass unchanged. A count buffer now opens its next bundle right
after the item that ends a `skip`, as a window does, rather than on the next item; the bundle
still open and empty as the source completes is left out, as before.

`BufferWithTime` and `WindowWithTime` now run the task of their time-and-shift counterparts,
whose context is the subscription's context rather than a `PeriodicContext`: a custom scheduler
that implements `Scheduler<PeriodicContext<_>>` only runs them once it implements `Scheduler<C>`
for any `C`.

## Rejected

- **One type per pair, with a parameter**: a strategy type (`BufferWithTime<OE, S, K = NoShift>`,
  as `Merge<S, L = Unlimited>`) or a `const SHIFT: bool`, with `BufferWithTimeAndShift` an alias.
  First weighed when the two forms kept different storage, so that the parameter would have owned
  all of `on_next`, two machines behind a trait. With one `Overlap` for both, it would carry only
  the second length and whether `T: Clone` is asked, and cost more than the thin types it
  replaces — a struct, a constructor and an `ObservableTypes` impl each:
  - The constructors clash. A `new` in an impl for each value of the parameter makes
    `BufferWithTime::new` ambiguous (E0034): an expression path infers the parameter rather than
    taking its default. One of them would be renamed, or the type hidden behind two aliases.
  - `T: Clone` follows the parameter, which one bound cannot: two `Observable` impls, repeating
    the largest where-clause of the module, or a hidden public trait over a marker type that
    selects the duplicate — more public surface for one function pointer.
  - The two would differ by a `true` or a marker type in error messages and in the
    documentation, and the general operator would lose its module path.
  - `Merge` takes its parameter from a fluent method on one constructor, `max_concurrency`. For
    the count pair that method is `.skip(n)`, which hides `ObservableExt::skip` (below), so the
    count and time pairs would no longer be written alike.
- **A buffer as a window collected** (`window(..).concat_map(|w| w.to_vec())`): a subject and a
  boxed observer per bundle, `E: Clone` on every buffer, and a trailing empty bundle the buffers do
  not emit; `Buffer`'s documentation lists the differences.
- **Bundles kept outside the lock, as windows are**, so that both kinds take the same path: every
  buffered item would be an event of its own, a second lock and a move of the observer each.
- **The newest sink kept apart from the older ones** (`older: VecDeque<S>` beside
  `newest: Option<S>`), so that a single group allocates no `VecDeque` buffer: every operation of
  `Overlap` then had two cases, and its emptiness an invariant between the two fields, for one
  allocation per subscription.
- **Windows behind a lock of their own**, so that they too could be fed under the machine's
  update: a second lock per item. **Groups as a trait object**, so that one machine is compiled
  once: dynamic dispatch on every hook.
- **A skip on `buffer` and `window`**, which have a boundary rather than a count or a time. Their
  generalization is a toggle — openings, and a closing observable for each — a different
  operator (RxJS's `bufferToggle`, Rx.NET's `Buffer(openings, closingSelector)`), not taken up.
- **A shift on `*_with_time_or_count`**: a full bundle restarts the timer, while a shift opens on
  a fixed rate; neither Rx.NET nor RxJava combines them.
- **`.skip(n)` as a method on `BufferWithCount`**, as `max_concurrency` is: an inherent `skip`
  would hide `ObservableExt::skip` on the operator.
