//! What an observable becomes as a `futures::Stream` of its items: [`ObservableTryStream`], behind
//! [`ObservableExt::into_try_stream`](crate::observable::ObservableExt::into_try_stream), and
//! [`ObservableStream`] for a source that cannot fail, behind
//! [`ObservableExt::into_stream`](crate::observable::ObservableExt::into_stream); and the
//! [`StreamBuffer`] that decides what either keeps between two polls.
//!
//! # Backpressure
//!
//! The crate has no `request(n)`: an observable pushes, and the only word its observer has is
//! [`Flow::Stop`]. Demand-driven consumption is what `Stream` is for, so the pull side lives there
//! rather than in a second observable type — the `Flowable` of RxJava — and the two meet at the
//! conversions.
//!
//! - From a stream: `FromStream` polls its stream only after `on_next` returns, so the stream's
//!   pace is kept through every synchronous operator, up to the first asynchronous one.
//! - From a stream, at the pace of an asynchronous handling:
//!   [`FromStreamAcked`](crate::operators::creating::from_stream_acked::FromStreamAcked) polls
//!   its stream only once the [`StreamAck`](crate::scheduler::StreamAck) sent with an earlier item
//!   is dropped, which [`hold`](crate::observable::ObservableExt::hold) delays until the
//!   observable that handles the item has ended. The demand is still the stream's, polled or not;
//!   the `StreamAck` only carries the consumer's "done" back to it.
//! - To a stream: `into_stream()` keeps every item that arrives between two polls, so a source
//!   faster than the consumer grows its buffer without bound.
//!   [`into_stream_with_buffer`](crate::observable::ObservableExt::into_stream_with_buffer) bounds
//!   it with a [`StreamBuffer`] that decides what survives: [`Latest`] keeps the newest item,
//!   [`Bounded`] a fixed number of them, dropping the oldest or the newest; an implementation of
//!   your own may fold the items into one.
//!
//! ```rust
//! use futures::{FutureExt, StreamExt};
//! use rx_rust::{
//!     observable::{ObservableExt, stream::Latest},
//!     observer::Observer,
//!     subject::publish_subject::PublishSubject,
//! };
//!
//! let mut subject = PublishSubject::local();
//! let mut stream = subject.clone().into_stream_with_buffer(Latest::new());
//! assert_eq!(stream.next().now_or_never(), None); // the first poll subscribes
//!
//! assert!(subject.on_next(1).is_continue());
//! assert!(subject.on_next(2).is_continue());
//! assert!(subject.on_next(3).is_continue());
//! assert_eq!(stream.next().now_or_never(), Some(Some(3)));
//! ```

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::{Shared, ThreadMode, mutable::MutableHelper},
    utils::lazy_subscription::LazySubscription,
};
use educe::Educe;
use futures::Stream;
use std::{
    collections::VecDeque,
    convert::Infallible,
    num::NonZeroUsize,
    task::{Poll, Waker},
};

#[derive(Educe)]
#[educe(Debug)]
struct ObservableTryStreamContext<E, B> {
    buffer: B,
    waker: Option<Waker>,
    termination: Option<Termination<E>>,
}

/// Converts a fallible observable into a `futures::Stream` of `Result`s that can be used with
/// `async/await` and `futures::TryStreamExt`.
///
/// Each item becomes `Ok(item)`. An error from the source becomes the last item, `Err(error)`,
/// after which the stream ends; completion ends the stream without an item. A source that cannot
/// fail goes through
/// [`ObservableStream`] instead.
///
/// Like any stream, it does nothing until it is polled: that first poll is what subscribes to the
/// source. Items are buffered until they are polled, so the stream never stops the source itself;
/// dropping the stream disposes the subscription instead. The buffer is a [`StreamBuffer`]:
/// [`Unbounded`] by default, which keeps everything, or the one handed to
/// [`with_buffer`](Self::with_buffer) — see there for what a source faster than the consumer
/// costs and how to bound it.
///
/// # Examples
/// ```rust
/// use futures::TryStreamExt;
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::creating::{from_iter::FromIter, throw::Throw},
/// };
///
/// futures::executor::block_on(async {
///     let source = FromIter::new([1, 2, 3]).with_error_type::<&str>();
///     let values: Result<Vec<_>, _> = source.into_try_stream().try_collect().await;
///     assert_eq!(values, Ok(vec![1, 2, 3]));
///
///     let source = FromIter::new([1, 2])
///         .with_error_type()
///         .concat_with(Throw::new("boom").with_item_type());
///     let values: Result<Vec<_>, _> = source.into_try_stream().try_collect().await;
///     assert_eq!(values, Err("boom"));
/// });
/// ```
///
/// # A source that never terminates
///
/// The stream ends only with a termination of the source, so after the last item of a source that
/// never terminates it stays pending forever. That is the case of a source that falls silent while
/// it holds its observer, and also of one that lets go of the observer without a termination, as
/// [`Never`](crate::operators::creating::never::Never) does at once: letting go says that nothing
/// more will come, but not whether the stream succeeded, so the stream does not end on the
/// source's behalf (see `docs/decisions/0004-a-source-that-drops-its-observer.md`).
///
/// To bound the wait, put a [`timeout`](crate::observable::ObservableExt::timeout) in front of it:
/// it fires once its duration passes without an item, on such a source too, and the stream then
/// yields `Err(timeout::Error::Timeout)` and ends. A runtime's own timeout around each poll, such
/// as `tokio::time::timeout`, works as well.
///
/// ```rust
/// use futures::{FutureExt, StreamExt};
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::never::Never, utility::timeout::Error},
///     scheduler::virtual_time::VirtualTime,
/// };
/// use std::time::Duration;
///
/// let time = VirtualTime::new();
/// let mut stream = Never
///     .with_item_type::<i32>()
///     .timeout(Duration::from_secs(1), time.scheduler())
///     .into_try_stream();
/// assert_eq!(stream.next().now_or_never(), None); // Subscribes; still pending.
///
/// time.advance_by(Duration::from_secs(1));
/// assert_eq!(stream.next().now_or_never(), Some(Some(Err(Error::Timeout))));
/// assert_eq!(stream.next().now_or_never(), Some(None));
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableTryStream<T, E, OE, B = Unbounded<T>>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    #[educe(Debug(ignore))]
    subscription: LazySubscription<OE, DisposeOnDrop<OE::Disposal>>,
    #[educe(Debug(ignore))]
    context: ContextPtr<E, B>,
}

/// The state the stream shares with its observer. It is always behind the thread-safe pointer,
/// whatever the source's mode, so neither the stream nor its observer is generic over one: every
/// synchronous source is `Local`, and many of them are `Send`, so a pointer picked from the mode
/// would make the stream over them `!Send` and keep it out of a multi-threaded executor. A source
/// that really is bound to its thread keeps the stream `!Send` by itself. On one thread the lock is
/// never contended, and the observer is the crate's own, so the mode has nothing else to decide.
type ContextPtr<E, B> = <Shared as ThreadMode>::Ptr<ObservableTryStreamContext<E, B>>;

impl<T, E, OE> ObservableTryStream<T, E, OE>
where
    OE: ObservableTypes<Item = T, Error = E>,
{
    /// Buffers every item until it is polled; see [`Unbounded`].
    pub fn new(source: OE) -> Self {
        Self::with_buffer(source, Unbounded::new())
    }
}

impl<T, E, OE, B> ObservableTryStream<T, E, OE, B>
where
    OE: ObservableTypes<Item = T, Error = E>,
    B: StreamBuffer<T>,
{
    /// Keeps the items that arrive between two polls in `buffer`, which decides what a source
    /// faster than the consumer costs: [`Unbounded`] keeps everything, [`Latest`] only the newest
    /// item and [`Bounded`] a fixed number of them.
    pub fn with_buffer(source: OE, buffer: B) -> Self {
        Self {
            subscription: LazySubscription::new(source),
            context: Shared::ptr(ObservableTryStreamContext {
                buffer,
                waker: None,
                termination: None,
            }),
        }
    }
}

impl<T, E, OE, B> Unpin for ObservableTryStream<T, E, OE, B> where
    OE: ObservableTypes<Item = T, Error = E>
{
}

impl<T, E, OE, B> Stream for ObservableTryStream<T, E, OE, B>
where
    OE: Observable<ObservableTryStreamObserver<E, B>, Item = T, Error = E>,
    B: StreamBuffer<T>,
{
    type Item = Result<B::Item, E>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        this.subscription
            .subscribe_once(|| ObservableTryStreamObserver {
                context: this.context.clone(),
            });

        // The waker this one replaces is handed back, because dropping a `Waker` runs the
        // external code of its vtable, which must not run under the lock.
        let (poll, previous_waker) = self.context.with_mut(|context| {
            let poll = if let Some(value) = context.buffer.pop() {
                Poll::Ready(Some(Ok(value)))
            } else {
                match context.termination.take() {
                    None => Poll::Pending,
                    Some(termination) => {
                        // An error is handed out once, as the last item; from then on the
                        // stream ends the way a completed one does.
                        context.termination = Some(Termination::Completed);
                        match termination {
                            Termination::Error(error) => Poll::Ready(Some(Err(error))),
                            Termination::Completed => Poll::Ready(None),
                        }
                    }
                }
            };
            let previous_waker = if poll.is_pending() {
                context.waker.replace(cx.waker().clone())
            } else {
                context.waker.take()
            };
            (poll, previous_waker)
        });
        drop(previous_waker); // Drop outside the lock to avoid potential deadlock
        if matches!(poll, Poll::Ready(None) | Poll::Ready(Some(Err(_)))) {
            // The stream is over, so the source is released now instead of whenever the stream
            // itself is dropped.
            self.subscription.release();
        }
        poll
    }
}

// Not generic over a mode: the context it shares with the stream is always `Shared`'s, see
// `ContextPtr`.
#[doc(hidden)]
pub struct ObservableTryStreamObserver<E, B> {
    context: ContextPtr<E, B>,
}

impl<T, E, B> Observer<T, E> for ObservableTryStreamObserver<E, B>
where
    B: StreamBuffer<T>,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The waker is taken under the lock and woken after it is released, because waking runs
        // external code, which must not run under the lock. The item the buffer evicts to make
        // room is dropped outside it for the same reason.
        let (evicted, waker) = self
            .context
            .with_mut(|context| (context.buffer.push(value), context.waker.take()));
        drop(evicted);
        if let Some(waker) = waker {
            waker.wake();
        }
        // The stream buffers whatever arrives, so it never stops the source itself: dropping the
        // stream disposes the subscription instead.
        Flow::Continue
    }

    fn on_termination(self, termination: Termination<E>) {
        // The waker is woken outside the lock, like in `on_next`. The termination this one
        // replaces is dropped outside it too; there is none unless the source breaks its
        // contract, since a terminated observer receives nothing more.
        let (waker, replaced) = self.context.with_mut(|context| {
            (
                context.waker.take(),
                context.termination.replace(termination),
            )
        });
        drop(replaced);
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Converts an observable that cannot fail into a `futures::Stream` that can be used with
/// `async/await`.
///
/// Each item is yielded as is and completion ends the stream. This is [`ObservableTryStream`]
/// without the error it can never carry; see there for how it subscribes and buffers.
///
/// A source that never completes keeps it pending after its last item, `Never` among them; see
/// [`ObservableTryStream`]. A [`timeout`](crate::observable::ObservableExt::timeout) in front
/// bounds the wait, and makes the source fallible, so the stream of that one is an
/// [`into_try_stream`](crate::observable::ObservableExt::into_try_stream).
///
/// # Examples
/// ```rust
/// use futures::StreamExt;
/// use rx_rust::{
///     observable::stream::ObservableStream, operators::creating::from_iter::FromIter,
/// };
///
/// futures::executor::block_on(async {
///     let source = FromIter::new([1, 2, 3]);
///     let mut stream = ObservableStream::new(source);
///     let values: Vec<_> = (&mut stream).collect().await;
///     assert_eq!(values, vec![1, 2, 3]);
/// });
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableStream<T, OE, B = Unbounded<T>>
where
    OE: ObservableTypes<Item = T, Error = Infallible>,
{
    stream: ObservableTryStream<T, Infallible, OE, B>,
}

impl<T, OE> ObservableStream<T, OE>
where
    OE: ObservableTypes<Item = T, Error = Infallible>,
{
    /// Buffers every item until it is polled; see [`Unbounded`].
    pub fn new(source: OE) -> Self {
        Self::with_buffer(source, Unbounded::new())
    }
}

impl<T, OE, B> ObservableStream<T, OE, B>
where
    OE: ObservableTypes<Item = T, Error = Infallible>,
    B: StreamBuffer<T>,
{
    /// Keeps the items that arrive between two polls in `buffer`; see
    /// [`ObservableTryStream::with_buffer`].
    pub fn with_buffer(source: OE, buffer: B) -> Self {
        Self {
            stream: ObservableTryStream::with_buffer(source, buffer),
        }
    }
}

impl<T, OE, B> Unpin for ObservableStream<T, OE, B> where
    OE: ObservableTypes<Item = T, Error = Infallible>
{
}

impl<T, OE, B> Stream for ObservableStream<T, OE, B>
where
    OE: Observable<ObservableTryStreamObserver<Infallible, B>, Item = T, Error = Infallible>,
    B: StreamBuffer<T>,
{
    type Item = B::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.stream)
            .poll_next(cx)
            .map(|item| {
                item.map(|result| {
                    let Ok(value) = result;
                    value
                })
            })
    }
}

/// Decides what
/// [`into_stream_with_buffer`](crate::observable::ObservableExt::into_stream_with_buffer) keeps
/// when the source pushes faster than the stream is polled.
///
/// An observable pushes at its own pace while a `Stream` hands out one item per poll, so the
/// items that arrive between two polls have to go somewhere. The buffer is where: every item
/// the source pushes goes through [`push`](Self::push), and every poll takes the next item out
/// with [`pop`](Self::pop). A `Stream` cannot slow its source down, so the buffer alone
/// decides what survives — and at what cost in memory — when the consumer falls behind.
///
/// Three buffers come with the crate: [`Unbounded`] keeps everything, [`Latest`] keeps the
/// newest item only, and [`Bounded`] keeps a fixed number of items and drops the oldest or the
/// newest beyond that. An implementation of your own can also fold the items that pile up into
/// one, which is why the item the stream yields ([`Item`](Self::Item)) need not be the item
/// the source pushes.
///
/// `push` and `pop` run while the adapter's state is locked. They must not re-enter the adapter;
/// return evicted values from `push` so that their destructors run outside that lock.
///
/// # Examples
/// A buffer that adds up the numbers that arrive between two polls:
/// ```rust
/// use futures::{FutureExt, StreamExt};
/// use rx_rust::{
///     observable::{ObservableExt, stream::StreamBuffer},
///     observer::Observer,
///     subject::publish_subject::PublishSubject,
/// };
///
/// #[derive(Default)]
/// struct Sum(Option<i32>);
///
/// impl StreamBuffer<i32> for Sum {
///     type Item = i32;
///
///     fn push(&mut self, item: i32) -> Option<i32> {
///         *self.0.get_or_insert(0) += item;
///         None
///     }
///
///     fn pop(&mut self) -> Option<i32> {
///         self.0.take()
///     }
/// }
///
/// let mut subject = PublishSubject::local();
/// let mut stream = subject.clone().into_stream_with_buffer(Sum::default());
/// assert_eq!(stream.next().now_or_never(), None); // subscribes
///
/// assert!(subject.on_next(1).is_continue());
/// assert!(subject.on_next(2).is_continue());
/// assert!(subject.on_next(3).is_continue());
/// assert_eq!(stream.next().now_or_never(), Some(Some(6)));
/// ```
pub trait StreamBuffer<T> {
    /// What the stream yields.
    type Item;

    /// Stores an item the source pushed.
    ///
    /// Returns the item that had to go to make room, if any, so that it is dropped outside the
    /// lock the buffer lives under.
    fn push(&mut self, item: T) -> Option<T>;

    /// Takes the next item to yield, or `None` when the stream has to wait for the source.
    fn pop(&mut self) -> Option<Self::Item>;
}

/// Keeps every item, in order. This is what
/// [`into_stream`](crate::observable::ObservableExt::into_stream) uses.
///
/// Nothing is ever dropped, so a source faster than the consumer grows the buffer without bound.
#[derive(Educe)]
#[educe(Debug, Default)]
pub struct Unbounded<T>(VecDeque<T>);

impl<T> Unbounded<T> {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self(VecDeque::new())
    }
}

impl<T> StreamBuffer<T> for Unbounded<T> {
    type Item = T;

    fn push(&mut self, item: T) -> Option<T> {
        self.0.push_back(item);
        None
    }

    fn pop(&mut self) -> Option<T> {
        self.0.pop_front()
    }
}

/// Keeps only the newest item: each item the source pushes replaces the one waiting to be
/// polled. This is `onBackpressureLatest` of other ReactiveX stacks.
///
/// The memory cost is one item whatever the pace of the source, at the price of skipping the
/// items the consumer was too slow to see — right for a stream of states, where only the
/// current one matters, and wrong for a stream of events.
#[derive(Educe)]
#[educe(Debug, Default)]
pub struct Latest<T>(Option<T>);

impl<T> Latest<T> {
    /// Creates an empty buffer.
    pub fn new() -> Self {
        Self(None)
    }
}

impl<T> StreamBuffer<T> for Latest<T> {
    type Item = T;

    fn push(&mut self, item: T) -> Option<T> {
        self.0.replace(item)
    }

    fn pop(&mut self) -> Option<T> {
        self.0.take()
    }
}

/// What [`Bounded`] does with an item that arrives when the buffer is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// The oldest waiting item makes room for the new one, so the buffer holds the newest
    /// items.
    DropOldest,
    /// The new item is dropped, so the buffer holds the oldest items.
    DropNewest,
}

/// Keeps at most `capacity` items, in order, and applies an [`Overflow`] rule beyond that. This
/// is `onBackpressureBuffer` with a capacity of other ReactiveX stacks.
///
/// The capacity is a [`NonZeroUsize`]: a buffer that holds nothing would yield nothing.
/// [`Latest`] is `Bounded::drop_oldest` with a capacity of one.
#[derive(Educe)]
#[educe(Debug)]
pub struct Bounded<T> {
    capacity: NonZeroUsize,
    overflow: Overflow,
    values: VecDeque<T>,
}

impl<T> Bounded<T> {
    /// Keeps at most `capacity` items and applies `overflow` beyond that.
    pub fn new(capacity: NonZeroUsize, overflow: Overflow) -> Self {
        Self {
            capacity,
            overflow,
            values: VecDeque::with_capacity(capacity.get()),
        }
    }

    /// Keeps the newest `capacity` items; see [`Overflow::DropOldest`].
    pub fn drop_oldest(capacity: NonZeroUsize) -> Self {
        Self::new(capacity, Overflow::DropOldest)
    }

    /// Keeps the oldest `capacity` items; see [`Overflow::DropNewest`].
    pub fn drop_newest(capacity: NonZeroUsize) -> Self {
        Self::new(capacity, Overflow::DropNewest)
    }
}

impl<T> StreamBuffer<T> for Bounded<T> {
    type Item = T;

    fn push(&mut self, item: T) -> Option<T> {
        if self.values.len() < self.capacity.get() {
            self.values.push_back(item);
            return None;
        }
        match self.overflow {
            Overflow::DropOldest => {
                let evicted = self.values.pop_front();
                self.values.push_back(item);
                evicted
            }
            Overflow::DropNewest => Some(item),
        }
    }

    fn pop(&mut self) -> Option<T> {
        self.values.pop_front()
    }
}
