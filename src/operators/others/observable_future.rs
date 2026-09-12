use crate::{
    observable::Observable, operators::others::observable_try_future::ObservableTryFuture,
    utils::types::MaybeSend,
};
use educe::Educe;
use std::{convert::Infallible, task::Poll};

/// A `Future` of the first item of an Observable that cannot fail.
///
/// It resolves with `Some(item)` on the first item, stopping the source right there, and with
/// `None` when the source completes without one. This is
/// [`ObservableTryFuture`] without the error it can never carry; see there for how it subscribes
/// and when it releases the source.
///
/// # Examples
/// ```rust
/// # #[cfg(not(feature = "tokio-scheduler"))]
/// # fn main() {}
/// # #[cfg(feature = "tokio-scheduler")]
/// #[tokio::main]
/// async fn main() {
///     use rx_rust::{
///         observable::ObservableExt,
///         operators::creating::{empty::Empty, from_iter::FromIter},
///     };
///
///     let first = FromIter::new([10, 20, 30]).into_future().await;
///     assert_eq!(first, Some(10));
///
///     let none = Empty.with_item_type::<i32>().into_future().await;
///     assert_eq!(none, None);
/// }
/// ```
#[derive(Educe)]
#[educe(Debug)]
pub struct ObservableFuture<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    future: ObservableTryFuture<'or, T, Infallible, OE>,
}

impl<'or, T, OE> ObservableFuture<'or, T, OE>
where
    OE: Observable<'or, T, Infallible>,
{
    pub fn new(source: OE) -> Self {
        Self {
            future: ObservableTryFuture::new(source),
        }
    }
}

impl<'or, T, OE> Unpin for ObservableFuture<'or, T, OE> where OE: Observable<'or, T, Infallible> {}

impl<'or, T, OE> Future for ObservableFuture<'or, T, OE>
where
    T: MaybeSend + 'or,
    OE: Observable<'or, T, Infallible>,
{
    type Output = Option<T>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        std::pin::Pin::new(&mut self.future).poll(cx).map(|result| {
            let Ok(value) = result;
            value
        })
    }
}
