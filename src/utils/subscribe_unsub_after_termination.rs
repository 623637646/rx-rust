use crate::{
    delegate_disposal,
    disposable::Disposable,
    observable::Subscription,
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{Mutable, MutableHelper, Shared},
};
use educe::Educe;

#[derive(Educe)]
#[educe(Debug)]
enum SubState<D: Disposable> {
    Initialized,
    Subscribed(Subscription<D>),
    Unsubscribed,
}

// TODO: Disposable should not be Cloneable
impl<D: Disposable> Disposable for Shared<Mutable<SubState<D>>> {
    fn dispose(self) {
        match safe_lock!(mem_replace: self, SubState::Unsubscribed) {
            SubState::Initialized => {}
            SubState::Subscribed(subscription) => subscription.dispose(),
            SubState::Unsubscribed => {}
        }
    }
}

delegate_disposal!(
    Disposal<D>,
    Shared<Mutable<SubState<D>>>,
    where D: Disposable
);

/// Wraps subscription creation so that termination from the observer automatically disposes the inner subscription.
pub fn subscribe_unsub_after_termination<OR, D, F>(
    observer: OR,
    builder: F,
) -> Subscription<Disposal<D>>
where
    D: Disposable,
    F: FnOnce(UnsubAfterTerminationObserver<OR, D>) -> Subscription<D>,
{
    let sub_state = Shared::new(Mutable::new(SubState::Initialized));
    let observer = UnsubAfterTerminationObserver {
        observer,
        sub_state: sub_state.clone(),
    };
    let sub = builder(observer);

    sub_state.lock_mut(|mut lock| match &*lock {
        SubState::Initialized => {
            *lock = SubState::Subscribed(sub);
            drop(lock);
        }
        SubState::Subscribed(_) => {
            drop(lock);
            unreachable!()
        }
        SubState::Unsubscribed => {
            drop(lock);
            sub.dispose()
        }
    });

    sub_state.into()
}

#[derive(Educe)]
#[educe(Debug)]
pub struct UnsubAfterTerminationObserver<OR, D: Disposable> {
    observer: OR,
    sub_state: Shared<Mutable<SubState<D>>>,
}

impl<T, E, OR, D> Observer<T, E> for UnsubAfterTerminationObserver<OR, D>
where
    OR: Observer<T, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        self.observer.on_termination(termination);
        self.sub_state.dispose();
    }
}
