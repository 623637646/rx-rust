//! Elements addressed by position — a tuple, whose positions are known at compile time and whose
//! elements differ in type, or a `Vec`, whose positions are known at run time — shared by sources,
//! combinations, disposals and join patterns.
//!
//! Positions retain each tuple element's type. A `Combination` selects its
//! latest-value and queue storage, and clones a combination out once every position has a value.
//! Reservation stores retain values by position and drop those left in reverse order. This layer
//! knows nothing about observables or subscriptions.

pub(crate) mod combination;
pub(crate) mod position;
pub(crate) mod slots;
pub(crate) mod tuple;
