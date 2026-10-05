//! An observer whose concrete type is erased, and the box each thread mode erases into.

use super::{Flow, Observer, Termination};
use crate::thread_mode::{Local, Shared, ThreadMode};
use educe::Educe;

trait ErasedObserver<T, E>: Observer<T, E> {
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>);
}

impl<T, E, OR> ErasedObserver<T, E> for OR
where
    OR: Observer<T, E>,
{
    fn on_termination_boxed(self: Box<Self>, termination: Termination<E>) {
        Observer::on_termination(*self, termination);
    }
}

macro_rules! boxed_observer {
    ($(#[$meta:meta])* $name:ident $(, $send:ident)?) => {
        $(#[$meta])*
        #[derive(Educe)]
        #[educe(Debug)]
        pub struct $name<'or, T, E>(
            #[educe(Debug(ignore))] Box<dyn ErasedObserver<T, E> $(+ $send)? + 'or>,
        );

        impl<'or, T, E> $name<'or, T, E> {
            /// Boxes `observer`.
            pub fn new(observer: impl Observer<T, E> $(+ $send)? + 'or) -> Self {
                Self(Box::new(observer))
            }
        }

        impl<T, E> Observer<T, E> for $name<'_, T, E> {
            #[inline]
            fn on_next(&mut self, value: T) -> Flow {
                self.0.on_next(value)
            }

            #[inline]
            fn on_termination(self, termination: Termination<E>) {
                self.0.on_termination_boxed(termination);
            }
        }
    };
}

boxed_observer!(
    /// An observer whose concrete type is erased.
    ///
    /// [`Observer::on_termination`] takes `self` by value, which a `Box<dyn Observer>` could not
    /// call (see <https://stackoverflow.com/q/46620790/9315497>), so the erasure goes through a
    /// private trait that terminates a `Box<Self>` instead. It is not `Send`; [`SendBoxedObserver`]
    /// is. Operators that hand the observer to user code, such as `create` and
    /// `hook_on_termination`, pass the one their [thread mode](ObserverMode::BoxedObserver) picks.
    ///
    /// # Examples
    /// ```rust
    /// use rx_rust::observer::{boxed_observer::BoxedObserver, BoxedObserverExt, Flow, Observer, Termination};
    /// use rx_rust::observer::callback_observer::CallbackObserver;
    ///
    /// let mut seen = Vec::new();
    /// let mut observer: BoxedObserver<'_, i32, ()> =
    ///     CallbackObserver::new(|value| seen.push(value), |_| {}).into_boxed();
    /// assert_eq!(observer.on_next(1), Flow::Continue);
    /// observer.on_termination(Termination::Completed);
    /// assert_eq!(seen, [1]);
    /// ```
    BoxedObserver
);

boxed_observer!(
    /// A [`BoxedObserver`] that is also `Send`, for state that crosses threads.
    SendBoxedObserver,
    Send
);

/// What a [thread mode](crate::thread_mode) means for an observer: the box it is erased into, and
/// how to get it there.
///
/// It extends [`ThreadMode`] here rather than being part of it, so that the thread mode depends on
/// nothing above it. Both modes implement it, so any concrete mode has it; code generic over a
/// mode states `M: ObserverMode` where it boxes an observer.
pub trait ObserverMode: ThreadMode + Sized {
    /// The boxed observer a state of this mode stores: [`BoxedObserver`] or [`SendBoxedObserver`].
    type BoxedObserver<'a, T, E>: Observer<T, E>;

    /// Boxes `observer` the way this mode stores it: as it is for [`Local`], only if it is `Send`
    /// for [`Shared`], which [`IntoBoxedObserver`] checks where the mode is concrete.
    fn boxed<'a, T, E, OR>(observer: OR) -> Self::BoxedObserver<'a, T, E>
    where
        OR: IntoBoxedObserver<'a, T, E, Self>,
    {
        OR::box_observer(observer)
    }
}

impl ObserverMode for Local {
    type BoxedObserver<'a, T, E> = BoxedObserver<'a, T, E>;
}

impl ObserverMode for Shared {
    type BoxedObserver<'a, T, E> = SendBoxedObserver<'a, T, E>;
}

/// An observer that can be boxed into the [`ObserverMode::BoxedObserver`] of `M`: any observer for
/// [`Local`], a `Send` one for [`Shared`].
///
/// This is how an operator that is generic over its mode — `create`, the subjects, the hooks —
/// states what it asks of its observer: `OR: IntoBoxedObserver<'a, T, E, M>`, checked, `Send`
/// included, where the mode is concrete. The boxing itself is spelled
/// [`M::boxed(observer)`](ObserverMode::boxed).
pub trait IntoBoxedObserver<'a, T, E, M: ObserverMode>: Observer<T, E> + Sized {
    /// Boxes the observer; call it as [`ObserverMode::boxed`]. An associated function rather than a
    /// method, so that it does not show up on every observer next to
    /// [`into_boxed`](super::BoxedObserverExt::into_boxed).
    #[doc(hidden)]
    fn box_observer(observer: Self) -> M::BoxedObserver<'a, T, E>;
}

impl<'a, T, E, OR> IntoBoxedObserver<'a, T, E, Local> for OR
where
    OR: Observer<T, E> + 'a,
{
    fn box_observer(observer: Self) -> BoxedObserver<'a, T, E> {
        BoxedObserver::new(observer)
    }
}

impl<'a, T, E, OR> IntoBoxedObserver<'a, T, E, Shared> for OR
where
    OR: Observer<T, E> + Send + 'a,
{
    fn box_observer(observer: Self) -> SendBoxedObserver<'a, T, E> {
        SendBoxedObserver::new(observer)
    }
}
