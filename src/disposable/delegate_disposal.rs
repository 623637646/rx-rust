//! The [`delegate_disposal!`](crate::delegate_disposal) macro.

/// Defines a named disposal type wrapping an inner one, to which it delegates
/// [`Disposable::dispose`].
///
/// What decides an operator's `type Disposal` is whether another implementation would change it:
/// the type is part of the public API, so one that follows the implementation makes a rewrite a
/// breaking change. A type no implementation would change is written out:
///
/// - one passed through: the disposal of its source or scheduler (`OE::Disposal`,
///   `S::Disposal`), the user's, or `()` for a source with nothing to dispose;
/// - that of the operator this one is by definition — a special case of it or a fixed composition
///   with it (`first` is `element_at(0)`, `switch_map` is `map` then `switch`) — not merely one it
///   happens to be built on;
/// - a disposal that is what the observable is rather than how it is built: an
///   [`EitherObservable`](crate::observable::either_observable::EitherObservable) has an
///   [`EitherDisposal`](crate::disposable::either_disposal::EitherDisposal).
///
/// Anything else — a combination, however shallow, such as `Option<D>` or
/// `ChainDisposal<ReplaceableDisposal<M, D2>, D1>`, the named disposal of a
/// [`utils`](crate::utils) helper, which another helper would change, or a type naming one of the
/// operator's private types, such as its model — gets its `Disposal` from this macro, or from a
/// struct written by hand when disposing does more than delegate. The public `type Disposal` then
/// shows neither how the disposal is built nor the types inside it, which stay private: the field
/// of the generated type is private, and an associated type cannot name a private type itself. A
/// `pub type` alias would do neither: it is transparent, and cannot name a private type. Nor does a
/// delegate go around a type passed through, which would lose the type it shares with its source
/// and nest with the chain.
///
/// Its parameters are what the operator's types decide — the mode, the items, the disposals of its
/// sources and scheduler — and what is built of them goes inside, so that the public type shows
/// none of it. Its `where` clause holds only what the inner type needs to be well formed.
///
/// The generated type is documented by the attributes given, or by a generic sentence without
/// them.
///
/// The generated type implements [`Disposable`] and `From<Inner>`, so
/// [`DisposableExt::into_dispose_on_drop`] converts the inner value to the named disposal and wraps
/// it in [`DisposeOnDrop`]. It also implements [`Debug`](std::fmt::Debug) whenever the inner
/// disposal does, so that a [`DisposeOnDrop`] holding it can be printed.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     delegate_disposal,
///     disposable::{
///         Disposable, DisposableExt, callback_disposal::CallbackDisposal,
///         dispose_on_drop::DisposeOnDrop,
///     },
/// };
///
/// delegate_disposal!(
///     /// The disposal of my operator.
///     MyDisposal<F>,
///     Option<CallbackDisposal<F>>,
///     where F: FnOnce()
/// );
///
/// let mut disposed = false;
/// let disposal: DisposeOnDrop<MyDisposal<_>> =
///     Some(CallbackDisposal::new(|| disposed = true)).into_dispose_on_drop();
/// assert!(format!("{disposal:?}").contains("MyDisposal"));
///
/// drop(disposal);
/// assert!(disposed);
/// ```
///
/// [`Disposable`]: crate::disposable::Disposable
/// [`Disposable::dispose`]: crate::disposable::Disposable::dispose
/// [`DisposableExt::into_dispose_on_drop`]: crate::disposable::DisposableExt::into_dispose_on_drop
/// [`DisposeOnDrop`]: crate::disposable::dispose_on_drop::DisposeOnDrop
#[macro_export]
macro_rules! delegate_disposal {
    (
        $(#[$meta:meta])*
        $name:ident<$($generic:tt),+ $(,)?>,
        $inner:ty $(,)?
        where $($where_clause:tt)+
    ) => {
        $crate::delegate_disposal! {
            @impl
            [$(#[$meta])*]
            [$name]
            [$($generic),+]
            [$inner]
            [where $($where_clause)+]
            [, $($where_clause)+]
        }
    };

    (
        $(#[$meta:meta])*
        $name:ident<$($generic:tt),+ $(,)?>,
        $inner:ty
        $(,)?
    ) => {
        $crate::delegate_disposal! {
            @impl
            [$(#[$meta])*]
            [$name]
            [$($generic),+]
            [$inner]
            []
            []
        }
    };

    (
        @impl
        []
        $($rest:tt)*
    ) => {
        $crate::delegate_disposal! {
            @impl
            [#[doc = "The disposal of the subscriptions made by this module."]]
            $($rest)*
        }
    };

    (
        @impl
        [$($meta:tt)+]
        [$name:ident]
        [$($generic:tt),+]
        [$inner:ty]
        [$($struct_where:tt)*]
        [$($where_clause:tt)*]
    ) => {
        $($meta)+
        pub struct $name<$($generic),+>($inner) $($struct_where)*;

        impl<$($generic),+> $crate::disposable::Disposable for $name<$($generic),+>
        where
            $inner: $crate::disposable::Disposable
            $($where_clause)*
        {
            fn dispose(self) {
                self.0.dispose();
            }
        }

        impl<$($generic),+> ::core::fmt::Debug for $name<$($generic),+>
        where
            $inner: ::core::fmt::Debug
            $($where_clause)*
        {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                formatter
                    .debug_tuple(::core::stringify!($name))
                    .field(&self.0)
                    .finish()
            }
        }

        impl<$($generic),+> From<$inner>
            for $name<$($generic),+>
        $($struct_where)*
        {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };
}
