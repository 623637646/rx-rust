//! Hot sources: a subject is an [`Observer`] and an [`Observable`] at once.
//!
//! Push into the observer end, subscribe to the observable end. Subjects are `Clone`, so one
//! clone is kept for sending after another has been subscribed. They differ in what a late
//! subscriber receives:
//!
//! | Subject | Late subscribers get |
//! |---|---|
//! | [`PublishSubject`](publish_subject::PublishSubject) | Only what is emitted after they subscribe. |
//! | [`BehaviorSubject`](behavior_subject::BehaviorSubject) | The latest value first, then everything after. |
//! | [`ReplaySubject`](replay_subject::ReplaySubject) | The buffered values, then everything after; the termination is replayed too. |
//! | [`AsyncSubject`](async_subject::AsyncSubject) | The last value, delivered on completion; nothing on error. |
//! | [`unicast_subject`] | A single-consumer pipe that buffers what is sent before the subscription. |
//!
//! Every subject terminates at most once, and reports it through [`Subject::terminated`].
//!
//! # Examples
//! ```rust
//! use rx_rust::{
//!     observable::ObservableExt,
//!     observer::{Observer, Termination},
//!     subject::publish_subject::PublishSubject,
//! };
//!
//! let mut seen = Vec::new();
//! let subject = PublishSubject::<i32, std::convert::Infallible, _>::local();
//! let mut sender = subject.clone();
//! let subscription = subject.subscribe_with_callback(|value| seen.push(value), |_| {});
//!
//! let _ = sender.on_next(1);
//! let _ = sender.on_next(2);
//! sender.on_termination(Termination::Completed);
//!
//! drop(subscription);
//! assert_eq!(seen, [1, 2]);
//! ```

pub mod async_subject;
pub mod behavior_subject;
pub mod publish_subject;
pub mod replay_subject;
pub mod unicast_subject;

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::{Observer, Termination},
};
use educe::Educe;

/// A bridge that is both an [`Observer`] and an [`Observable`]. See the [module
/// documentation](self) and <https://reactivex.io/documentation/subject.html>.
pub trait Subject<T, E>: ObservableTypes<Item = T, Error = E> + Observer<T, E> {
    /// The termination this subject has received, if it has received one.
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone;
}

/// Helpers available on every [`Subject`].
pub trait SubjectExt<T, E>: Sized {
    /// Wraps the subject so that only its [`Observable`] side is exposed.
    fn into_observable(self) -> SubjectObservable<Self> {
        SubjectObservable::new(self)
    }
}

impl<T, E, S> SubjectExt<T, E> for S where S: Subject<T, E> {}

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
/// let mut sender = PublishSubject::<i32, std::convert::Infallible, rx_rust::thread_mode::Local>::local();
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
    /// Wraps `subject`; [`SubjectExt::into_observable`] is the fluent form.
    pub fn new(subject: S) -> Self {
        Self(subject)
    }
}

impl<S> ObservableTypes for SubjectObservable<S>
where
    S: ObservableTypes,
{
    type Item = S::Item;
    type Error = S::Error;
    type Mode = S::Mode;
    type Disposal = S::Disposal;
}

impl<S, OR> Observable<OR> for SubjectObservable<S>
where
    OR: Observer<S::Item, S::Error>,
    S: Observable<OR>,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        self.0.subscribe(observer)
    }
}
