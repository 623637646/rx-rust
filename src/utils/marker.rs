use std::marker::PhantomData;

/// Using `PhantomData<fn(T) -> T>` instead of `PhantomData<T>` to make MarkerType to be `Send + Sync` when T is not `Send + Sync`.
/// For more detail: https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
/// But the lifetime of MarkerType is affected by T. Which means T and MarkerType have the same lifetime.
/// TODO: find a better solution, so we can remove some restriction like `T: 'static` in some cases.
/// For more detail: https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505
pub(crate) type MarkerType<T> = PhantomData<fn(T) -> T>;
