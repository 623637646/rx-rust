use std::marker::PhantomData;

/// Placeholder for a type parameter that a struct carries but never stores.
///
/// `PhantomData<fn(T) -> T>` is used instead of `PhantomData<T>` so the marker is `MaybeSend` /
/// `MaybeSync` even when `T` is not, and stays invariant in `T`.
/// For more detail: <https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns>
///
/// The marker still ties the struct's lifetime to `T`: `MarkerType<&'a U>` is not `'static`.
/// There is no way to avoid that. `PhantomData<X>: 'r` holds only if `X: 'r`, and the check is
/// structural over `X`, so every spelling that mentions `T` inherits `T`'s lifetime. The trait
/// object trick suggested in
/// <https://users.rust-lang.org/t/getting-phantomdata-to-have-a-static-lifetime/38505> does not
/// work either: `PhantomData<Box<dyn Fn(T) -> T + 'static>>` and
/// `PhantomData<*const (dyn Fn(T) -> T + 'static)>` are both rejected — a `dyn Trait + 'static`
/// object bound does not erase the lifetimes of the trait's own arguments.
///
/// The only real escape is to not name the type parameter on the struct at all. Operators carry it
/// because it would otherwise be an unconstrained impl parameter (E0207): in
/// `impl<'or, T0, T, E, OE, F> Observable<'or, T, E> for Map<OE, F> where OE: Observable<'or, T0, E>`
/// the source item type `T0` appears only in a where-clause trait bound, which does not constrain
/// it. Making `Observable` carry associated `Item` / `Err` types instead of generic parameters
/// would turn `T0` into the projection `OE::Item` and remove the marker from most adapters, the way
/// `std::iter::Map<I, F>` needs no marker. That is a crate-wide breaking change, it does not help
/// the operators whose parameter is chosen by the caller (`with_item_type::<T>`,
/// `with_error_type::<E>`, `collect::<C>`), and it makes things worse for `Create`, whose `T` / `E`
/// sit in the *input* type of its builder closure, which never constrains anything.
///
/// In practice the leak costs little: nearly every `T: 'static` bound in this crate comes from a
/// scheduler owning a value across a task, not from this marker, and where a marker does add a
/// lifetime the source observable already carries the same lifetime anyway.
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
