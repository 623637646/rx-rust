//! The [`ObservableStream`] adapter, behind
//! [`ObservableExt::into_stream`](crate::observable::ObservableExt::into_stream),
//! [`ObservableExt::into_stream_with_buffer`](crate::observable::ObservableExt::into_stream_with_buffer).

use crate::thread_mode::Shared;
use crate::{
    observable::{Observable, ObservableTypes},
    operators::others::observable_try_stream::{
        ObservableTryStream, ObservableTryStreamObserver, StreamBuffer, Unbounded,
    },
};
use educe::Educe;
use futures::Stream;
use std::{convert::Infallible, task::Poll};

/// Converts an Observable that cannot fail into a `futures::Stream` that can be used with
/// `async/await`.
///
/// Each item is yielded as is and completion ends the stream. This is [`ObservableTryStream`]
/// without the error it can never carry; see there for how it subscribes and buffers.
///
/// # Examples
/// ```rust
/// use futures::StreamExt;
/// use rx_rust::{
///     operators::{
///         creating::from_iter::FromIter,
///         others::observable_stream::ObservableStream,
///     },
/// };
///
/// futures::executor::block_on(async {
///     let source = FromIter::new(vec![1, 2, 3]);
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
    OE: Observable<ObservableTryStreamObserver<Shared, Infallible, B>, Item = T, Error = Infallible>,
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
