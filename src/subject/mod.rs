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
//! | [`unicast_subject`](unicast_subject::unicast_subject) | A single-consumer pipe that buffers what is sent before the subscription. |
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
//! let subject = PublishSubject::<i32, std::convert::Infallible>::new();
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
pub mod subject_observable;
pub mod unicast_subject;

use crate::{
    observable::Observable,
    observer::{Observer, Termination},
    subject::subject_observable::SubjectObservable,
};

/// A bridge that is both an [`Observer`] and an [`Observable`]. See the [module documentation](self)
/// and <https://reactivex.io/documentation/subject.html>.
pub trait Subject<'or, T, E>: Observable<'or, T, E> + Observer<T, E> {
    /// The termination this subject has received, if it has received one.
    fn terminated(&self) -> Option<Termination<E>>
    where
        E: Clone;
}

/// Helpers available on every [`Subject`].
pub trait SubjectExt<'or, T, E>: Sized {
    /// Wraps the subject so that only its [`Observable`] side is exposed.
    fn into_observable(self) -> SubjectObservable<Self> {
        SubjectObservable::new(self)
    }
}

impl<'or, T, E, S> SubjectExt<'or, T, E> for S where S: Subject<'or, T, E> {}
