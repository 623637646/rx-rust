use std::marker::PhantomData;

/// Using `PhantomData<fn(T) -> T>` instead of `PhantomData<T>` to make MarkerType to be `MaybeSend` when T is not `MaybeSend`.
/// For more detail: <https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>
/// But the lifetime of MarkerType is affected by T. Which means T and MarkerType have the same lifetime.
/// TODO: find a better solution, so we can remove some restriction like `T: 'static` in some cases.
/// For more detail: <https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505>
pub type MarkerType<T> = PhantomData<fn(T) -> T>;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        pub type Shared<T> = std::rc::Rc<T>;
        pub type WeakShared<T> = std::rc::Weak<T>;

        pub trait MaybeSend {}
        impl<T> MaybeSend for T {}
        pub trait MaybeSync {}
        impl<T> MaybeSync for T {}
    } else {
        pub type Shared<T> = std::sync::Arc<T>;
        pub type WeakShared<T> = std::sync::Weak<T>;

        pub trait MaybeSend: Send {}
        impl<T> MaybeSend for T where T: Send {}
        pub trait MaybeSync: Sync {}
        impl<T> MaybeSync for T where T: Sync {}
    }
}
