# 0012 — A fixed number of sources is a tuple; a run-time number is a list

## Status

Adopted.

## Context

`combine_latest`, `zip`, `merge` and `with_latest_from` took exactly two sources, each its own
type parameter (`CombineLatest<OE1, OE2>`). More sources meant nesting —
`a.combine_latest(b).combine_latest(c)` — which emits nested tuples, `((a, b), c)`, takes a lock
and a context per level, and clones the inner tuple once more at every level it is stored in. The
`*_all` operators (`CombineLatestAll`, `ZipAll`, `ForkJoin` over a `Vec`) took any number of
sources, but of one type, and emitted a `Vec`. `fork_join` existed only in that form, though its
usual use — requests run side by side — has results of different types.

Rust has no variadic generics, so one type cannot take any number of sources of different types.
`When` (`and` / `then` / `when`) already took a tuple of one to six sources, through a trait
implemented by a macro for each arity and an observer generic over the index of its source.

## Decision

Two forms, by what is known when the program is written:

- **A fixed number of sources, of any types: a tuple.** `CombineLatest<S>`, `Zip<S>`, `Merge<S>`,
  `ForkJoin<S>` and `When<S, P>` take a tuple of one to eight observables with one error type
  (`Merge` and `Concat` also with one item type); `WithLatestFrom<OE, S>` takes its other observables as one.
  The constructor is `new((a, b, c))`, and the items are a tuple of the sources' items in the same
  order: `(T0, T1, T2)`, and `(T, T0, T1, …)` for `with_latest_from`.
- **A number decided as the program runs, of one type: a list.** The `new_from_iter` of the same
  types — `CombineLatest`, `Zip`, `ForkJoin`, `Merge`, `Concat`, `Amb`, `WithLatestFrom` —
  emitting a `Vec` where they combine. `new` is the name of the constructor over an observable of
  observables, and none of those has a `new_from_iter`: a list merged with a `max_concurrency` is
  `Merge::new_from_iter(list).max_concurrency(n)`, a tuple `Merge::new(tuple).max_concurrency(n)`,
  and either concatenated is `Concat`.

The fluent methods keep their two-source form: `a.combine_latest(b)`, `a.zip(b)`,
`a.fork_join(b)` and `a.with_latest_from(b)` emit `(A, B)`, `a.merge(b)` and `a.amb(b)` a `T`. They
return the tuple operator over `(Self, B)` — `CombineLatest<(Self, OE2)>`,
`WithLatestFrom<Self, (OE1,)>` — so only code that names the types or calls `new` changes. Every
operator over a source set has one, `When` excepted, whose plans a pair of sources does not make.
They are named after the operator, with no `_with` suffix: `a.merge(b)`, `a.amb(b)` and
`a.concat(b)`; `merge_with`, `amb_with` and `concat_with` were renamed so, as one style for every method that combines `self` with another
observable.

Eight is the limit: `When` keeps a set of sources as a `u8` of bits, which a `const` assertion in
its per-arity impls checks, and `with_latest_from` emits one element more than it has sources,
which stays within the twelve elements the standard library implements `Debug`, `Clone` and
`PartialEq` for. One table, `for_each_arity!`, generates every implementation over a tuple, so
they all cover the same arities: the tuple of sources, the typed access to a tuple's elements,
the tuple `Disposable`, and `When`'s patterns and tuples of plans. It gives each element its
position and three names, `A`, `B` and `C`, with no role of their own, since what a caller needs
differs: a source, its item and its disposal; a pattern's source index as a const parameter, its
item and its disposal; a plan. A tuple of plans nests one `ControlFlow` per plan rather than
declare an enum per arity, whose name the table could not give: `Break` with the items of the plan
that matched, `Continue` past one that did not. A type of the crate's own would have to be `pub`,
as the value of an associated type of the public `Plans` that `When`'s disposal names.
A generic impl costs nothing until a program uses its arity: going from six to eight left the
library's build time unchanged.

## Shared source-set machinery

The two input shapes share their operator state machines through `observable::sources`, while
keeping their own storage and output shapes:

- `SourceSet` (public, sealed, re-exported by `observable`) describes `Item`, `Error` and `Mode`,
  `Item` being a `Combination` (see below), and selects, as hidden associated items, the resources operators keep for the sources
  that public disposals name: `Cursor`, the sources not
  pulled yet and their positions; and `SubscriptionSlots`, each source's subscription at its position,
  the one store of subscriptions: a batch fills it and holds it whole, an operator pulling its
  sources one at a time releases each slot as its source completes. They are hidden because no user names them. `SourceSet` carries types only, as the
  public `Combination` does: a method on it, hidden or not, could be called by any user who imports
  the trait. It holds only what a public signature needs: `CollectedCursor`, the cursor over every
  source once they are all collected, which no public type names, is on the crate-private
  `InternalSourceSet` (see "History"), while `Cursor` stays here because Merge and ConcatEager
  with a limit name it in their disposals. Every source set
  provides all of them, on the same bounds as its metadata, so one trait selects them rather than
  one per resource. `SameItemSources` adds the common item type that Merge and Amb need, which
  not every source set has. Inspecting source metadata requires no collection, storage, cloning
  or observer capability.
  The output and raw queues the `Combination` of the items selects are projected from it where
  they are used (`<S::Item as Combination>::After<T>` in WithLatestFrom,
  `<S::Item as Combination>::Queues` in Zip), without knowing the sources or requiring `Clone`.
  The trait is public and re-exported by `observable`. Each projection had a public alias
  (`ItemAfterOf<S, T>`, `QueuesOf<S>`); each served one operator, and an alias a public
  associated type names must stay public, so they were removed rather than made private.
- `InternalSourceSet: SourceSet` is crate-private and holds what no public signature needs: the
  operations that need no observer — `into_collected_cursor` collects every source together with
  their position count, before any source is subscribed (see below), and `into_cursor` prepares
  the cursor — and `CollectedCursor`, the type the first returns; the second returns
  `SourceSet::Cursor`. Every source set implements it on the same terms as `SourceSet`; it is
  apart so that all of this stays out of the public API.
  Collecting a tuple only prepares its cursor; `IterSources<I>` collects a homogeneous finite
  iterator into a `Vec`, and enumerates it, only when an operator needs its position count to
  initialize readiness and completion state. Fewer sources may be subscribed when the operator
  stops during registration.
- `When` uses only the cursor; merge the cursor and the slots, which its public
  disposal names (the cursor with a limit only); the other operators the batch subscriptions. Preparation bounds belong on
  `Observable`, except when its disposal itself retains the cursor and slots. An `ObservableTypes` impl needs no bound on `S::Item`
  for the storage and output types it names, since `SourceSet` already makes it a `Combination`;
  the operations and the cloning capability belong only on `Observable`.
- `ReservationSlots<K>` in `utils::positional::slots` fills and releases the value at one position, with
  no dependency on sources or disposals. Each position is implicitly reserved, filled once and
  released once, in either order; a fill after a release hands the value back. The same module
  implements it on `TupleSlots` and `ListSlots`, which hold arbitrary values and drop those left
  in reverse position order, including during unwinding if one panics. `TupleSlots` reverses its
  tuple with `IntoReversed` from the tuple machinery and drops it; neither store needs
  `Disposable`. `SourceSet`'s
  `SubscriptionSlots` retains the source-specific choice of subscription storage, wrapping each source's disposal
  in `DisposeOnDrop` before storing it.
  `ReservationSlot` lives in the same module, with three states: `Reserved`, `Filled` and
  `Released`. A release is final, whether it came before the fill or after it: a fill arriving
  afterwards is handed back, and a released slot is never reserved again. A second release and a
  fill over a stored value panic. There used to be a fourth state, `Finished`, apart from
  `ReleasedBeforeFill`, only to make a second fill after a release panic too: a check the list
  slots never had, since they remove an entry once both operations are done, and one no operator
  relied on — handing the value back, to be disposed, is the safe answer anyway.
- The tuples implement `SourceSet` and `SameItemSources` directly, one arity at a time; no trait
  of their own sets them apart from a list, since no operator needs one: `When`'s `Sources` is
  implemented for the tuples alone, and keeps the tuple-specific handles and plans. Iterators
  use `IterSources<I>`, built only by the `new_from_iter` constructors, to avoid overlapping
  implementations between arbitrary `IntoIterator` types and tuples, which the standard library
  could make iterable.
- `TupleIndex<INDEX>` and `usize` are static and dynamic positions, both a `Position`.
  `AtPosition` reaches the correctly typed storage element at one without boxing, in a tuple or a
  `Vec`; it is not named `ElementAt`, the public operator. They sit in
  `utils::positional::position`, apart from the sources, since they know nothing of what an
  element is; the arity table sits in `utils::positional::tuple`, since the tuple `Disposable` and
  `When`'s plans use it without indexing anything. `ObserverFamily` produces an observer at
  either kind of index; the operators reach their element through `AtPosition` alone. Amb also
  records its winner's position as a number, to recognize the winner's subscription when it is
  stored after the win. A
  context-based operator implements that trait on its context directly; no separate wrapper or
  extra context clone is needed to build its source observers. The tuple implementations are
  generated from one arity table. Its reversed order serves `IntoReversed` alone, and
  everything that goes from the last element to the first — `TupleSlots`, the tuple
  `Disposable`, the inputs `When` releases — reverses its tuple with it and drops the result.
- CombineLatest and WithLatestFrom keep the raw `Latest` storage, with no count of missing
  values. They replace a position through `AtPosition` (`at_mut(k).replace(value)`, the
  displaced value going out for dropping outside the lock) and ask for a combination on every
  value with `CloneLatest::clone_all`, which answers `None` until every position has a value. It
  checks the positions first and clones only then, so that no clone made before a missing
  position is found is dropped under the lock. The check is one pass over the positions, which
  cloning a complete combination takes anyway.
- ForkJoin keeps the same raw storage. It asks for its one combination
  as its last source completes, when every position has a value anyway — a source completing
  without one has ended the stream already. `LatestOps::take_all` moves the combination out
  without requiring `Clone`.
- Zip's model owns raw queues, an empty-queue count and its completed-source combination budget.
  `AtPosition` reaches each source's queue at a typed or dynamic position. Insertion decrements
  the count when a queue was empty; once no queue is empty, `QueueOps::pop_all` moves out one
  complete combination and counts the queues left empty in the same pass. An event producing
  no combination needs no readiness scan. `Combination::Queues` selects each combination's queue
  layout: heterogeneous tuples retain their types, while lists use vectors. Zip uses one model
  and observer for both forms; their raw queue type is a generic parameter, named from sources
  by `<S::Item as Combination>::Queues`. Its model is created for the position count its sources are collected with,
  and starts its empty-queue count from the queues it actually has. Readiness and completion policy stay
  together in the operator, with no
  separate queue wrapper, function-pointer dispatch or per-arity operator impls. This adds no
  `Clone`, `Send` or lifetime requirements.
- `Combination` lives in `utils::positional::combination`, alongside its tuple and vector
  implementations. It is public, sealed, and selects types only: `Latest`, the storage of latest
  values, `Queues`, and `After<T>`, the output after a primary item, which operator disposals and
  items name. One trait selects the three, since every combination has all of them on the same
  terms: three traits would constrain nothing more. What the operators do with them is in
  crate-private subtraits: `LatestOps` and `QueueOps` create storage for a position count
  and move a combination out without cloning; `CloneLatest: LatestOps`
  requires `Clone` only for cloning a combination out; `PrependOps` moves the combination after a
  primary item. A public trait carries no operation, so the storage layout and its panics on a
  missing value are no promise to users.
  WithLatestFrom clones a combination only on a primary emission, then
  prepends that item; its output type remains inspectable for items that are not `Clone`.
  CombineLatest and ForkJoin retain completion accounting and the rule that completion without
  a value ends the stream. ForkJoin takes its combination by moving its items. Zip retains its completed-source
  budget: a completed source's queue only shrinks, so the fewest items left in one bounds the
  remaining combinations. Empty-input termination stays with each operator; empty auxiliary
  storage remains ready for WithLatestFrom.
- `SubscribeBatch<F, SL>` subscribes every source a collected cursor has left, at once, through
  `subscribe_batch`. It is crate-private and returns the guarded slots. It has one impl,
  over `CollectedCursor` (see "One subscription path" below); the trait remains as the bound an operator
  writes instead of naming the batch. CombineLatest, Zip,
  ForkJoin and WithLatestFrom collect their sources first and subscribe
  `InternalSourceSet::CollectedCursor` through it. Lists store their subscriptions as they consume sources,
  without reserving a length. Enumeration checks for a stop
  both before pulling and before subscribing: `Iterator::next` can re-enter a source already
  subscribed and end the stream itself.
- `SubscribeEach` hands each remaining source of a cursor, in order, to a `SourceSubscriber`
  implemented once per position and source type, so that the bounds an operator needs to
  subscribe a source sit on its own impl. Each cursor implements it directly, in
  `observable::sources::subscribe`: a tuple of `Option`s, per arity of the table in
  `utils::positional::tuple`, retains the sources not taken yet, and an `Enumerate<I>` retains an
  iterator and its next position. The cursor is the receiver, so neither a separate
  remaining-elements associated type nor an external position is needed. Each source returns
  `ControlFlow<B>`; breaking consumes that source and leaves later ones for the next call. A
  subscriber whose break type is `Infallible` is handed every source by one `subscribe_each`, a
  complete pass. `When` uses that rather than
  batch subscription: it checks, source by source, that a plan still waits on the source before
  subscribing it (its `observer` answers `None` otherwise), and stores each subscription in that source's input as soon as it is
  made, so that a later source releasing it disposes it at once. Its context is the subscriber,
  as it is the `ObserverFamily` of the others.
  `InternalSourceSet::into_cursor` prepares the cursor without a subscriber: a tuple wraps its sources in
  `Some`, and `IterSources<I>` enumerates `I::IntoIter`. Preparation does not collect the iterator
  or subscribe any source. The iterator cursor's traversal uses the iterator's own `try_for_each`.
- A batch stores its subscriptions in the source set's `SubscriptionSlots`, released last
  subscribed first even when iteration or subscription panics: the slots drop their values in
  reverse position order, and the backends make slots of guarded disposals a `Disposable` whose
  disposal is that drop. The batch returns them in a `DisposeOnDrop`, as
  `Observable::subscribe` returns its subscription; no
  builder of `subscribe_with_context` ever holds an unguarded subscription.
- Amb subscribes its cursor through `SubscribeEach`, as merge with a limit does, while the race is
  open, and stores each subscription in its `SourceSet::SubscriptionSlots` as soon as it is made.
  The winner swaps the slots for fresh ones from `Default` and moves its own subscription across
  at its typed position (`ReservationSlots::release`, then `fill`); the old slots, the losers'
  alone, are dropped outside the lock, the last subscribed first. A winner whose subscription is
  not stored yet — one that wins as it is subscribed — finds its position still reserved in the
  fresh slots, and its subscription is stored there once made; the subscription of any other
  source made after the win is disposed at once. A loser subscribed before a win during
  registration is therefore disposed at the win. The winner forwards subsequent events directly
  without an operator lock. Amb used to subscribe a batch and store it whole once registration
  ended, which needed a `SubscriptionSet::take_all_but(key: usize)` of its own: an operation that
  rebuilt a tuple of subscriptions from a run-time position, which no other operator used, and
  that left a loser subscribed during registration running until registration ended. The slots
  express the same choice with the reservation and positional machinery merge with a limit
  already shares. The cursor stays in `subscribe`: unlike merge with a limit, nothing resumes the
  traversal later, so neither Amb's observers nor its disposal name it.
- `merge` is one state machine, its `Driver`, with a limit or without (see "One Merge" below).
  With a limit, `Merge<S, NonZeroUsize>`, it takes the remaining sources through
  `SubscribeEach`, subscribing at most `max_concurrency` of them at once. The collection
  becomes a `Cursor` — a tuple of `Option`s, or `Enumerate<I::IntoIter>` — and its subscriber
  subscribes each element at its static `TupleIndex` or dynamic `usize` position. The tuple
  cursor takes its sources per arity, at static positions, rather than dispatching on a run-time
  position in the source backend. The traversal returns `ControlFlow::Continue` when exhausted; breaking because the
  limit is full or the stream has stopped is private to merge. Both backends discover the end by
  exhausting traversal, with no last-element flag. For a list the subscriber is asked whether the
  stream is still running after `Iterator::next`, the only user code before a subscription.
  The subscriptions sit in `SourceSet::SubscriptionSlots` (`TupleSlots`, or
  `ListSlots` keyed by position), initialized through `Default`: one `ReservationSlot` per tuple
  source, reserved from the start, or an empty map for a list. Each source is subscribed at most once,
  so its slot is filled once it is subscribed and released as it completes. A source that completes
  as it is subscribed releases the reservation, and the fill then hands its subscription back. A
  list's map holds only the slots that are not idle, so that a list that never ends holds only
  those of the sources still running; dropped, the slots release them the last subscribed first.

  It counts its running sources under its lock, the place of the next one counted before it is
  pulled, and keeps the cursor `Held` by the one thread pulling, outside the lock, `Parked` in
  the model while there is no room, or `Exhausted`. The update that stores a subscription also
  counts the next place when there is room, so a pull takes one update per source; only when there
  is none does the driver update again, to park, checking for room once more, since a completion
  may have made some in between. A completion takes a parked cursor, counting the next place in
  the same update, and pulls on; one that comes while another thread holds it only frees the
  place, which that thread sees before it parks, so a source that completes as it is subscribed
  never recurses. Nothing is queued: a source is pulled only once there is room for it, which is
  what lets an iterator that never ends be merged with a limit. A panic while pulling — in
  `Iterator::next`, in a source's `subscribe`, or in the downstream observer a source emits to as
  it is subscribed — would leave a place counted for good, so the driver stops the context, as
  `subscribe_with_context` does when its builder panics; `MergeAll` does the same when the
  subscription of an inner observable panics. Releasing a completed subscription runs user code
  too: before continuing with a selected source, both operators check for a stop caused by its
  disposal, and their panic guards cover that disposal as well. The traversal is captured as a
  function pointer where `Merge` is subscribed, as `SubscribeFn` is: an observer that resumes it on
  completion would otherwise bound its own `Observer` impl on itself.

  The limit is a type parameter (`()`, no limit, by default) rather than a `NonZeroUsize::MAX` on
  one path, because the pulling form costs something in its types: its observers and its
  subscription name the cursor, so in the `Shared` mode the sources not pulled yet must be
  `Send`, and a value the iterator borrows must outlive the subscription. A merge with no limit
  subscribes everything at once and keeps none of that: the same driver, whose model names no
  cursor (see "One Merge" below). No limit is `()`, not a marker type of its own: such a marker
  would belong to the machinery `Merge` and `ConcatEager` share, which has no public path, so both
  would re-export it, one item under two paths.

`CombineLatest`, `Zip`, `ForkJoin`, `Merge` and `Amb` accept tuples through `new` and iterators
through `new_from_iter`. `WithLatestFrom::new_from_iter` emits `(primary_item, Vec<Item>)`; an
empty auxiliary list emits an empty vector on each primary event. There is no separate type for
the list form: a `CombineLatestAll` beside `CombineLatest::new_from_iter` would be a second name
for one operator, and RxJS's `combineLatestAll` takes an observable of observables, not a list.

The high-order `MergeAll`, `ConcatAll`, `Switch` and `ExhaustAll` retain their
own dynamic-source semantics, on the one machine of decision 0013, which shares nothing with a
source set. None of them has a list form.
`MergeAll::new_from_iter` merged a list as `Merge::new_from_iter` does, and then came back with a
`max_concurrency`, the one thing `Merge` did not do; it is gone again now that `Merge` takes one.
`ConcatAll::new_from_iter` went for the same reason: it is `Merge` with a `max_concurrency` of
one, which pulls the list where it pushed every observable of it into its queue at once, never
returning for an iterator that does not end.
`Switch::new_from_iter` and `ExhaustAll::new_from_iter` went last. What sets those two apart is
when an inner observable arrives, and the observables of a list all arrive as the operator is
subscribed: a list switched is its last observable, with whatever the ones before it emit as they
are subscribed, and a list exhausted is its first one that does not complete at once. Neither is
worth a constructor, and `Switch::new(FromIter::new(list).with_error_type())` still says it.
The split is by who decides when a source arrives: an observable of observables pushes them, so
`MergeAll` and `ConcatAll` must queue those they have no room for, and their queue has no bound;
a tuple or a list is pulled, so `Merge` takes the next source only once there is room for it.
A collection abstraction does not make a sequential operator or a join-pattern engine an operator
that subscribes all its sources at once.

## `Concat` is a tuple of pairs, or a `Merge` of one at a time

`Concat<S>` takes a tuple through `new` and a list through `new_from_iter`, as `Merge` does, and
`a.concat(b)` is `Concat<(Self, OE2)>`. It is not one more state machine over a source set:

- A tuple is chained into pairs, `(a, b, c)` into the pair of `a` and of the pair of `b` and `c`,
  by `ConcatSources`, a sealed trait over `SameItemSources` implemented once per arity. A pair
  subscribes its second source with the downstream observer itself, once the first has completed.
  Forwarding values takes no lock. A `ReplaceableDisposal` guards the second source's
  subscription. The observer of the source at `i` could not name the
  one at `i + 1` in a type — that is `{INDEX + 1}`, which needs the unstable
  `generic_const_exprs` — which is what the nesting is for: each pair only names its own second
  source.
- A list is `Merge::new_from_iter(list).max_concurrency(1)`, whose pull takes the next source
  only once the one before it has completed, without recursion however many of them complete as
  they are subscribed, and so concatenates an iterator that never ends.

Until then `Concat` had two sources only, and a tuple or a list of them was to be concatenated as
a `Merge` with a `max_concurrency` of one, which nobody looking for `concat` finds, and which
takes the lock of a merge for each value of sources that never run together.

## `ConcatEager` is a `Merge` whose sources tell their position

`ConcatEager<S, L>` takes a tuple through `new`, a list through `new_from_iter`, and a limit
through `max_concurrency`, as `Merge` does; `a.concat_eager(b)` is `ConcatEager<(Self, OE2)>`. It
is the source-set form of `ConcatEagerAll` (decision 0013), the one cell that was missing:
`MergeAll` has `Merge`, `ConcatAll` has `Concat`, while `Switch` and `ExhaustAll` have none, since
over sources known up front they would keep only the last one, or the first.

It is not one more state machine. It runs `Merge`'s `Driver` over its sources made
`IndexedSource`s, the wrapper `ConcatEagerAll` gives its inner observables too: `IndexSources` maps
a tuple to a tuple of `IndexedSource`s, each at its position, and an `IterSources` to one over an
iterator that wraps each source as it is pulled. An `IndexedSource` is subscribed with an
`IndexingObserver`, which sends `IndexedEvent::Next { position, value }` and, as the source
completes, `IndexedEvent::Completed { position }` before the completion that frees its place. The
sources tell their position, not the driver, which knows nothing of it, as `flatten` does not for
`ConcatEagerAll`: the driver once took a `SourceTag` type parameter that wrapped each source's
observer, an identity for `Merge`, and it was dropped for this, the one form both eager operators
share. The mapping changes only the type of the cursor, which with a limit waits in the model, so
the disposal of a `ConcatEager` with a limit names `IndexedCursorOf<S>`; the indexed sources'
error, mode and subscriptions are those of `S`. The merge is subscribed with `ReorderObserver`,
built around the `StopView` of the context, which emits the values of the first source that has
not completed and keeps the others by position. Positions run from 0 with no gap, and the driver
pulls the sources in order, so with a limit the first source not completed is always among those
running: with a `max_concurrency` of one it is `Concat`.

A tuple of sources of different types is concatenated eagerly without boxing, which
`ConcatEagerAll` over a `FromIter` of the sources could not do. With no limit an iterator is pulled
to its end as the stream is subscribed, as `Merge`'s is; one that never ends needs a limit.

## Where the shared pieces live

The `Driver` and what it needs — its model, `MergeObserver` and the
`subscribe_unlimited` / `subscribe_limited` entry points — are the crate-private
`combining/internal/merge.rs`, as the flattening machine is `combining/internal/flatten.rs`
(decision 0013): `merge.rs` and `concat_eager.rs` only choose a limit, their sources and an observer, and
name the disposal through a `delegate_disposal!` over `internal::merge::ContextDisposal`.
The sources and observers that put values back in order, `IndexedSource`, `IndexSources`,
`IndexedEvent`, `IndexingObserver` and `ReorderObserver`, are `combining/internal/reorder.rs`,
used by both machines: by `ConcatEagerAll` and `ConcatEagerMap` over `internal::flatten`, and by
`ConcatEager` over `internal::merge`. Both modules are private; the types a public signature names
stay `pub` and are re-exported by those operators.

They are kept to those operators on purpose. Every operator over a source set could in principle
be a merge of tagged sources under an observer that combines them, and none of the others is:

- `Concat` over a tuple hands the downstream observer from each pair to the next and forwards
  values without locking.
- `Amb` forwards the winner's events with no lock, and disposes the losers as it wins, while the
  driver holds every subscription until its source completes.
- `Zip`, `CombineLatest`, `ForkJoin` and `WithLatestFrom` hold items of different types at typed
  positions — the driver's context has one item type — need their position count before
  subscribing, and end on rules of their own; they share the batch instead.
- `SequenceEqual`'s two sources have one item type, so it would need no `Either`, but its
  two-step context already does the work, and its comparison may run under the lock (decision
  0008): tags and an end event per source would buy nothing.
- The operators whose two sources have roles (`take_until`, `skip_until`, `sample`, `buffer`,
  `window`) end with one of them, or stop one alone, which the driver cannot.
- An observable of observables pushes its sources: `internal::flatten`.

## Not made tuples

- **Sources with roles**: `sample`, `take_until`, `skip_until`, `buffer`, `window` (a source and a
  notifier — merge notifiers to have several), `join`, `group_join` (a left and a right),
  `sequence_equal`, and the fallbacks of `catch`, `retry`, `switch_if_empty`.

## Rejected

- **`CombineLatest3` … `CombineLatest6`**, as RxJava's overloads or `futures::join3`: a public type,
  a fluent method and a page of documentation per arity and operator, for what one generic type
  does.
- **A fluent method taking a source or a tuple** (`a.combine_latest((b, c))` emitting `(A, B, C)`):
  it needs an impl for every `ObservableTypes` and another for tuples, whose coherence depends on
  no tuple ever being an observable. The tuple form goes through `new`.
- **`Zip` built on `When`** with one plan: the same semantics, but the plans' machinery — the
  bits of the sources each plan needs, a selector per match — for nothing `zip` uses.
- **One homogeneous storage representation for both forms**: tuples retain heterogeneous state
  and tuple outputs; lists retain their runtime size and `Vec` outputs. Sharing a generic operator
  through associated storage types preserves both capabilities without boxing.

## Sources are collected with their position count

The collecting operators — `CombineLatest`, `Zip`, `ForkJoin`, `WithLatestFrom` — collect their
sources with `InternalSourceSet::into_collected_cursor()`, before subscribing anything, and get their position count
from the same call: a tuple's arity, or the length of the `Vec` a list adapter collected. The
operator then creates its value storage for that count — `LatestOps::empty(n)`, or Zip's `Model::empty(n)` — and Zip's count of empty queues
starts at it. The storage does not report its positions back: the count
`into_collected_cursor` returns is the only one there is, a tuple's being its arity, which a
debug assertion checks where a tuple's storage is created. An operator that needs the position
count before subscribing, for its completion count or its empty-input rule, takes the count
`into_collected_cursor` returned. The position count is not the number of sources subscribed, which may be
smaller when the operator stops during registration.

The count is returned with the collection rather than by a second method: only a collection has
one — a lazy `IterSources` does not know how many sources it will yield — so a separate
`position_count` would need a trait of its own for collected sets, and an operator could ask it
of the wrong value.

The collection comes as a cursor, `InternalSourceSet::CollectedCursor`: the `CollectedCursor<C>` wrapper around the
tuple's own cursor, or around an enumeration of the `Vec`, subscribed by `SubscribeBatch`. It was a source set first,
`SourceSet::Collected` — the tuple itself or an `IterSources` over the `Vec`, bound to select the
same slots — whose only use was to have its cursor prepared again; the operation, then named
`collect`, now returns that cursor, and is named `into_collected_cursor` after what it returns,
next to `into_cursor`: `collect` read as `Iterator::collect`, and said nothing of the cursor. A collected list stores its subscriptions in the list slots, rather than
reserving its known length: a cost paid once per subscription, against a second store.

`SourceSet` selects no value storage. Completion, empty-input behavior, error policy,
WithLatestFrom's primary/auxiliary ordering and Zip's completed-source queue budget remain with
their operators; Zip's queue model stays in `zip.rs`. Lazy batches, cursor traversal, positional
reservations and the pushed inner observables of decision 0013 do not join this seam. Collection
preserves static dispatch and the existing `Clone`, `Send` and lifetime bounds. The position
count comes from the tuple's arity or the vector's length in constant time. Collected iterator
subscriptions use the shared map of slots, and the list form of latest-value storage checks
readiness by scanning its positions before cloning.

Not chosen:

- **Storage accepting an arbitrary raw store** with an unchecked "every position is empty"
  precondition: it replaces one caller invariant with another.
- **Typed initialization** (`()` for a tuple, a length for a list): a separate item-shape
  interface, which still needs the list adapter to match its sources.
- **Collecting into a storage**, `collect_into::<St: PositionStorage<I>>()`, creating the storage
  inside the adapters: it was the design for a while (see History). Its trait only forwarded to
  `LatestOps::empty` and `QueueOps::empty`, and the count it was to keep from going astray is the
  only one an operator holds after collecting.
- **Encapsulating the whole context and subscription**: the model factories, empty handlers and
  primary-source continuations it needs make its interface larger than what it hides.

## One subscription path

Every operator subscribes its sources the same way: a cursor, from `InternalSourceSet::into_cursor` or
`into_collected_cursor`, then `SubscribeEach` with a `SourceSubscriber`, each subscription stored in `SubscriptionSlots` at the
position of its source. `When`, Amb and Merge bring a subscriber of their own; the
batch of the collecting operators — CombineLatest, Zip, ForkJoin, WithLatestFrom — is the shared
one, `sources::batch::Batch`, over the cursor `into_collected_cursor` returns. It subscribes each source with
the observer `ObserverFamily::at` gives for its
position, fills the slot, and breaks once `ObserverFamily::is_active` no longer holds:
checked once before the traversal and after each subscription. That is every point where the
stream can end on this thread: the cursor is collected, a tuple or the enumeration of a `Vec`,
so advancing it runs no user code. The batch used to check again before each source of
a list, as the subscribers of a cursor still pulling an iterator do (`still_running`, see
below); with a collected cursor that check followed the previous one with nothing in between,
and cost a list one more lock per source. The cursor is consumed by `subscribe_batch`: no observer or
disposal names it, so a batch gains no retained-cursor `Send` or lifetime bound.

That the cursor is collected is in its type, not a convention. `into_collected_cursor` returns
it in a `CollectedCursor<C>`, a newtype whose field only the `sources` module and its two
backends reach, and `SubscribeBatch` is implemented for that type alone. A cursor still pulling
an iterator, from `into_cursor`, has no `subscribe_batch`: with the check gone from the
batch, one over such a cursor would subscribe the source that `Iterator::next` returned after
ending the stream. The alternative was to keep the check for whatever cursor comes; it never
ran on the paths that exist, and no test could reach it. The type is crate-private: the one
associated type naming it, `InternalSourceSet::CollectedCursor`, is on a crate-private trait.

`SubscribeBatch<F, SL>` has the one impl, for a `CollectedCursor<C>` whose cursor takes the
batch as its subscriber (`C: for<'a> SubscribeEach<Batch<'a, F, _>, ()>`); the per-source bounds
sit on the batch's `SourceSubscriber` impl. It was `LazySubscribeSources`, named against a `CollectedSubscribeSources`
long gone; the collecting operators call it on a collection, which is not lazy.

A subscriber that takes both tuples and lists — the batch, Amb's, Merge's — has one
impl, over `K: Position`. Amb's and Merge's each used to have two, alike but for one check: the
`usize` one looked again, before subscribing, whether the stream was still running, since
`Iterator::next` had just run. The `Enumerate` impl of `SubscribeEach` now makes that check
for every subscriber, right after `Iterator::next` (`still_running`, see "Two levels" below); the
tuple impl never asks it, so a tuple pays nothing for it. The batch answers it without looking,
its cursor being collected: a collected list is an `Enumerate` too, over the `Vec`, and goes
through the same impl. `When`, which takes tuples only, keeps its impl over `TupleIndex`.

Batches used to be a second mechanism next to the cursor: `subscribe_batch` was implemented by
the tuple backend, per arity, and by the list backend with a loop of its own, into a second
store, `SourceSet::Subscriptions` — a tuple of `OptionDisposal`s, or `ListSubscriptions` over a
`Vec`. Traversal by position, the stop checks around `Iterator::next` and release in reverse
order on a panic were each written twice. The activity check was a closure argument, always
`|| context.is_active()` on the family itself; it is now the family's method, which also keeps
the batch's type free of a closure parameter the blanket bound could not name. For a while that
method sat in a supertrait of its own, `IsActive`, implemented once for every
`SubscriptionContext` so that no family repeated it; that made `utils::subscribe_with_context`,
a module for users writing operators, depend on `observable::sources` for a trait with one
implementor. Each of the four families now forwards to its context in one line.

Accepted costs: a tuple slot is a `ReservationSlot` with a state a batch never reaches
(`Released`), and a list's slots are a `BTreeMap` where a batch would do
with a `Vec` — paid at subscription, not per event. The slots module stays independent of
disposal: the `Disposable` impls are in the source backends, for slots of `DisposeOnDrop` values
only.

A batch returned its subscribed count with the slots, in a `SubscriptionBatch`, for the one
operator that read it: Merge with no limit, which was a batch then. It returns the guarded slots
alone since Merge left (below).

## Two levels

Subscribing the sources is layered, the batch built on the subscriber, and an operator names
only the higher one that fits it:

1. **One source at a time** — `SourceSubscriber` and `SubscribeEach`, in
   `observable::sources::subscribe`. `When`, Amb and Merge each write a subscriber.
2. **A batch** — `ObserverFamily` and `SubscribeBatch`, in `observable::sources::batch`: the
   subscriber of the collecting operators, which only say which observer a position gets.

The four subscribers that existed before the first level — the batch's, `When`'s, Amb's,
Merge's — each repeated one skeleton: look again after `Iterator::next`, make the
observer, subscribe, store the subscription, say whether to go on. A `SourceSubscriber<K, OE, B>`
is that skeleton with the operator's parts as methods — `still_running`, `observer`, `store` —
and the private `subscribe_one` runs the last three in order for one source. What stays
with each operator is what differs: where the subscription goes (the batch's own slots, the
model, the race state, an input), why it breaks (`()`, Merge's `Pause`, never), and whether the
cursor is resumed.

What it buys is not shorter operators — each still writes its three methods — but two
rules that used to be conventions: `still_running` is a required method, asked for every source
that came out of an iterator, so a subscriber of lists cannot forget the
check; and a subscription reaches the operator only as `store`'s argument, so it cannot be left
unstored. `When`, whose sources are a tuple, is never asked `still_running`, and is the only
subscriber whose `observer` answers `None`: a source no plan waits on any more is dropped
without being subscribed, and the traversal goes on.

The trait is parameterized by the position and the source type, not by
generic methods: `When` stores a subscription in an input whose type names the item of that one
source, a bound only an impl per position can carry.

`SubscribeEach` has two impls, one per cursor: the tuple of `Option`s, generated per arity,
whose where-clause names the subscriber's observer at each position, and `Enumerate<I>`. The
first is where the bounds of each source sit; the second is where `still_running` is asked,
next to the `Iterator::next` that makes it necessary. `SubscribeEach` has no `subscribe_all`:
`When`, the one caller of a complete pass, writes
`let ControlFlow::Continue(()) = cursor.subscribe_each(&mut context);`, an irrefutable pattern
since its break type is `Infallible`.

There used to be a third level below these, a traversal in `utils::positional::visit` that knew
positions and element types but not observables: `VisitElements`, implemented by both cursors,
handed each element to an `ElementVisitor`, and a private visitor, `Subscribing`, wrapped the
subscriber and ran its steps. Subscribing was its one use, so the traversal's independence from
observables bought nothing, while it cost a wrapper — a blanket `ElementVisitor` impl for every
subscriber would have made coherence reject the traversal's own test visitors — and a
`Position::DYNAMIC` constant, true for a `usize`, that let the visitor tell whether an element
had come out of an iterator. That was a property of the cursor, not of the position: a
collected list is enumerated at `usize` positions too, without running user code. With each
cursor implementing `SubscribeEach` itself, the check sits in the iterator's impl and the
constant is gone. The subscriber also had a fourth method, `admit`, asked before `observer` and
used only by `When`; its answer is now `observer`'s `Option`, so the order of the two is no
longer a rule to keep.

## One Merge

`Merge` and `Merge<S, NonZeroUsize>` are one state machine, the `Driver`: places counted under
the lock, each subscription in the slot of its source from its subscription to its completion,
the cursor `Held`, `Parked` or `Exhausted`. With no limit the model's limit is
`NonZeroUsize::MAX`, so there is always room and the cursor is never parked: the cursor the
driver traverses (`run`'s `X`) is then not the type the model can park (`C`), which is
`Infallible`. The sources stay with the call to `subscribe`, and neither the observers nor the
disposal name them — the reason the limit is a type parameter is kept, without a second
implementation. `park`, which turns the traversed cursor into the parked one, is the identity with
a limit and unreachable without.

Merge with no limit used to be a batch: every source subscribed at once into subscriptions held
whole until the stream ended, completions counted in a signed `pending` that went negative until
the batch's subscribed count was added. Its documentation told users with a long list of
short-lived sources to ask for `max_concurrency(NonZeroUsize::MAX)` instead, which released each
subscription as its source completed. Both forms do now. The cost is in subscribing and
completing, not in forwarding a value: the update that was a check of the stream now also stores
the subscription, a completion also releases it, and a list keeps them in a map.

A source that completes as it is subscribed is therefore disposed there, in subscription order,
rather than with the others as the stream ends, the last subscribed first
(`test_unsubscribe_order_on_synchronous_completion_panic`).

## History

Until the collection was given its storage, each collecting operator ran the construction itself:
`collect`, then `len` on the collected sources, then `LatestValues::new(len)` or
`QueueStore::empty(len)` with an empty-queue count of `len`, then `len == 0` for its empty input,
then `CollectedSubscribeSources::subscribe_collected_batch`, which preallocated a list's
subscriptions. Tuple storage ignored the length while readiness trusted it. `LatestStore`,
`QueueStore` and `PrependItem` carried their operations publicly (`empty`, `take_all`,
`pop_all`, `prepend`), with `LatestOf<S>` naming a latest-value storage nothing used.

SourceSet initially also selected a batch `Disposal`, and `SubscribeSources: CollectSources`
combined collected and lazy subscription. That made a batch resource part of metadata even for
cursor-based operators, and made immediate enumeration inherit complete collection. The final
strategy split moved resources to `BatchSubscriptionTypes::Subscriptions` and removed those legacy
projections and entry points. `SubscriptionSet::len` is gone too: batch results carry the actual
registration count. Normal constructors and output
shapes are unchanged. The latest-value and output-composition traits and aliases remain public.

The split then went further than the methods needed: `BatchSubscriptionTypes`, `CursorSources`
and `SubscriptionSlotTypes` each selected one resource type (and the cursor's `into_cursor`),
each as a trait of its own over `SourceSet`, so that `SourceSet` would describe metadata alone.
But every source set implemented all three, under the very bounds of its `SourceSet` impl, so
they constrained nothing an operator could not already rely on: they made three hidden public
traits, a re-export each and an impl each per arity and for lists, and put three traits in the
bounds of the limited merge. They were folded into `SourceSet` as hidden associated items.
What stays apart is what does constrain: `SameItemSources`, which a tuple of mixed items lacks,
and the crate-private subscription and collection traits, which carry observer bounds or name
the crate-private `PositionStorage`, which a public trait's method cannot.

`SourceSet::into_cursor` then remained the one operation on the public trait: hidden from the
documentation, yet callable by any user importing `SourceSet`. It moved, with collection, into
the crate-private `SourceOps`, which every source set implements on the same terms; the
operators preparing a cursor (Amb, Merge with a limit, When) bound it on their `Observable` impl.
`Collected` first moved with it, as the one type a public signature never names; but `Cursor` and
`Collected` are the same kind of thing, the form `into_cursor` and `collect` prepare the sources
in, and splitting them by whether a disposal happens to name one put the two return types in two
traits. `Collected` joined `Cursor` in `SourceSet`, so that the public trait holds every type and
`SourceOps` every operation.

That split by kind left `CollectedCursor<C>` a `pub`, `#[doc(hidden)]` type with a hidden
re-export from `observable`: the value of an associated type in a public trait's impl must be as
visible as the trait (E0446), so `SourceSet::CollectedCursor` kept the newtype public, and with
it, hidden but in the public API, the guarantee it carries, that advancing the cursor runs no
user code. `SourceOps` became `InternalSourceSet`, and `CollectedCursor` moved onto it: the two
traits are now split by visibility, which the compiler enforces, rather than by kind, which only
the docs did. A type that a public signature names (`Cursor`, `SubscriptionSlots`) must stay on
`SourceSet`, or the crate does not compile. The cost is the one the earlier move was undone for:
`Cursor` and `CollectedCursor` sit on two traits, and `Cursor`'s documentation says why. In
return the newtype is crate-private, the operators that subscribe a batch bound
`<S as InternalSourceSet>::CollectedCursor` next to the `S: InternalSourceSet` they already
wrote, and the trait's name says why it is apart, which `Ops` stopped doing once it held a type.

Collection first returned the operator's storage, `CollectSources::collect_into::<St>()` with a
crate-private `PositionStorage<I>` that `LatestValues` and Zip's model implemented, so that the
adapters alone matched storage to sources. Once the storages derived their counts from the
positions they have, the trait only forwarded to `LatestOps::empty` and `QueueOps::empty`, and
every caller named the storage by turbofish; `collect` returns the count instead.

Operators initially coordinated optional values and missing-value counts, or queues and
empty-queue counts, themselves. LatestValues updates latest-value storage and readiness
together. A CombineLatest author replaces a value and attempts a snapshot rather
than updating an optional slot, detecting its first value, decrementing a separate counter and
proving that raw storage can be cloned. Completion policy still belongs to the operator. ForkJoin uses the
same object without Clone and moves the result out once; this first slice established the
bounds and Local behavior before the other consumers migrated. The wrappers keep the existing
tuple/vector layouts, static dispatch and allocation paths, with no boxing or runtime erasure.

Zip's queue readiness was temporarily extracted into `QueuedValues`, then implemented through
generic collection mapping and through direct per-arity queue layouts with a function pointer.
It now uses the original raw-queue model again: the operator coordinates insertion, its
empty-queue count and complete removal through `QueueStore`. This keeps the queue algorithm in
`zip.rs` without a separate wrapper or mapping module. `QueueStore` and `QueuesOf` remained public;
constructors, output shapes and behavior are unchanged. Latest-value storage and output
composition retain their existing traits.

The type selections were then three public, sealed traits in `utils::positional::stores`:
`LatestStore::Storage`, `QueueStore::Storage` and `PrependItem::Output<T>`. Every one of them was
implemented for exactly the tuples of one to eight items and `Vec<T>`, with no bound of its own,
so, as with the resource traits folded into `SourceSet` above, the split constrained nothing: it
only made operators list `LatestStore + PrependItem` and every `ObservableTypes` impl repeat a
bound on `S::Item`. They became the one `Combination`, named after the glossary term for what they
are implemented on, with `Latest`, `Queues` and `After<T>`; `SourceSet::Item` is bound by it, so
the aliases and the operators' `ObservableTypes` impls need no bound on the items, and the module
is `combination`. `LatestValues`' `snapshot` and `finish` became `clone_combination` and
`take_combination`: the glossary avoids "snapshot" for a combination, and `finish` suggested a
terminal state that `LatestValues` does not have. `take_combination` was then removed: ForkJoin was
its one caller, and it reset the count of missing values so that the positions could fill again,
which no operator did — ForkJoin ends its stream as it takes the combination. ForkJoin now keeps
the raw storage and calls `LatestOps::take_all`.

`LatestValues` itself was removed next. Its count of missing values only cached what the storage
says, at the price of an invariant kept on every replacement, and it saved one pass over the
positions before the first combination — the pass every later value pays to clone one.
`CloneLatest::clone_all`, which panicked on a missing value, answers `None` instead, and
CombineLatest and WithLatestFrom keep the raw storage as ForkJoin does. `LatestOps::position_count`
and `QueueOps::position_count`, which reported a storage's positions back for those counts, went
with it: the count `into_collected_cursor` returns is the same number.

Until this decision, `CombineLatest`, `Zip`, `Merge` and `WithLatestFrom` were two-source types
(`CombineLatest<OE1, OE2>`, `new(a, b)`), and `ForkJoin` was the list form, now
`ForkJoin::new_from_iter`. `When` had its own copy of the per-arity machinery. `CombineLatestAll`,
`ZipAll`, `ForkJoinAll` and `AmbAll` were kept for a while as wrappers delegating to
`new_from_iter`, then removed before a release.

The shared machinery originally lived in `utils::source_tuple`, then `observable::source_tuple`.
It now lives in `observable::sources`; the original public tuple trait paths remain unchanged.
The original decision kept separate tuple and list algorithms. Associated storage types now let
those forms share their state machines while preserving their distinct outputs and subscription
rules.
