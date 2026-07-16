use crate::{
    delegate_disposal,
    disposable::{Disposable, DisposableExt, shared_disposal::SharedDisposal},
    observable::Subscription,
    observer::{Observer, Termination},
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
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
        self.shared_disposal.dispose(); // TODO: if on_termination panic, the disposal is not disposed
    }
}
