# Test checklist

The canonical list of test case names. Every operator, subject and source gets its own file under
`tests/` and covers the subset of cases below that applies to its shape, using these exact names —
do not invent new names for existing cases. See [AGENTS.md](../AGENTS.md) for how the tests are
built (`test_channel()`, `Checker`, and which source fits which case) and how to run them narrowly.

## Basic cases (every observable)

1. `test_completed`
2. `test_error`
3. `test_unsubscribe`
4. `test_ref`
5. `test_mut_ref`
6. `test_async`
7. `test_subscribe_by_different_observer`
8. `test_unsub_on_next_by_take`

## Flow

Downstream ends its own stream with `Flow::Stop`; the source must stop and must not call
`on_termination`.

1. `test_stop_on_next` — the observer itself stops, via `Checker::stopping_after`: it receives no
   further values, is not terminated, and is dropped.
2. `Stop` travelling back to the source through `take`: operators and subjects do not get a
   separate case. Instead, `test_unsub_on_next_by_take` asserts that the send which lets `take`
   complete answers `is_stop()` (operators that go through a scheduler answer `is_continue()`, since
   the value is delivered later). Only sources (`from_iter` etc., which have no sender to ask) get a
   separate `test_stop_on_next_by_take`, asserting that the source stopped producing: a synchronous
   source must stop, and an infinite iterator must not loop forever.

## Non-creating observable

1. `test_multiple_operation`
2. `test_without_convenient_api`

## Revertible observable

An operator that another one undoes — `materialize` / `dematerialize`, and `window` or `group_by`
followed by a flattening operator — checks that the pair gives the source back.

1. `test_revert_completed`
2. `test_revert_error`

## Hot observable

Subjects, `ConnectableController`, `RefCount` — anything that can borrow or own the sender while a
subscription is active.

1. `test_complete_on_next`
2. `test_error_on_next`
3. `test_unsub_on_next`
4. `test_sub_on_next`
5. `test_next_on_next`
6. `test_unsub_on_completed`
7. `test_sub_on_completed`
8. `test_unsub_on_error`
9. `test_sub_on_error`

## Using a lock (`with_mut` / `with_ref`)

1. `test_next_on_sub`
2. `test_complete_on_sub`
3. `test_error_on_sub`
4. `test_sub_on_sub` — another subscription happens during subscription. Only hot observables have
   shared state; cold operators get a fresh context per subscription, so this does not apply to
   them.
5. `test_next_on_unsub` — the event is emitted from the upstream's own disposal, i.e. during
   unsubscription. Pass-through operators forward it to the observer; context-based operators drop
   it.
6. `test_complete_on_unsub`
7. `test_error_on_unsub`
8. `test_sub_on_unsub` — same as 4, hot observables only.
9. `test_race_condition` (only a few operators have it so far)

## Using a scheduler (the operator or source takes a `Scheduler`)

Some scheduler-based operators have no `test_next_on_sub` (e.g. `from_future`).

1. All cases from "Using a lock"
2. `test_complete_after_next`
3. `test_error_after_next`
4. `test_unsub_after_next`
5. `test_unsub_after_completed`
6. `test_unsub_after_error`
7. `test_order_with_continuous_next`
8. `test_completed_no_delay` (if applicable: the operator takes an optional delay)
9. `test_abandon_after_next` — the source drops its observer without a termination
   (`sender.abandon()`) while the operator still has work of its own: queued or delayed values, a
   pending debounce, a timer. That work runs its course, and the observer is dropped, never
   terminated, once nothing more can come. See
   [decision 0004](decisions/0004-a-source-that-drops-its-observer.md).
10. `test_unsub_after_abandon` — the same, then the subscription is disposed before that work ran:
    the disposal cancels it, and nothing is delivered.

## Compile checks

1. `test_lifetime_sub`, `test_lifetime_or`, `test_lifetime_or_sub` — a value borrowed by the
   subscription, the observer, or both, outlives what borrows it. Each test declares the borrowed
   value first under `// OK`, and shows the reverse order, which must not compile, commented out
   under `// Error`.
2. `test_fn`
3. `test_clone`
4. `test_type_inference_with_subscribe`
5. `test_type_inference_without_subscribe`

A guarantee that some code must *not* compile is a `compile_fail` doctest on the item that makes it,
next to a twin that compiles and differs in one place; "not `Send`" is also asserted in
`tests/local_mode.rs`. See [decision 0003](decisions/0003-test-infrastructure.md).
