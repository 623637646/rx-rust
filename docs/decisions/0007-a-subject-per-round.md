# 0007: A subject per round

How long the subject of a `ConnectableController` / `RefCount` lives, and who decides when it is
replaced.

## A factory creates the subject, and the subject is reset after each round

Status: adopted. Specified by `tests/ref_count.rs` (`test_subscribe_after_completed`,
`test_subscribe_after_error`, `test_reset_on_*`, `test_sub_on_completed`, the two
`test_race_condition*`), `tests/publish.rs` (`test_disconnect_resets_the_subject`,
`test_reset_on_*`) and `tests/publish_behavior.rs` (`test_share_api_starts_over`).

The controller stores a context `C` and a subject factory `fn(&mut C) -> S`, and calls the factory
with the context for a new subject whenever the current one is needed and missing:
`multicast(context, subject_factory)`. The subject is reset, so that the next subscriber joins a
new one:

| When | Decided by | Default |
|---|---|---|
| The source fails | `ConnectableController::reset_on_error` | reset |
| The source completes | `ConnectableController::reset_on_complete` | reset |
| `disconnect()` | — | always reset |
| The last subscriber leaves a `RefCount` before the source terminated | `RefCount::reset_on_ref_count_zero` | reset and disconnect; `false` keeps the source connected |

A subject kept after its source terminated (`reset_on_*(false)`) is not reset when the last
subscriber of a `RefCount` leaves: it replays what it has, and the source is not connected again.
A manual `disconnect()` resets it all the same.

The subject is reset *before* the termination reaches it, so a subscriber that re-subscribes from
its termination (`repeat`) joins the next subject.

Each subject has a generation. A `RefCount` counts its subscribers per generation (a round): a
subscriber of an earlier round that leaves late does not count out one of the current round, and a
subscriber that arrives after the round's source terminated starts a new round and a new
connection even while the earlier subscribers still hold their subscriptions — a termination does
not dispose them in this crate. The `RefCount` takes its lock, then the controller's, never the
other way round, and resets the subject under both when its count reaches zero, so that a
subscriber cannot join a subject that is about to lose its source.

### Why

- **Nothing outlives its round by accident.** With one subject for the whole lifetime, a
  terminated source left the shared observable terminated for good — later subscribers got only the
  termination, while the source was subscribed again for nothing — and the current value of
  `share_behavior` or the buffer of `share_replay` leaked from one connection into the next.
- **It is what the other Rx libraries converged on.** RxJS 7 `share` / `connectable`, RxJava 3
  (`reset()` and `refCount`) and RxSwift's default `.whileConnected` create a subject per
  connection; Rx.NET, which keeps one, is the odd one out.
- **Each option sits where its event is seen.** The termination passes between the source and the
  subject, in the controller; the count is the `RefCount`'s. No separate `Share` operator is needed.
- **Keeping a subject is still possible, and explicit.** `reset_on_complete(false)` caches a result;
  a factory returning clones of a subject kept in the context keeps it forever.

## The factory is a context and a function pointer

Status: adopted.

What the factory needs — the initial value of `publish_behavior`, the buffer size of `replay` —
goes in the context, and the factory is a function pointer, a closure that captures nothing. The
`publish*` / `share*` methods pass `()` when there is nothing to keep. The context is the third
type parameter of `ConnectableController<OE, S, C = ()>` and `RefCount<OE, S, C = ()>`, so their
return types read as what they build: `RefCount<Self, PublishSubjectOf<'a, Self>>`,
`RefCount<Self, BehaviorSubjectOf<'a, Self>, Self::Item>`.

The factory is called under the controller's lock, and the `RefCount`'s too when one of its
subscribers needs a subject: decision 0008 exempts it from keeping user code out of a lock.

### Why

- **Every type is nameable, with nothing added.** No factory type of the crate's own appears in a
  signature: the subject and the context say it all.
- **It is the crate's way already.** `debug` keeps its context apart from a callback that is a
  function pointer in `debug_to_stdout`, and a scheduler `Task` is a context and a `fn` handler.

The cost is the context argument of every `multicast` call, `()` with a `|_|` factory when there is
nothing to keep, and a factory that cannot capture: what it would capture goes in the context.

## Rejected

- **A `Share` operator with all three options**, as RxJS 7's `share(config)`: duplicates
  `ConnectableController` + `RefCount`, which the options fit into.
- **A `Connector` trait with a blanket impl for `FnMut() -> S`**, as RxJS's nullary `connector`:
  `multicast(|| …)` reads better, but `share_behavior` and `share_replay` then need connector
  structs of their own (`BehaviorConnector`, `ReplayConnector`) to be nameable, one more trait and
  pattern than the crate has elsewhere, and return types that name a connector rather than the
  subject.
- **`F: FnMut() -> S` with no trait.** A closure's type cannot be named in the return types of
  `share_behavior` and `share_replay`; an `impl FnMut()` there works (return-position `impl Trait`
  in traits, `Send` included) but leaves those types unnameable.
- **Delayed reset** (RxJS's notifier-valued `reset_on_*`): not needed so far.
