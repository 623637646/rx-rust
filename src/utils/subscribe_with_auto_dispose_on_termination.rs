//! The helper for an operator that ends the stream before its source does.
//!
//! `take`, `first`, `all` and friends terminate downstream on their own, and must then dispose
//! their source. [`subscribe_with_auto_dispose_on_termination`] wraps the downstream observer so
//! that terminating it — or its answering [`Flow::Stop`] — disposes the source subscription,
//! including when that happens synchronously while the source is still being subscribed to.
//!
//! An operator with shared state uses [`subscribe_with_context`](crate::utils::subscribe_with_context)
//! instead, whose owning form covers the same case.

use crate::{
    delegate_disposal,
    disposable::{Disposable, DisposableExt, shared_disposal::SharedDisposal},
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::on_panic::on_panic,
};
use educe::Educe;

delegate_disposal!(
    Disposal<D>,
    SharedDisposal<Subscription<D>>,
    where D: Disposable
);

/// Subscribes through `builder`, disposing the subscription it returns as soon as the observer
/// is terminated or stops.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{Observable, ObservableExt, Subscription},
///     observer::{Flow, Observer, Termination},
///     operators::creating::range::Range,
///     utils::subscribe_with_auto_dispose_on_termination::subscribe_with_auto_dispose_on_termination,
/// };
///
/// // An operator that completes after the first value: the source is disposed by the helper.
/// struct FirstObserver<OR>(Option<OR>);
///
/// impl<OR: Observer<i32, E>, E> Observer<i32, E> for FirstObserver<OR> {
///     fn on_next(&mut self, value: i32) -> Flow {
///         if let Some(mut observer) = self.0.take() {
///             if observer.on_next(value).is_continue() {
///                 observer.on_termination(Termination::Completed); // Disposes the source.
///             }
///         }
///         Flow::Stop
///     }
///     fn on_termination(self, termination: Termination<E>) {
///         if let Some(observer) = self.0 {
///             observer.on_termination(termination);
///         }
///     }
/// }
///
/// let mut seen = Vec::new();
/// let observer = rx_rust::observer::callback_observer::CallbackObserver::new(
///     |value| seen.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
/// let subscription = subscribe_with_auto_dispose_on_termination(observer, |observer| {
///     Range::new(1..).subscribe(FirstObserver(Some(observer)))
/// });
/// drop(subscription);
/// assert_eq!(seen, [1]);
/// ```
pub fn subscribe_with_auto_dispose_on_termination<OR, D, F>(
    observer: OR,
    builder: F,
) -> Subscription<Disposal<D>>
where
    D: Disposable,
    F: FnOnce(AutoDisposeOnTerminationObserver<OR, D>) -> Subscription<D>,
{
    let shared_disposal = SharedDisposal::default();
    let observer = AutoDisposeOnTerminationObserver {
        observer,
        shared_disposal: shared_disposal.clone(),
    };
    shared_disposal.replace(|| builder(observer));

    shared_disposal.into_subscription()
}

/// Whether `OR` is an [`AutoDisposeOnTerminationObserver`], whatever its generic arguments are.
///
/// Only the outermost type is recognized: an auto-disposing observer wrapped inside another
/// observer is not detected. This is a best-effort check meant for `debug_assert!`, not a complete
/// one.
pub(crate) fn is_auto_dispose_on_termination_observer<OR>() -> bool {
    fn type_name_without_generics<T>() -> &'static str {
        let name = std::any::type_name::<T>();
        name.split_once('<').map_or(name, |(name, _)| name)
    }

    type_name_without_generics::<OR>()
        == type_name_without_generics::<AutoDisposeOnTerminationObserver<(), ()>>()
}

/// The observer [`subscribe_with_auto_dispose_on_termination`] hands to its builder: the
/// downstream observer, plus the disposal of the source to run when it terminates or stops.
#[derive(Educe)]
#[educe(Debug)]
pub struct AutoDisposeOnTerminationObserver<OR, D: Disposable> {
    observer: OR,
    shared_disposal: SharedDisposal<Subscription<D>>,
}

impl<T, E, OR, D> Observer<T, E> for AutoDisposeOnTerminationObserver<OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        let flow = self.observer.on_next(value);
        if flow.is_stop() {
            // The observer ended its own stream, which is what a termination does too, so the
            // subscription is disposed here as well: the source is asked to stop by the flow this
            // returns, and released by the disposal whether or not it honors it.
            self.shared_disposal.clone().dispose();
        }
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        let Self {
            observer,
            shared_disposal,
        } = self;
        // Without the guard the source stays subscribed after the termination, until the
        // subscription `subscribe_with_auto_dispose_on_termination` returned is dropped. The
        // returning path disposes right after the callback, so the panicking path takes exactly
        // the locks the returning one would: nothing new can deadlock on the unwinding thread.
        let guard = on_panic(|| shared_disposal.clone().dispose());
        observer.on_termination(termination);
        drop(guard);
        shared_disposal.dispose();
    }
}
