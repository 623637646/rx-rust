//! The [`delegate_disposal!`] macro.

/// Defines a named disposal type wrapping an inner one, to which it delegates
/// [`Disposable::dispose`].
///
/// An operator's `type D` is often a nesting such as `ChainDisposal<SharedDisposal<Subscription<D2>>, D1>`;
/// this macro puts a short, stable name in front of it. Use it for two or more nested wrapper
/// layers (`Outer<Inner<D>>`), and return a single layer (`Outer<D>`) as it is.
///
/// The generated type implements [`Disposable`] and `From<Inner>`, so
/// [`DisposableExt::into_subscription`] turns the inner value into a [`Subscription`] of it.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     delegate_disposal,
///     disposable::{callback_disposal::CallbackDisposal, option_disposal::OptionDisposal, Disposable, DisposableExt},
///     observable::Subscription,
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
/// let subscription: Subscription<MyDisposal<_>> =
///     OptionDisposal::some(CallbackDisposal::new(|| disposed = true)).into_subscription();
/// drop(subscription);
/// assert!(disposed);
/// ```
///
/// [`Disposable`]: crate::disposable::Disposable
/// [`Disposable::dispose`]: crate::disposable::Disposable::dispose
/// [`DisposableExt::into_subscription`]: crate::disposable::DisposableExt::into_subscription
/// [`Subscription`]: crate::observable::Subscription
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
        [$($meta:tt)*]
        [$name:ident]
        [$($generic:tt),+]
        [$inner:ty]
        [$($struct_where:tt)*]
        [$($where_clause:tt)*]
    ) => {
        #[doc = concat!("A disposal delegating to `", stringify!($inner), "`.")]
        $($meta)*
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
