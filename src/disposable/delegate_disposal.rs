/// Defines a named disposal type that delegates [`Disposable::dispose`] to an
/// inner disposal value.
///
/// This macro is specifically intended to put a short, stable name in front of
/// a disposal type with two or more nested type-wrapper layers. Count layers
/// along the deepest path through the type: `Outer<D>` has one layer, while
/// `Outer<Inner<D>>` has two. Use this macro for the latter and for deeper
/// compositions. Every instantiated concrete generic type counts as a layer;
/// for example, `OptionDisposal<ConcreteDisposal<'a, T>>` has two layers.
///
/// Do not use this macro for a disposal type with only one wrapper layer. Write
/// and return that concrete type directly instead.
///
/// The generated type implements [`Disposable`] by forwarding to the inner
/// value. It also implements conversion from the inner value to the generated
/// type. Use [`DisposableExt::into_subscription`] to convert the inner value
/// directly into a [`Subscription`] of the generated type.
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
