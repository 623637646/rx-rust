use crate::{
    observable::{Observable, Subscription},
    observer::{Flow, Observer, Termination},
    utils::mutable::{Mutable, MutableHelper},
    utils::types::{MaybeSend, Shared},
};
use educe::Educe;
use futures::Stream;
use std::{
    collections::VecDeque,
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

/// Converts a fallible Observable into a `futures::Stream` of `Result`s that can be used with
/// `async/await` and `futures::TryStreamExt`.
///
/// Each item becomes `Ok(item)`. An error from the source becomes the last item, `Err(error)`,
/// after which the stream ends; completion ends the stream without an item. A source that cannot
/// fail goes through
/// [`ObservableStream`](crate::operators::others::observable_stream::ObservableStream) instead.
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
///     let source = FromIter::new(vec![1, 2, 3]).with_error_type::<&str>();
///     let values: Result<Vec<_>, _> = source.into_try_stream().try_collect().await;
///     assert_eq!(values, Ok(vec![1, 2, 3]));
///
///     let source = FromIter::new(vec![1, 2])
///         .with_error_type()
///         .concat_with(Throw::new("boom").with_item_type());
///     let values: Result<Vec<_>, _> = source.into_try_stream().try_collect().await;
///     assert_eq!(values, Err("boom"));
/// });
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableTryStream<'or, T, E, OE, B = Unbounded<T>>
where
    OE: Observable<'or, T, E>,
{
    source: Option<OE>,
    sub: Option<Subscription<OE::D>>,
    context: Shared<Mutable<ObservableTryStreamContext<E, B>>>,
}

impl<'or, T, E, OE> ObservableTryStream<'or, T, E, OE>
where
    OE: Observable<'or, T, E>,
{
    /// Buffers every item until it is polled; see [`Unbounded`].
    pub fn new(source: OE) -> Self {
        Self::with_buffer(source, Unbounded::new())
    }
}

impl<'or, T, E, OE, B> ObservableTryStream<'or, T, E, OE, B>
where
    OE: Observable<'or, T, E>,
    B: StreamBuffer<T>,
{
    /// Keeps the items that arrive between two polls in `buffer`, which decides what a source
    /// faster than the consumer costs: [`Unbounded`] keeps everything,
    /// [`Latest`] only the newest item and
    /// [`Bounded`] a fixed number of them.
    pub fn with_buffer(source: OE, buffer: B) -> Self {
        Self {
            source: Some(source),
            sub: None,
            context: Shared::new(Mutable::new(ObservableTryStreamContext {
                buffer,
                waker: None,
                termination: None,
            })),
        }
    }
}

impl<'or, T, E, OE, B> Unpin for ObservableTryStream<'or, T, E, OE, B> where
    OE: Observable<'or, T, E>
{
}

impl<'or, T, E, OE, B> Stream for ObservableTryStream<'or, T, E, OE, B>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    B: StreamBuffer<T> + MaybeSend + 'or,
{
    type Item = Result<B::Item, E>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        if let Some(source) = self.source.take() {
            let observer = ObservableTryStreamObserver {
                context: self.context.clone(),
            };
            let sub = source.subscribe(observer);
            self.sub = Some(sub);
        }

        let waker = cx.waker().clone();
        // The waker this one replaces is handed back, because dropping a `Waker` runs the
        // external code of its vtable, which must not run under the lock.
        let (poll, previous_waker) = self.context.with_mut(|context| {
            let previous_waker = context.waker.replace(waker);
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
            (poll, previous_waker)
        });
        drop(previous_waker); // Drop outside the lock to avoid potential deadlock
        if matches!(poll, Poll::Ready(None) | Poll::Ready(Some(Err(_)))) {
            // The stream is over, so the source is released now instead of whenever the stream
            // itself is dropped.
            self.sub = None;
        }
        poll
    }
}

struct ObservableTryStreamObserver<E, B> {
    context: Shared<Mutable<ObservableTryStreamContext<E, B>>>,
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

/// Decides what [`into_stream_with`](crate::observable::ObservableExt::into_stream_with) keeps
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
/// # Examples
/// A buffer that adds up the numbers that arrive between two polls:
/// ```rust
/// use futures::{FutureExt, StreamExt};
/// use rx_rust::{
///     observable::ObservableExt, observer::Observer,
///     operators::others::observable_try_stream::StreamBuffer,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
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
/// let mut subject = PublishSubject::<_, Infallible>::new();
/// let mut stream = subject.clone().into_stream_with(Sum::default());
/// assert_eq!(stream.next().now_or_never(), None); // subscribes
///
/// subject.on_next(1);
/// subject.on_next(2);
/// subject.on_next(3);
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

/// Keeps every item, in order. This is what [`into_stream`](crate::observable::ObservableExt::into_stream) uses.
///
/// Nothing is ever dropped, so a source faster than the consumer grows the buffer without bound.
#[derive(Educe)]
#[educe(Debug, Default)]
pub struct Unbounded<T>(VecDeque<T>);

impl<T> Unbounded<T> {
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
