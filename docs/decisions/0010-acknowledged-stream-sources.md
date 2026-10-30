# 0010: Acknowledged stream sources

How a pipeline handles a source one value at a time, asynchronously, without reading ahead: a
large file read chunk by chunk, each chunk uploaded before the next is read.

## A stream source that waits for an acknowledgement, and `hold`

Status: adopted.

`FromStreamAcked` and `FromTryStreamAcked` convert a `Stream` like `FromStream` and
`FromTryStream`, but send each item with a `StreamAck`, and poll the stream only while they hold
a permit: one by default, `with_permits(n)` for `n`. Every item sent takes a permit; dropping its
`StreamAck` gives it back and wakes the source's task. `hold(resource)`, an operator of its own,
keeps a value for as long as the subscription to its source lasts; holding the `StreamAck` in the
observable made of the item acknowledges the item once that observable has ended:

```rust
FromTryStreamAcked::new(chunks, scheduler.clone())
    .concat_map(move |(chunk, ack)| FromTryFuture::new(upload(chunk), scheduler.clone()).hold(ack))
```

### Why

- **An observer cannot say "later".** `on_next` is synchronous and answers a `Flow`: `Continue` or
  `Stop`. `FromStream` polls the stream again once `on_next` returns, which keeps the stream's pace
  only up to the first asynchronous operator: a `concat_map` over an asynchronous inner returns at
  once, queues the value, and the stream is read to its end. The `StreamAck` is the "done" an
  `on_next` cannot give, carried with the item to wherever the work ends.
- **Dropping is acknowledging.** A `StreamAck` that must be called would stall the stream whenever
  one is forgotten, and every operator that drops an item (`filter`, `skip`, `debounce`, `sample`)
  would have to know about it. With the drop, those operators acknowledge on their own, and so
  does a disposal, a panic or an inner that lets go of its observer.
- **`hold` works with every flattening operator.** `concat_map` handles one item at a time;
  `flat_map` under `with_permits(n)` handles up to `n` at once, the bounded concurrency nothing
  else gives; `switch_map` drops the `StreamAck` of the inner it cancels. And it is not specific to
  acknowledgements: a lock guard, a temporary file or a metering token is tied to a subscription
  the same way. It is ReactiveX's `Using` without the factories, since a resource is an owned
  value, dropped with the observer that holds it.
- **It keeps the stance on `request(n)`.** The README says an observable has no `request(n)` and
  that demand-driven consumption is what `Stream` is for. That still holds: no observer, subject
  or operator gains a demand protocol, and the pull side stays in the `Stream`. What is new is a
  bridge from that pull side into a pipeline: the source polls only as the consumer finishes, and
  the consumer's "done" is a drop, not a request. The README, `observable::stream` and
  `docs/stream-comparison.md` say so where they state the stance.
- **It is opt-in and local.** No trait changes, no operator learns a protocol: the backpressure
  lives between one source and the place its `StreamAck`s are dropped. The rest of the pipeline is
  pushed, as before.
- **The task is the scheduler's.** It is `Task::from_stream_acked_then`, in
  `scheduler/task/stream_acked.rs` next to `from_stream_then`, and public like it, for operators
  written outside the crate; `StreamAck` lives there with it, since the task hands it out, and
  `from_stream_acked` re-exports it. The permits and the waker sit behind one lock of the
  scheduler's mode, which the `StreamAck` shares: in the `Shared` mode it is `Send` and may be
  dropped on any thread, in the `Local` mode it is not.
- **The names follow their neighbours.** `from_stream_acked_then` and `StreamAckedThenContext`
  pair with `from_stream_then` and `StreamThenContext`, the modifier after the noun as in
  `FromStreamAcked`. `StreamAck` rather than `Ack`, since its home is `scheduler`, where a bare
  `Ack` does not say what is acknowledged. "Permit" is the source's budget, "ack" the handle that
  gives one back: two things, two words.

### Its limits

- **A `StreamAck` that is not held is dropped as the closure returns.** Nothing fails: the stream
  is then read as fast as a `FromStream`, and the backpressure is silently gone. `#[must_use]`
  cannot catch it, since the value is bound (`|(chunk, _ack)|`); the documentation of
  `FromStreamAcked` leads with the `hold` pattern, and a test pins the read-ahead without it.
- **An operator that holds items until more arrive holds their `StreamAck`s.**
  `buffer_with_count(n)`, `window_with_count`, `collect`, `reduce`, `last`, `take_last`,
  `skip_last`, `zip`, a `ReplaySubject`: with fewer permits than the items such an operator waits
  for, it waits for the source while the source waits for it. `with_permits` gives the permits
  before the items flow, which is the only point where that can be undone.
- **The end of the stream is seen only when the stream is polled**, once a permit is back: the
  source completes, or fails, after the last `StreamAck` is dropped, not when the stream ends.
- **An inner that lets go of its observer** without a termination drops the `StreamAck` it holds,
  so the next value is read; `concat_map` still waits for that inner, so that value waits behind
  it.

### Rejected

- **A method of the acknowledgement** (`ack.drop_with_observable(observable)`). It would give a
  type of the scheduler layer a method returning an observable, the layers the other way round;
  it would hold only a `StreamAck`; and it names how the value ends rather than what holds it. An
  `ObservableExt` method follows the crate's fluent API.
- **A `request(n)` on the subscription, or a `StreamAck` that carries the next `n`.** The `n` a
  `StreamAck` could give comes too late to undo the stall above, since the `StreamAck` stuck in a
  buffer never reaches the code that would give it; it raises whether `n` adds up or replaces the
  window; and a `request(0)` with no `StreamAck` left out never resumes. A window that adapts can
  be added to `StreamAck` later, as a method, without breaking anything; nothing needs it yet.
- **A cloneable `StreamAck`.** Several owners make it unclear which one is done; a reference count
  would make a `ReplaySubject` keep the stream paused forever.
- **"Permit" throughout** (`StreamPermit`, as tokio's `SemaphorePermit`, which is also released by
  a drop). Accurate for the handle, but the sources would become `FromStreamPermitted`, or keep
  "acked" and give one thing two names.
- **A name for the effect** (`FromStreamPaced`). It hides the handle the user holds, and when it is
  dropped is what the stall above depends on.
- **A fused `FromStreamConcatMap`, the `StreamAck` kept inside.** It is
  `stream.map_ok(|v| process(v).into_try_stream()).try_flatten()` under another name, so it adds
  nothing the conversions do not give.
- **A feedback loop through a `PublishSubject`** (a "next" request sent when a value is done) for
  each use. It works, and needs no new type, but every user rewrites it, and the order of
  subscribing and sending the first request is easy to get wrong.

### History

The first version had `concat_map_acked(f)`: a `concat_map` over `(value, ack)` pairs that called
`f` with the value and held the `StreamAck` in a hidden wrapper of the inner. It could not be
misused, since no `StreamAck` reached the user, but it worked with `concat_map` only, and took a
public operator type and three hidden ones. `hold` replaced it: the same wrapper, public and
generic, so that every flattening operator gets it, at the cost of a `StreamAck` that can be
forgotten (see "Its limits").
