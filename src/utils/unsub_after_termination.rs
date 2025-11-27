use crate::{
    disposable::{Disposable, subscription::Subscription},
    observer::{Observer, Termination},
    safe_lock,
    utils::types::{Mutable, MutableHelper, Shared},
};
use educe::Educe;

enum SubState<'sub> {
    Initialized,
    Subscribed(Subscription<'sub>),
    Unsubscribed,
}

impl Disposable for Shared<Mutable<SubState<'_>>> {
    fn dispose(self) {
        match safe_lock!(mem_replace: self, SubState::Unsubscribed) {
            SubState::Initialized => {}
            SubState::Subscribed(subscription) => subscription.dispose(),
            SubState::Unsubscribed => {}
        }
    }
}

/// Wraps subscription creation so that termination from the observer automatically disposes the inner subscription.
pub fn subscribe_unsub_after_termination<'sub, OR, F>(
    observer: OR,
    builder: F,
) -> Subscription<'sub>
where
    F: FnOnce(UnsubAfterTerminationObserver<'sub, OR>) -> Subscription<'sub>,
{
    let sub_state = Shared::new(Mutable::new(SubState::Initialized));
    let observer = UnsubAfterTerminationObserver {
        observer,
        sub_state: sub_state.clone(),
    };
    let sub = builder(observer);

    sub_state.lock_mut(|mut lock| match &*lock {
        SubState::Initialized => *lock = SubState::Subscribed(sub),
        SubState::Subscribed(_) => unreachable!(),
        SubState::Unsubscribed => sub.dispose(),
    });

    Subscription::new_with_disposal(sub_state)
}

#[derive(Educe)]
#[educe(Debug)]
pub struct UnsubAfterTerminationObserver<'sub, OR> {
    observer: OR,
    sub_state: Shared<Mutable<SubState<'sub>>>,
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
        self.sub_state.dispose();
    }
}
