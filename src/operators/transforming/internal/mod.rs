//! The crate-private machinery the transforming operators are built on. None of these modules is
//! an operator: each is shared by several of them.

pub(super) mod boundary;
pub(super) mod bundle;
pub(super) mod count;
pub(super) mod group;
pub(super) mod overlap;
pub(super) mod time_or_count;
pub(super) mod time_shift;
pub(super) mod timetable;
pub(super) mod window;
