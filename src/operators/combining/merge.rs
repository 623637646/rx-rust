use crate::utils::safe_lock::{SafeLockOption, SafeLockOptionObserver};
use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    utils::unsub_after_termination::subscribe_unsub_after_termination,
};
use educe::Educe;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Merge<OE1, OE2> {
    source_1: OE1,
    source_2: OE2,
}

impl<OE1, OE2> Merge<OE1, OE2> {
    pub fn new<'or, 'sub, T, E>(source_1: OE1, source_2: OE2) -> Self
    where
        OE1: Observable<'or, 'sub, T, E>,
        OE2: Observable<'or, 'sub, T, E>,
    {
        Self { source_1, source_2 }
    }
}

impl<'or, 'sub, T, E, OE1, OE2> Observable<'or, 'sub, T, E> for Merge<OE1, OE2>
where
    OE1: Observable<'or, 'sub, T, E>,
    OE2: Observable<'or, 'sub, T, E>,
    'sub: 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = Shared::new(Mutable::new(Some(observer)));
            let one_is_completed = Shared::new(AtomicBool::new(false));
            let onserver_1 = MergeObserver {
                observer: observer.clone(),
                one_is_completed: one_is_completed.clone(),
            };
            let onserver_2 = MergeObserver {
                observer,
                one_is_completed,
            };
            let subscription_1 = self.source_1.subscribe(onserver_1);
            let subscription_2 = self.source_2.subscribe(onserver_2);
            subscription_1 + subscription_2
        })
    }
}

struct MergeObserver<OR> {
    observer: Shared<Mutable<Option<OR>>>,
    one_is_completed: Shared<AtomicBool>,
}

impl<T, E, OR> Observer<T, E> for MergeObserver<OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.observer.safe_lock_on_next_if_some(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if self.one_is_completed.load(Ordering::SeqCst) {
                    if let Some(observer) = self.observer.safe_lock_take() {
                        observer.on_termination(termination);
                    }
                } else {
                    self.one_is_completed.store(true, Ordering::SeqCst);
                }
            }
            Termination::Error(_) => {
                if let Some(observer) = self.observer.safe_lock_take() {
                    observer.on_termination(termination);
                }
            }
        }
    }
}
