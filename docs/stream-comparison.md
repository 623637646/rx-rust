# rx-rust and `Stream`

A `futures::Stream` and an rx-rust observable both describe values that arrive over time. They
differ in who drives them, and that decides what each one is good at. They also convert into each
other, so a program does not have to pick only one.

## Pull and push

A **`Stream` is pulled**. Its one consumer polls it for the next item, and nothing happens until
it does. A slow consumer slows the producer down, so backpressure comes built in. A stream lives
inside an async task, and its state is whatever that task holds between two `await`s.

An **observable is pushed**. The source calls its observer each time something happens, at the
source's own pace. A subject or `share()` delivers each value to every subscriber at once. There is
no backpressure: an observer takes what arrives, or answers `Flow::Stop`, or is unsubscribed. An
observable whose source is synchronous, such as a subject fed from a GUI callback, needs no async
runtime at all. The state of each step (the pending value of a `debounce`, the running inner
subscription of a `switch_map`) belongs to the operator, not to the code that consumes it.

## Side by side

The program of [`examples/search_as_you_type.rs`](../examples/search_as_you_type.rs) searches only
once the user has stopped typing for 300 ms, and a new query cancels the search still running. Both
versions below use the same `search`:

```rust
async fn search(query: &str) -> String {
    println!("searching {query:?}");
    sleep(Duration::from_secs(1)).await;
    format!("results for {query:?}")
}
```

With rx-rust, each requirement is one operator:

```rust
let _subscription = input
    .clone()
    .debounce(Duration::from_millis(300), scheduler)
    .switch_map(move |query| FromFuture::new(search(query), search_scheduler.clone()))
    .subscribe_with_callback(|results| println!("{results}"), |_| {});
```

With a `Stream` of keystrokes, the `debounce` becomes a timer that every keystroke resets, and the
`switch_map` becomes a slot for the running search, where storing a new future drops, and so
cancels, the old one. A `select!` loop drives the timer, the slot and the input together:

```rust
async fn search_as_you_type(mut input: impl Stream<Item = &'static str> + Unpin) {
    let debounce = sleep(Duration::ZERO);
    tokio::pin!(debounce);
    let mut pending = None; // the text typed since the last pause
    let mut running: Option<Pin<Box<dyn Future<Output = String> + Send>>> = None;
    loop {
        tokio::select! {
            text = input.next() => match text {
                Some(text) => {
                    pending = Some(text);
                    debounce.as_mut().reset(Instant::now() + Duration::from_millis(300));
                }
                None => break,
            },
            () = &mut debounce, if pending.is_some() => {
                // Replacing the future drops the old one, which cancels its search.
                running = Some(Box::pin(search(pending.take().unwrap())));
            }
            results = async { running.as_mut().unwrap().await }, if running.is_some() => {
                running = None;
                println!("{results}");
            }
        }
    }
}
```

Both print the same thing for the same keystrokes. The `Stream` version is correct, but its logic
does not compose: adding a `distinct_until_changed`, a `timeout` on each search or a `retry` means
more state in the same loop, while the rx-rust version gets one more line each. Neither `futures`
nor `tokio-stream` has a `debounce` or a `switch_map`. Some time operators come from other crates
(`futures-time` has a `debounce`), but switching to the latest inner sequence stays hand-written.

## Which one to use

Situation                                                                  | Use
-------------------------------------------------------------------------- | --------
The consumer sets the pace: a file, a socket, a database cursor            | `Stream`
I/O that ends in a framework taking a `Stream`, such as `axum` or `tonic`  | `Stream`
Events from a UI, a sensor or a message bus, read by several consumers     | rx-rust
Logic shaped by time: `debounce`, `throttle`, `sample`, `timeout`          | rx-rust
Several sources combined: `switch_map`, `combine_latest`, `zip`, `merge_with` | rx-rust
Synchronous code with no async runtime                                     | rx-rust

Both can be tested without waiting for real time. Tokio's paused clock (`start_paused`,
`tokio::time::advance`) does it for anything that sleeps on Tokio's timer. rx-rust's
`VirtualTime` does it for any pipeline, whatever runtime it runs on in production, synchronously
and without a runtime.

## Using both

The two meet at the conversions, so each part of a program can use the one that fits it:

- `FromStream` and `FromTryStream` turn a stream into an observable. They poll the stream only
  after `on_next` returns, so the stream's pace is kept up to the first asynchronous operator.
- `into_stream()` and `into_try_stream()` turn an observable into a stream. A source faster than
  the consumer then grows the buffer between two polls. `into_stream_with_buffer` bounds that
  buffer: `Latest` keeps the newest item, `Bounded` a fixed number of them.
- `FromFuture` and `FromTryFuture` turn a one-shot call into an observable, the shape that goes
  inside `switch_map` or `concat_map`. `into_future()` resolves with the first item.

See [Futures and streams](../README.md#futures-and-streams) in the README for examples, and the
`observable::stream` module documentation for the buffers.
