use crate::{
    observer::{Observer, Termination},
    subscription::{
        Subscription,
        disposable::{Disposable, SharedDisposal},
    },
    utils::types::{Mutable, MutableHelper, Shared},
};

pub fn subscribe_unsub_after_termination<'sub, OR, F>(
    observer: OR,
    builder: F,
) -> Subscription<'sub>
where
    F: FnOnce(UnsubAfterTerminationObserver<'sub, OR>) -> Subscription<'sub>,
{
    let subscription = Shared::new(Mutable::new(None));
    let observer = UnsubAfterTerminationObserver {
        observer,
        subscription: subscription.clone(),
    };
    let sub = builder(observer);
    *subscription.lock_mut() = Some(sub);
    Subscription::new_with_disposal(SharedDisposal::new(subscription))
}

pub struct UnsubAfterTerminationObserver<'sub, OR> {
    observer: OR,
    subscription: Shared<Mutable<Option<Subscription<'sub>>>>,
}

impl<T, E, OR> Observer<T, E> for UnsubAfterTerminationObserver<'_, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
        if let Some(sub) = { self.subscription.lock_mut().take() } {
            sub.dispose()
        }
    }
}
