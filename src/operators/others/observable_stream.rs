use crate::{
    observable::Observable, operators::others::observable_try_stream::ObservableTryStream,
    utils::types::MaybeSend,
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
pub struct ObservableStream<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    stream: ObservableTryStream<'or, T, Infallible, OE>,
}

impl<'or, T, OE> ObservableStream<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    pub fn new(source: OE) -> Self {
        Self {
            stream: ObservableTryStream::new(source),
        }
    }
}

impl<'or, T, OE> Unpin for ObservableStream<'or, T, OE> where OE: Observable<'or, T, Infallible> {}

impl<'or, T, OE> Stream for ObservableStream<'or, T, OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, Infallible>,
{
    type Item = T;

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
