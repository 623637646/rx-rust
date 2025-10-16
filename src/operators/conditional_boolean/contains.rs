use crate::utils::types::NecessarySend;
use crate::utils::unsub_after_termination::subscribe_unsub_after_termination;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Emits a single boolean value that indicates whether a source Observable emits a specified item.
/// See <https://reactivex.io/documentation/operators/contains.html>
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Contains<T, OE> {
    source: OE,
    item: T,
}

impl<T, OE> Contains<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE, item: T) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self { source, item }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, bool, E> for Contains<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
    T: PartialEq + NecessarySend + 'or,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<bool, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let observer = ContainsObserver {
                observer: Some(observer),
                item: self.item,
            };
            self.source.subscribe(observer)
        })
    }
}

struct ContainsObserver<T, OR> {
    observer: Option<OR>,
    item: T,
}

impl<T, E, OR> Observer<T, E> for ContainsObserver<T, OR>
where
    OR: Observer<bool, E>,
    T: PartialEq,
{
    fn on_next(&mut self, value: T) {
        if self.item == value
            && let Some(mut observer) = self.observer.take()
        {
            observer.on_next(true);
            observer.on_termination(Termination::Completed);
        }
    }

    fn on_termination(mut self, termination: Termination<E>) {
        if let Some(mut observer) = self.observer.take() {
            match termination {
                Termination::Completed => observer.on_next(false),
                Termination::Error(_) => {}
            }
            observer.on_termination(termination);
        }
    }
}
