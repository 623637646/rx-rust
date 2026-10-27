//! Multicasting a cold source through a subject: `publish`, `replay`, `share` and manual
//! connection. See <https://reactivex.io/documentation/operators.html#connectable>.
//!
//! `share()` (`publish().ref_count()`) turns a cold observable into a shared, hot one: the source
//! is subscribed when the first observer arrives and unsubscribed when the last leaves.
//! `publish()` / `replay()` / `multicast()` give the
//! [`ConnectableController`](connectable_controller::ConnectableController) back instead, for a
//! manual `connect()` / `disconnect()`.
//!
//! The subject is created by a subject factory, a `fn(&mut C) -> S` called with a context `C`, and
//! a new one replaces it when it is reset, so each round starts over. By default the source
//! terminating, the last subscriber leaving and a manual `disconnect()` all reset it.
//! `reset_on_error(false)` / `reset_on_complete(false)` on the controller keep a terminated subject
//! for late subscribers, and `reset_on_ref_count_zero(false)` on the
//! [`RefCount`](ref_count::RefCount) keeps the source connected without subscribers:
//!
//! ```rust
//! use rx_rust::{
//!     observable::ObservableExt, operators::creating::from_iter::FromIter,
//!     subject::replay_subject::ReplaySubject,
//! };
//! use std::cell::RefCell;
//!
//! let seen = RefCell::new(Vec::new());
//! let cached = FromIter::new([1, 2, 3])
//!     .multicast(Some(1), |buffer_size| ReplaySubject::local(*buffer_size))
//!     .reset_on_complete(false)
//!     .ref_count();
//!
//! for _ in 0..2 {
//!     cached.clone().subscribe_with_callback(|value| seen.borrow_mut().push(value), |_| {});
//! }
//! // The second subscriber gets the kept `3`, and the source is not connected again.
//! assert_eq!(*seen.borrow(), [1, 2, 3, 3]);
//! ```

pub mod connectable_controller;
pub mod ref_count;
