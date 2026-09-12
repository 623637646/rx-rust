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
    task::{Poll, Waker},
};

#[derive(Educe)]
#[educe(Debug)]
struct ObservableTryStreamContext<T, E> {
    values: VecDeque<T>,
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
/// dropping the stream disposes the subscription instead.
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
pub struct ObservableTryStream<'or, T, E, OE>
where
    OE: Observable<'or, T, E>,
{
    source: Option<OE>,
    sub: Option<Subscription<OE::D>>,
    context: Shared<Mutable<ObservableTryStreamContext<T, E>>>,
}

impl<'or, T, E, OE> ObservableTryStream<'or, T, E, OE>
where
    OE: Observable<'or, T, E>,
{
    pub fn new(source: OE) -> Self {
        Self {
            source: Some(source),
            sub: None,
            context: Shared::new(Mutable::new(ObservableTryStreamContext {
                values: VecDeque::new(),
                waker: None,
                termination: None,
            })),
        }
    }
}

impl<'or, T, E, OE> Unpin for ObservableTryStream<'or, T, E, OE> where OE: Observable<'or, T, E> {}

impl<'or, T, E, OE> Stream for ObservableTryStream<'or, T, E, OE>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type Item = Result<T, E>;

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
            let poll = if let Some(value) = context.values.pop_front() {
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
        poll
    }
}

struct ObservableTryStreamObserver<T, E> {
    context: Shared<Mutable<ObservableTryStreamContext<T, E>>>,
}

impl<T, E> Observer<T, E> for ObservableTryStreamObserver<T, E> {
    fn on_next(&mut self, value: T) -> Flow {
        // The waker is taken under the lock and woken after it is released, because waking runs
        // external code, which must not run under the lock.
        let waker = self.context.with_mut(|context| {
            context.values.push_back(value);
            context.waker.take()
        });
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
