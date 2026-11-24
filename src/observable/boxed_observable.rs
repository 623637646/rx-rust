//! Observables whose concrete type is erased: the explicit boundaries where a chain stops being
//! statically dispatched.
//!
//! The types differ along three independent choices:
//!
//! - **Observer.** [`BoxedObservable`] accepts any observer and boxes it on subscription, so every
//!   event is a dynamic call. [`BoxedObservableFor`] names the observer type `OR`: only the
//!   subscription is dynamic, the events are not.
//! - **Cloning.** [`BoxedObservable`] subscribes its source once and accepts any source.
//!   [`CloneableBoxedObservable`] keeps the source behind a shared pointer and clones it on every
//!   subscription, so it can be cloned itself but requires the source to be `Clone`.
//! - **Threads.** The `Send` flavors keep the ability to move to another thread and so require
//!   their source, disposal and observers to be `Send` (the cloneable one also `Sync`, since its
//!   clones share the source).
//!
//! [`into_boxed`](crate::observable::ObservableExt::into_boxed) and
//! [`into_cloneable_boxed`](crate::observable::ObservableExt::into_cloneable_boxed) pick the
//! `Send` flavor from the observable's [`Mode`](crate::observable::ObservableTypes::Mode), the
//! way [`ObserverMode`] picks the boxed observer: the plain box for [`Local`], the `Send` one for
//! [`Shared`]. A box of the other flavor is a choice about storage, and its constructor makes it:
//! [`SendBoxedObservable::new`] moves a `Local` source that is `Send` to another thread before it
//! is subscribed, and [`BoxedObservable::new`] erases a `Shared` one that is not `Send`. Erasing
//! never changes the mode.
//!
//! The `For` types have no method: their observer type `OR` must be named anyway, and so can the
//! type itself, `BoxedObservableFor::<_, _, _, _, _, OR>::new(observable)`.

use super::{Observable, ObservableTypes};
use crate::{
    disposable::{
        boxed_disposal::{BoxedDisposal, SendBoxedDisposal},
        dispose_on_drop::DisposeOnDrop,
    },
    observer::{
        Observer,
        boxed_observer::{BoxedObserver, ObserverMode, SendBoxedObserver},
    },
    thread_mode::{Local, Shared, ThreadMode},
    utils::MarkerType,
};
use educe::Educe;
use std::{marker::PhantomData, rc::Rc, sync::Arc};

/// Generates an erased observable over a fixed observer type (`$observable_for`) and the newtype
/// over it that accepts any observer (`$observable`).
///
/// The storage decides the rest: `FnOnce` subscribes the source it owns once, `Fn` clones the
/// source on every subscription and makes both types `Clone`. `source` lists the bounds on the
/// boxed observable besides its `Observable` impls; a trailing `Send` also requires the disposal,
/// and the observers that `$observable` boxes, to be `Send`.
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
                    dyn $fn_trait(OR) -> DisposeOnDrop<$disposal<'sub>> $(+ $marker)* + 'oe,
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
                OE::Disposal: 'sub $(+ $send)?,
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

        impl<'sub, T, E, M: ThreadMode, OR> ObservableTypes
            for $observable_for<'sub, '_, T, E, M, OR>
        {
            type Item = T;
            type Error = E;
            type Mode = M;
            type Disposal = $disposal<'sub>;
        }

        impl<T, E, M: ThreadMode, OR> Observable<OR> for $observable_for<'_, '_, T, E, M, OR>
        where
            OR: Observer<T, E>,
        {
            #[inline]
            fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
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
                OE::Disposal: 'sub $(+ $send)?,
            {
                Self($observable_for::new(observable))
            }
        }

        impl<'sub, T, E, M: ThreadMode> ObservableTypes for $observable<'_, 'sub, '_, T, E, M> {
            type Item = T;
            type Error = E;
            type Mode = M;
            type Disposal = $disposal<'sub>;
        }

        impl<'or, T, E, M: ThreadMode, OR> Observable<OR> for $observable<'or, '_, '_, T, E, M>
        where
            OR: Observer<T, E> $(+ $send)? + 'or,
        {
            #[inline]
            fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
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

/// What a [thread mode](crate::thread_mode) means for an erased observable: the box
/// [`into_boxed`](crate::observable::ObservableExt::into_boxed) and
/// [`into_cloneable_boxed`](crate::observable::ObservableExt::into_cloneable_boxed) erase it into.
///
/// It extends [`ObserverMode`], as the box it picks stores the boxed observer of that mode: the
/// plain boxes for [`Local`], the `Send` ones for [`Shared`]. Code generic over a mode states
/// `M: BoxedObservableMode` where it erases an observable.
pub trait BoxedObservableMode: ObserverMode {
    /// The erased observable of this mode: [`BoxedObservable`] or [`SendBoxedObservable`].
    type Boxed<'or, 'sub, 'oe, T, E>: ObservableTypes<Item = T, Error = E, Mode = Self>;

    /// The cloneable erased observable of this mode: [`CloneableBoxedObservable`] or
    /// [`SendCloneableBoxedObservable`].
    type CloneableBoxed<'or, 'sub, 'oe, T, E>: ObservableTypes<Item = T, Error = E, Mode = Self>
        + Clone;
}

impl BoxedObservableMode for Local {
    type Boxed<'or, 'sub, 'oe, T, E> = BoxedObservable<'or, 'sub, 'oe, T, E, Local>;
    type CloneableBoxed<'or, 'sub, 'oe, T, E> =
        CloneableBoxedObservable<'or, 'sub, 'oe, T, E, Local>;
}

impl BoxedObservableMode for Shared {
    type Boxed<'or, 'sub, 'oe, T, E> = SendBoxedObservable<'or, 'sub, 'oe, T, E, Shared>;
    type CloneableBoxed<'or, 'sub, 'oe, T, E> =
        SendCloneableBoxedObservable<'or, 'sub, 'oe, T, E, Shared>;
}

/// An observable that can be erased into the [`BoxedObservableMode::Boxed`] of its mode `M`: any
/// observable for [`Local`], a `Send` one with a `Send` disposal for [`Shared`].
///
/// [`into_boxed`](crate::observable::ObservableExt::into_boxed) asks for it, so that what the box
/// requires, `Send` included, is checked where the mode is concrete.
pub trait IntoBoxedObservable<'or, 'sub, 'oe, M: BoxedObservableMode>:
    ObservableTypes + Sized
{
    /// Erases the observable; call it as
    /// [`into_boxed`](crate::observable::ObservableExt::into_boxed).
    #[doc(hidden)]
    fn box_observable(self) -> M::Boxed<'or, 'sub, 'oe, Self::Item, Self::Error>;
}

impl<'or, 'sub, 'oe, OE> IntoBoxedObservable<'or, 'sub, 'oe, Local> for OE
where
    OE: ObservableTypes<Mode = Local>
        + Observable<
            BoxedObserver<'or, <OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
        > + 'oe,
    <OE as ObservableTypes>::Disposal: 'sub,
{
    fn box_observable(self) -> BoxedObservable<'or, 'sub, 'oe, OE::Item, OE::Error, Local> {
        BoxedObservable::new(self)
    }
}

impl<'or, 'sub, 'oe, OE> IntoBoxedObservable<'or, 'sub, 'oe, Shared> for OE
where
    OE: ObservableTypes<Mode = Shared>
        + Observable<
            SendBoxedObserver<'or, <OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
        > + Send
        + 'oe,
    <OE as ObservableTypes>::Disposal: Send + 'sub,
{
    fn box_observable(self) -> SendBoxedObservable<'or, 'sub, 'oe, OE::Item, OE::Error, Shared> {
        SendBoxedObservable::new(self)
    }
}

/// An observable that can be erased into the [`BoxedObservableMode::CloneableBoxed`] of its mode
/// `M`: any `Clone` observable for [`Local`], a `Send` and `Sync` one with a `Send` disposal for
/// [`Shared`].
///
/// [`into_cloneable_boxed`](crate::observable::ObservableExt::into_cloneable_boxed) asks for it;
/// see [`IntoBoxedObservable`].
pub trait IntoCloneableBoxedObservable<'or, 'sub, 'oe, M: BoxedObservableMode>:
    ObservableTypes + Sized
{
    /// Erases the observable; call it as
    /// [`into_cloneable_boxed`](crate::observable::ObservableExt::into_cloneable_boxed).
    #[doc(hidden)]
    fn box_cloneable_observable(self)
    -> M::CloneableBoxed<'or, 'sub, 'oe, Self::Item, Self::Error>;
}

impl<'or, 'sub, 'oe, OE> IntoCloneableBoxedObservable<'or, 'sub, 'oe, Local> for OE
where
    OE: ObservableTypes<Mode = Local>
        + Observable<
            BoxedObserver<'or, <OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
        > + Clone
        + 'oe,
    <OE as ObservableTypes>::Disposal: 'sub,
{
    fn box_cloneable_observable(
        self,
    ) -> CloneableBoxedObservable<'or, 'sub, 'oe, OE::Item, OE::Error, Local> {
        CloneableBoxedObservable::new(self)
    }
}

impl<'or, 'sub, 'oe, OE> IntoCloneableBoxedObservable<'or, 'sub, 'oe, Shared> for OE
where
    OE: ObservableTypes<Mode = Shared>
        + Observable<
            SendBoxedObserver<'or, <OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
        > + Clone
        + Send
        + Sync
        + 'oe,
    <OE as ObservableTypes>::Disposal: Send + 'sub,
{
    fn box_cloneable_observable(
        self,
    ) -> SendCloneableBoxedObservable<'or, 'sub, 'oe, OE::Item, OE::Error, Shared> {
        SendCloneableBoxedObservable::new(self)
    }
}
