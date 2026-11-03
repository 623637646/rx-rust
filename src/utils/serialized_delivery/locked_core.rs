//! The lock around the state of a [`SerializedDelivery`](super::SerializedDelivery), in a module of
//! its own so that nothing but a stop can raise the flag that goes with it, which the rest of the
//! crate reads through a [`StopView`].

use super::Core;
use crate::thread_mode::{
    ThreadMode,
    mutable::{MutableBoolHelper, MutableHelper},
};
use educe::Educe;

/// The lock around a delivery's [`Core`], shared by every handle of the delivery and by its
/// [`DeliveryStop`](super::DeliveryStop)s, together with a flag saying, without the lock,
/// whether a [`stop_with`](Self::stop_with) has stopped it.
///
/// Until a stop has run, nothing can be parked in a stopped core, so the `Drop` of a handle has
/// nothing to release and, through [`after_stop`](Self::after_stop), takes no lock. The flag
/// is raised under the lock, together with the stop, so a handle that reads it raised and then
/// locks finds the state stopped. Its own module keeps the flag out of reach of the rest of the
/// delivery, so that nothing else can raise it.
#[derive(Educe)]
#[educe(Clone(bound()))]
pub(super) struct LockedCore<M: ThreadMode, T, E, R> {
    core: M::Ptr<Core<T, E, R>>,
    stopped: M::Flag,
}

impl<M: ThreadMode, T, E, R> LockedCore<M, T, E, R> {
    pub(super) fn new(core: Core<T, E, R>) -> Self {
        Self {
            core: M::ptr(core),
            stopped: M::Flag::default(),
        }
    }

    /// Runs `callback` on the core, under the lock.
    pub(super) fn with_mut<Out>(&self, callback: impl FnOnce(&mut Core<T, E, R>) -> Out) -> Out {
        self.core.with_mut(callback)
    }

    /// Stops the core and runs `callback` under the same lock: returns what the core held and
    /// what `callback` returned, to be dropped outside the lock, in that order.
    ///
    /// Every stop goes through here, whether or not it can reach the observer: a
    /// `DeliveryStop` passes a callback that does nothing, the delivery one that takes the
    /// observer out of its cell. Only a delivery that terminates ends elsewhere, in its loop,
    /// which consumes the observer.
    pub(super) fn stop_with<Out>(&self, callback: impl FnOnce() -> Out) -> (Core<T, E, R>, Out) {
        self.core.with_mut(|core| {
            self.stopped.write(true);
            (std::mem::replace(core, Core::Stopped), callback())
        })
    }

    /// Runs `callback` under the lock once a [`stop_with`](Self::stop_with) has run, and takes
    /// no lock before: [`None`] until then. The core is then stopped for good.
    pub(super) fn after_stop<Out>(&self, callback: impl FnOnce() -> Out) -> Option<Out> {
        if !self.stopped.read() {
            return None;
        }
        Some(self.core.with_mut(|core| {
            debug_assert!(matches!(core, Core::Stopped), "a stop is final");
            callback()
        }))
    }

    /// Creates a view of the flag, which tells without the lock whether a stop has run.
    pub(super) fn stop_view(&self) -> StopView<M> {
        StopView(self.stopped.clone())
    }
}

/// Tells, without a lock, whether a delivery has been stopped other than by its termination: by a
/// [`DeliveryStop`](super::DeliveryStop), or by its own loop, as its observer answers
/// [`Flow::Stop`](crate::observer::Flow::Stop) or panics.
///
/// While the observer runs, only the first can happen, so the observer learns from it that its
/// subscription has been disposed — from inside one of its own notifications, or from another
/// thread meanwhile — which the delivery itself checks between two events only. An observer that
/// emits several events for one, or runs user code before an emission, checks it before each of
/// them. A disposal during the termination raises it too: a stop raises it whatever state it finds
/// the core in, a termination that has already stopped it included.
///
/// It is raised under the lock, before the stop drops what the core held, so code that runs as the
/// resources are dropped — the disposal of a source, and a user callback it runs — sees it raised.
pub(crate) struct StopView<M: ThreadMode>(M::Flag);

impl<M: ThreadMode> StopView<M> {
    /// Whether the delivery has been stopped, for good, other than by its termination.
    pub(crate) fn is_stopped(&self) -> bool {
        self.0.read()
    }
}
