//! The crate-private machinery the combining operators are built on. None of these modules is an
//! operator: each is shared by several of them, those of [`transforming`](crate::operators::transforming)
//! included.

pub(super) mod merge;
