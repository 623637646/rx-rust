//! The types that switch between the single-threaded and the multi-threaded build.
//!
//! Code that must compile in both goes through these — never `Rc` / `Arc` or `Send` / `Sync`
//! directly. The `single-threaded` feature picks the `Rc` side and drops the bounds.

use std::marker::PhantomData;

/// Placeholder for a type parameter that a struct carries but never stores.
///
/// `PhantomData<fn(T) -> T>` rather than `PhantomData<T>`, so the marker is `Send` / `Sync`
/// whatever `T` is, and invariant in `T`
/// (<https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>).
///
/// Operators carry one because their source's item type would otherwise be an unconstrained impl
/// parameter (E0207): in `impl Observable<'or, T, E> for Map<OE, F> where OE: Observable<'or, T0, E>`
/// the where-clause alone does not constrain `T0`. The marker still ties the struct's lifetime to
/// `T` — `MarkerType<&'a U>` is not `'static`, and no spelling of `PhantomData` avoids that — but
/// in practice the source observable already carries the same lifetime, so it costs nothing.
pub type MarkerType<T> = PhantomData<fn(T) -> T>;

cfg_if::cfg_if! {
    if #[cfg(feature = "single-threaded")] {
        /// The shared pointer: [`Rc`](std::rc::Rc) in the single-threaded build, `Arc` otherwise.
        pub type Shared<T> = std::rc::Rc<T>;
        /// The weak counterpart of [`Shared`].
        pub type WeakShared<T> = std::rc::Weak<T>;

        /// `Send` in the multi-threaded build, and no bound at all in the single-threaded one.
        pub trait MaybeSend {}
        impl<T> MaybeSend for T {}
        /// `Sync` in the multi-threaded build, and no bound at all in the single-threaded one.
        pub trait MaybeSync {}
        impl<T> MaybeSync for T {}
    } else {
        /// The shared pointer: [`Arc`](std::sync::Arc) in the multi-threaded build, `Rc` otherwise.
        pub type Shared<T> = std::sync::Arc<T>;
        /// The weak counterpart of [`Shared`].
        pub type WeakShared<T> = std::sync::Weak<T>;

        /// `Send` in the multi-threaded build, and no bound at all in the single-threaded one.
        pub trait MaybeSend: Send {}
        impl<T> MaybeSend for T where T: Send {}
        /// `Sync` in the multi-threaded build, and no bound at all in the single-threaded one.
        pub trait MaybeSync: Sync {}
        impl<T> MaybeSync for T where T: Sync {}
    }
}
