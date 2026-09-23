//! The observable side of a subject, with the observer side hidden.

use super::Subject;
use crate::observable::Subscription;
use crate::utils::types::MaybeSend;
use crate::{observable::Observable, observer::Observer};
use educe::Educe;

/// A subject exposed as an [`Observable`] only, so that a consumer cannot push into it.
///
/// [`ConnectableController::observable`](crate::operators::connectable::connectable_controller::ConnectableController::observable)
/// hands one out.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     subject::{publish_subject::PublishSubject, SubjectExt},
/// };
///
/// let mut seen = Vec::new();
/// let mut sender = PublishSubject::<i32, std::convert::Infallible>::new();
/// let observable = sender.clone().into_observable(); // `on_next` is not available on it.
///
/// let subscription = observable.subscribe_with_callback(|value| seen.push(value), |_| {});
/// let _ = sender.on_next(1);
/// drop((subscription, sender));
/// assert_eq!(seen, [1]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SubjectObservable<S>(S);

impl<S> SubjectObservable<S> {
    /// Wraps `subject`; [`SubjectExt::into_observable`](super::SubjectExt::into_observable) is
    /// the fluent form.
    pub fn new(subject: S) -> Self {
        Self(subject)
    }
}

impl<'or, T, E, S> Observable<'or, T, E> for SubjectObservable<S>
where
    S: Subject<'or, T, E>,
{
    type D = S::D;

    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<Self::D> {
        self.0.subscribe(observer)
    }
}
