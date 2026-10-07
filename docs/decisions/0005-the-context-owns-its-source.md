# 0005: The context owns its source

Who holds the source subscription of an operator built on `subscribe_with_context`: the context,
so that the source is disposed as soon as the context stops.

## One helper, owning the source

Status: adopted. `test_stop_on_next` in `observe_on`, `delay` and `debounce` specifies it.

`subscribe_with_context` puts the subscription its builder returns inside the context, and the
context drops it, outside the lock, once it stops — by a disposal, by a termination from any of its
sources, notifiers or tasks, or because downstream answered `Flow::Stop`. The source is then
disposed on the thread that stopped the context.

### Why

- **A `Flow::Stop` delivered by a scheduler task.** `observe_on`, `delay` and `debounce` deliver
  from a task. When downstream answers `Stop` there, the context stops, but the source is not in
  the call stack to hear it: it learns only from the `Stop` its next `on_next` gets back. A source
  that sends nothing more stayed subscribed until the subscription was dropped.
- **The wrong choice was asymmetric.** With two helpers, choosing the non-owning one where the
  context could stop on its own left the source running — a bug; choosing the owning one where it
  was not needed cost one more lock per subscription. With one helper there is nothing to choose.
- **One helper, one disposal type**, and one rule fewer for whoever writes an operator.

### Cost

- **A scheduler task now names the source's disposal**, since the task holds the context, which
  holds the source subscription. A scheduler that requires `Send + 'static` tasks requires it of
  `OE::Disposal` too, for `observe_on`, `delay`, `debounce` as already for `timeout`, `sample` and
  the time buffers. A source whose subscription borrows a local, or is not `Send`, cannot go through
  them; that is why these operators have no `test_lifetime_sub`. There is no way around it: to
  dispose the source from the task, the task has to reach it.
- **The source is disposed on the thread that stopped the context**, which for an operator that
  delivers from a scheduler is the scheduler's thread: after an `observe_on` has delivered the
  completion, its upstream is torn down on the observing thread, not on the one that drops the
  subscription (`observe_on::test_multiple_operation`).

## History

Until this decision there were two helpers. `subscribe_with_context` chained the source
subscription after the context's disposal, so that the source was disposed only with the returned
subscription; it was for an operator that could terminate only from inside its source's own
`on_termination`, where the source is finished anyway. `subscribe_with_context_owning_source` was
the helper above. `observe_on`, `delay` and `debounce` used the first: their source's disposal
stayed out of their tasks, so it could borrow, and was always disposed on the caller's thread.

Rejected, with that design: keeping both and documenting `Flow::Stop` as a reason to own the
source. Every operator that delivers from a task would then have needed the owning helper, which
left the other one with no user that the rule would not have to argue about.
