//! Observables whose concrete type is erased: the explicit boundaries where a chain stops being
//! statically dispatched.
//!
//! The types differ along three independent choices:
//!
//! - **Observer.** [`BoxedObservable`] accepts any observer and boxes it on subscription, so every
//!   event is a dynamic call. [`BoxedObservableFor`] names the observer type `OR`: only the
//!   subscription is dynamic, the events are not.
//! - **Cloning.** [`BoxedObservable`] subscribes its source once and accepts anything.
//!   [`CloneableBoxedObservable`] keeps the source behind a shared pointer and clones it on every
//!   subscription, so it can be cloned itself but requires the source to be `Clone`.
//! - **Threads.** The `Send` flavors keep the ability to move to another thread and so require
//!   their source, disposal and observers to be `Send` (the cloneable one also `Sync`, since its
//!   clones share the source).
//!
//! Which `Send` flavor to use is a choice about storage, independent of the observable's
//! [`Mode`](crate::observable::ObservableTypes::Mode): a `Local` source can be boxed into a `Send`
//! box and moved to another thread before it is subscribed, and erasing never changes the mode.

use super::{Observable, ObservableTypes, Observer};
use crate::utils::MarkerType;
use crate::{
    disposable::boxed_disposal::{BoxedDisposal, SendBoxedDisposal},
    observable::Subscription,
    observer::boxed_observer::{BoxedObserver, SendBoxedObserver},
    thread_mode::ThreadMode,
};
use educe::Educe;
use std::{marker::PhantomData, rc::Rc, sync::Arc};

/// Generates an erased observable over a fixed observer type (`$observable_for`) and the newtype
/// over it that accepts any observer (`$observable`).
///
/// The storage decides the rest: `FnOnce` subscribes the source it owns once, `Fn` clones the
/// source on every subscription and makes both types `Clone`. `source` lists the bounds on the
/// boxed observable besides its `Observable` impls; a trailing `Send` also requires the disposal
/// and the observers to be `Send`.
macro_rules! boxed_observable {
    (@subscribe FnOnce, $source:ident) => {
        $source
    };
    (@subscribe Fn, $source:ident) => {
        $source.clone()
    };

    (@derive FnOnce, $item:item) => {
        #[derive(Educe)]
        #[educe(Debug)]
        $item
    };
    (@derive Fn, $item:item) => {
        #[derive(Educe)]
        #[educe(Debug, Clone(bound()))]
        $item
    };

    (
        $pointer:ident<dyn $fn_trait:ident $(+ $marker:ident)*>,
        source: [$($source_bound:ident),*],
        $(#[$meta:meta])*
        $observable:ident,
        $(#[$for_meta:meta])*
        $observable_for:ident,
        $observer:ident,
        $disposal:ident
        $(, $send:ident)?
    ) => {
        boxed_observable!(@derive $fn_trait,
            $(#[$for_meta])*
            pub struct $observable_for<'sub, 'oe, T, E, M, OR> {
                #[educe(Debug(ignore))]
                subscribe: $pointer<
                    dyn $fn_trait(OR) -> Subscription<$disposal<'sub>> $(+ $marker)* + 'oe,
                >,
                #[educe(Debug(ignore))]
                _marker: MarkerType<(T, E, M)>,
            }
        );

        impl<'sub, 'oe, T, E, M, OR> $observable_for<'sub, 'oe, T, E, M, OR> {
            /// Boxes `observable`, which keeps its own thread mode.
            pub fn new<OE>(observable: OE) -> Self
            where
                OR: Observer<T, E>,
                OE: Observable<OR> + ObservableTypes<Item = T, Error = E, Mode = M>
                    $(+ $source_bound)* + 'oe,
                OE::D: 'sub $(+ $send)?,
            {
                Self {
                    subscribe: $pointer::new(move |observer| {
                        let source = boxed_observable!(@subscribe $fn_trait, observable);
                        source.subscribe(observer).map_inner($disposal::new)
                    }),
                    _marker: PhantomData,
                }
            }
        }

        impl<'sub, T, E, M: ThreadMode, OR> ObservableTypes for $observable_for<'sub, '_, T, E, M, OR> {
            type Item = T;
            type Error = E;
            type Mode = M;
            type D = $disposal<'sub>;
        }

        impl<T, E, M: ThreadMode, OR> Observable<OR> for $observable_for<'_, '_, T, E, M, OR>
        where
            OR: Observer<T, E>,
        {
            #[inline]
            fn subscribe(self, observer: OR) -> Subscription<Self::D> {
                (self.subscribe)(observer)
            }
        }

        boxed_observable!(@derive $fn_trait,
            $(#[$meta])*
            pub struct $observable<'or, 'sub, 'oe, T, E, M>(
                #[educe(Debug(ignore))] $observable_for<'sub, 'oe, T, E, M, $observer<'or, T, E>>,
            );
        );

        impl<'or, 'sub, 'oe, T, E, M> $observable<'or, 'sub, 'oe, T, E, M> {
            /// Boxes `observable`, which keeps its own thread mode.
            pub fn new<OE>(observable: OE) -> Self
            where
                OE: Observable<$observer<'or, T, E>>
                    + ObservableTypes<Item = T, Error = E, Mode = M>
                    $(+ $source_bound)* + 'oe,
                OE::D: 'sub $(+ $send)?,
            {
                Self($observable_for::new(observable))
            }
        }

        impl<'sub, T, E, M: ThreadMode> ObservableTypes for $observable<'_, 'sub, '_, T, E, M> {
            type Item = T;
            type Error = E;
            type Mode = M;
            type D = $disposal<'sub>;
        }

        impl<'or, T, E, M: ThreadMode, OR> Observable<OR> for $observable<'or, '_, '_, T, E, M>
        where
            OR: Observer<T, E> $(+ $send)? + 'or,
        {
            #[inline]
            fn subscribe(self, observer: OR) -> Subscription<Self::D> {
                self.0.subscribe($observer::new(observer))
            }
        }

    };
}

boxed_observable!(
    Box<dyn FnOnce>,
    source: [],
    /// An observable whose concrete type is erased.
    ///
    /// Two observables of different types can then be stored together, or returned from either
    /// branch of an `if`. Every observer is boxed into a [`BoxedObserver`] when it subscribes, so
    /// each event costs a dynamic call; [`BoxedObservableFor`] avoids that when the observer's
    /// type is known. It is not `Send`; [`SendBoxedObservable`] is. It cannot be cloned;
    /// [`CloneableBoxedObservable`] can.
    ///
    /// The lifetimes bound the observer (`'or`), the disposal (`'sub`) and the observable itself
    /// (`'oe`); `'static` for all three is the common case.
    ///
    /// # Examples
    /// ```rust
    /// use rx_rust::{
    ///     observable::{boxed_observable::BoxedObservable, ObservableExt},
    ///     operators::creating::{from_iter::FromIter, just::Just},
    ///     thread_mode::Local,
    /// };
    /// use std::cell::RefCell;
    ///
    /// let seen = RefCell::new(Vec::new());
    /// let observables: Vec<BoxedObservable<'_, 'static, 'static, i32, _, Local>> = vec![
    ///     Just::new(1).into_boxed(),
    ///     FromIter::new([2, 3]).into_boxed(),
    /// ];
    /// for observable in observables {
    ///     observable.subscribe_with_callback(|value| seen.borrow_mut().push(value), |_| {});
    /// }
    /// assert_eq!(*seen.borrow(), [1, 2, 3]);
    /// ```
    BoxedObservable,
    /// An observable whose concrete type is erased, subscribed to by observers of the type `OR`.
    ///
    /// Only the subscription goes through a dynamic call: the source gets the concrete `OR`, so
    /// every event after it is statically dispatched. The price is that `OR` is part of the type.
    /// Use [`BoxedObservable`] to accept any observer.
    BoxedObservableFor,
    BoxedObserver,
    BoxedDisposal
);

boxed_observable!(
    Box<dyn FnOnce + Send>,
    source: [Send],
    /// A [`BoxedObservable`] that is `Send`, and so are its disposal and observers.
    SendBoxedObservable,
    /// A [`BoxedObservableFor`] that is `Send`, and so is its disposal. The observer need not be:
    /// what the source requires of it is already stated by `Observable<OR>`.
    SendBoxedObservableFor,
    SendBoxedObserver,
    SendBoxedDisposal,
    Send
);

boxed_observable!(
    Rc<dyn Fn>,
    source: [Clone],
    /// A [`BoxedObservable`] that can be cloned.
    ///
    /// The observable is kept behind a shared pointer and cloned on every subscription, so the
    /// wrapped type must be `Clone`. This is what an observable of observables holds when the inner
    /// ones must be erased, since `PublishSubject` and friends require `T: Clone`. It is not
    /// `Send`; [`SendCloneableBoxedObservable`] is.
    ///
    /// # Examples
    /// ```rust
    /// use rx_rust::{observable::ObservableExt, operators::creating::just::Just};
    /// use std::cell::RefCell;
    ///
    /// let seen = RefCell::new(Vec::new());
    /// let observable = Just::new(1).into_cloneable_boxed();
    /// let copy = observable.clone();
    ///
    /// observable.subscribe_with_callback(|value| seen.borrow_mut().push(value), |_| {});
    /// copy.subscribe_with_callback(|value| seen.borrow_mut().push(value), |_| {});
    /// assert_eq!(*seen.borrow(), [1, 1]);
    /// ```
    CloneableBoxedObservable,
    /// A [`BoxedObservableFor`] that can be cloned: the [`CloneableBoxedObservable`] of a fixed
    /// observer type `OR`.
    CloneableBoxedObservableFor,
    BoxedObserver,
    BoxedDisposal
);

boxed_observable!(
    Arc<dyn Fn + Send + Sync>,
    source: [Clone, Send, Sync],
    /// A [`CloneableBoxedObservable`] that is `Send` and `Sync`; its disposal and observers are
    /// `Send`.
    SendCloneableBoxedObservable,
    /// A [`CloneableBoxedObservableFor`] that is `Send` and `Sync`, and whose disposal is `Send`.
    SendCloneableBoxedObservableFor,
    SendBoxedObserver,
    SendBoxedDisposal,
    Send
);
