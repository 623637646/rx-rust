//! Runs a callback once the stream is over, whatever ended it.
//!
//! The [`DoFinally`] operator, behind
//! [`ObservableExt::do_finally`](crate::observable::ObservableExt::do_finally).

use crate::{
    disposable::{Disposable, callback_disposal::CallbackDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::{ThreadMode, mutable::MutableExt},
};
use educe::Educe;

/// Invokes a callback once the stream is over, whatever ended it: a termination, a disposal, or
/// the source letting go of the observer.
/// See <https://reactivex.io/documentation/operators/do.html> (`doFinally`).
///
/// The callback runs once, at the first of these:
///
/// - the source terminates: after downstream was notified;
/// - the subscription is disposed: after the source's subscription was;
/// - the source drops the observer without a termination: because downstream answered
///   [`Flow::Break`], say, or the source let go of it with nothing more to send.
///
/// [`do_after_termination`](crate::observable::ObservableExt::do_after_termination) covers the
/// first case alone, and
/// [`do_after_disposal`](crate::observable::ObservableExt::do_after_disposal) runs only once the
/// subscription is dropped, however long after the termination that is: this is the one place to
/// release a resource as soon as the stream no longer needs it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     operators::{creating::from_iter::FromIter, utility::do_finally::DoFinally},
/// };
/// use std::cell::RefCell;
///
/// let events = RefCell::new(Vec::new());
/// let subscription = DoFinally::new(FromIter::new([1, 2]), || {
///     events.borrow_mut().push("finalized")
/// })
/// .subscribe_with_callback(
///     |_| events.borrow_mut().push("next"),
///     |_| events.borrow_mut().push("completed"),
/// );
/// // The stream is over as soon as it completed, not once the subscription is dropped.
/// assert_eq!(*events.borrow(), ["next", "next", "completed", "finalized"]);
/// drop(subscription);
/// assert_eq!(events.borrow().len(), 4);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DoFinally<OE, F> {
    source: OE,
    callback: F,
}

impl<OE, F> DoFinally<OE, F> {
    /// Creates a [`DoFinally`] over `source`;
    /// [`ObservableExt::do_finally`](crate::observable::ObservableExt::do_finally) is the fluent form.
    pub fn new(source: OE, callback: F) -> Self
    where
        F: FnOnce(),
    {
        Self { source, callback }
    }
}

impl<T, E, OE, F> ObservableTypes for DoFinally<OE, F>
where
    OE: ObservableTypes<Item = T, Error = E>,
    F: FnOnce(),
{
    type Item = T;
    type Error = E;
    type Mode = OE::Mode;
    /// First the source's subscription, then the callback, unless the stream ended before.
    type Disposal = Disposal<OE::Mode, OE::Disposal, F>;
}

impl<T, E, OE, F, OR> Observable<OR> for DoFinally<OE, F>
where
    OR: Observer<T, E>,
    OE: Observable<DoFinallyObserver<<OE as ObservableTypes>::Mode, OR, F>, Item = T, Error = E>,
    F: FnOnce(),
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        let callback = OE::Mode::ptr(Some(self.callback));
        let observer = DoFinallyObserver {
            observer,
            callback: RunOnDrop(callback.clone()),
        };
        self.source
            .subscribe(observer)
            .map_inner(|source| Disposal { source, callback })
    }
}

/// The disposal of a [`DoFinally`] subscription: disposes the source, then runs the callback
/// unless the stream already ran it.
#[derive(Educe)]
#[educe(Debug)]
pub struct Disposal<M: ThreadMode, D, F> {
    source: D,
    #[educe(Debug(ignore))]
    callback: M::Ptr<Option<F>>,
}

impl<M: ThreadMode, D: Disposable, F: FnOnce()> Disposable for Disposal<M, D, F> {
    fn dispose(self) {
        // Claimed before the source's disposal can drop the observer and run it, and run after
        // that disposal, even if it panics.
        let _callback = self
            .callback
            .take_value()
            .map(|callback| DisposeOnDrop::new(CallbackDisposal::new(callback)));
        self.source.dispose();
    }
}

#[doc(hidden)]
pub struct DoFinallyObserver<M: ThreadMode, OR, F: FnOnce()> {
    // Fields drop in declaration order: downstream is released before the callback runs.
    observer: OR,
    callback: RunOnDrop<M, F>,
}

impl<M: ThreadMode, T, E, OR, F> Observer<T, E> for DoFinallyObserver<M, OR, F>
where
    OR: Observer<T, E>,
    F: FnOnce(),
{
    fn on_next(&mut self, value: T) -> Flow {
        self.observer.on_next(value)
    }

    fn on_termination(self, termination: Termination<E>) {
        let Self { observer, callback } = self;
        observer.on_termination(termination);
        drop(callback);
    }
}

/// Runs the callback as it is dropped, unless something ran it before.
struct RunOnDrop<M: ThreadMode, F: FnOnce()>(M::Ptr<Option<F>>);

impl<M: ThreadMode, F: FnOnce()> Drop for RunOnDrop<M, F> {
    fn drop(&mut self) {
        if let Some(callback) = self.0.take_value() {
            callback();
        }
    }
}
