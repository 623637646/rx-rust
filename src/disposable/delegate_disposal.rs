//! The [`delegate_disposal!`](crate::delegate_disposal) macro.

/// Defines a named disposal type wrapping an inner one, to which it delegates
/// [`Disposable::dispose`].
///
/// An operator's `type Disposal` is one of three:
///
/// - one passed through: the disposal of its source or scheduler (`OE::Disposal`,
///   `S::Disposal`), or that of the operator it is built on;
/// - the named disposal of a [`utils`](crate::utils) helper, used as it is, when its parameters
///   are only the operator's own (`subscribe_with_auto_dispose_on_termination::Disposal<OE::Mode,
///   OE::Disposal>`);
/// - a `Disposal` of the operator's own module.
///
/// Anything else — a combination such as `ChainDisposal<SharedDisposal<DisposeOnDrop<D2>>, D1>`,
/// or a type naming one of the operator's private types, such as its model — gets its `Disposal`
/// from this macro. The public `type Disposal` then shows neither how the disposal is built nor the
/// types inside it, which stay private: the field of the generated type is private, and an
/// associated type cannot name a private type itself.
///
/// The generated type is documented by the attributes given, or by a generic sentence without
/// them.
///
/// The generated type implements [`Disposable`] and `From<Inner>`, so
/// [`DisposableExt::into_dispose_on_drop`] converts the inner value to the named disposal and wraps
/// it in [`DisposeOnDrop`].
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     delegate_disposal,
///     disposable::{
///         Disposable, DisposableExt, callback_disposal::CallbackDisposal,
///         dispose_on_drop::DisposeOnDrop, option_disposal::OptionDisposal,
///     },
/// };
///
/// delegate_disposal!(
///     /// The disposal of my operator.
///     MyDisposal<F>,
///     OptionDisposal<CallbackDisposal<F>>,
///     where F: FnOnce()
/// );
///
/// let mut disposed = false;
/// let disposal: DisposeOnDrop<MyDisposal<_>> =
///     OptionDisposal::some(CallbackDisposal::new(|| disposed = true)).into_dispose_on_drop();
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
