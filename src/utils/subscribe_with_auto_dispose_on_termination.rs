use crate::{
    delegate_disposal,
    disposable::{Disposable, DisposableExt, shared_disposal::SharedDisposal},
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    utils::on_panic::on_panic,
};
use educe::Educe;

delegate_disposal!(
    Disposal<D>,
    SharedDisposal<Subscription<D>>,
    where D: Disposable
);

/// Wraps subscription creation so that termination from the observer automatically disposes the inner subscription.
pub fn subscribe_with_auto_dispose_on_termination<OR, D, F>(
    observer: OR,
    builder: F,
) -> Subscription<Disposal<D>>
where
    D: Disposable,
    F: FnOnce(AutoDisposeOnTerminationObserver<OR, D>) -> Subscription<D>,
{
    let shared_disposal = SharedDisposal::default();
    let observer = AutoDisposeOnTerminationObserver {
        observer,
        shared_disposal: shared_disposal.clone(),
    };
    shared_disposal.replace(|| builder(observer));

    shared_disposal.into_subscription()
}

/// Whether `OR` is an [`AutoDisposeOnTerminationObserver`], whatever its generic arguments are.
///
/// Only the outermost type is recognized: an auto-disposing observer wrapped inside another
/// observer is not detected. This is a best-effort check meant for `debug_assert!`, not a complete
/// one.
pub(crate) fn is_auto_dispose_on_termination_observer<OR>() -> bool {
    fn type_name_without_generics<T>() -> &'static str {
        let name = std::any::type_name::<T>();
        name.split_once('<').map_or(name, |(name, _)| name)
    }

    type_name_without_generics::<OR>()
        == type_name_without_generics::<AutoDisposeOnTerminationObserver<(), ()>>()
}

#[derive(Educe)]
#[educe(Debug)]
pub struct AutoDisposeOnTerminationObserver<OR, D: Disposable> {
    observer: OR,
    shared_disposal: SharedDisposal<Subscription<D>>,
}

impl<T, E, OR, D> Observer<T, E> for AutoDisposeOnTerminationObserver<OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        let flow = self.observer.on_next(value);
        if flow.is_stop() {
            // The observer ended its own stream, which is what a termination does too, so the
            // subscription is disposed here as well: the source is asked to stop by the flow this
            // returns, and released by the disposal whether or not it honors it.
            self.shared_disposal.clone().dispose();
        }
        flow
    }

    fn on_termination(self, termination: Termination<E>) {
        let Self {
            observer,
            shared_disposal,
        } = self;
        // Without the guard the source stays subscribed after the termination, until the
        // subscription `subscribe_with_auto_dispose_on_termination` returned is dropped. The
        // returning path disposes right after the callback, so the panicking path takes exactly
        // the locks the returning one would: nothing new can deadlock on the unwinding thread.
        let guard = on_panic(|| shared_disposal.clone().dispose());
        observer.on_termination(termination);
        drop(guard);
        shared_disposal.dispose();
    }
}
