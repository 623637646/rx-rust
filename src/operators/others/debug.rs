use crate::disposable::callback_disposal::CallbackDisposal;
use crate::utils::types::{NecessarySendSync, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::fmt::Display;

pub enum DebugEvent<'a, T, E> {
    OnNext(&'a T),
    OnTermination(&'a Termination<E>),
    Subscribed,
    Disposed,
}

/// Logs all items from the source Observable to the console, and re-emits them. This is useful for debugging.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         others::debug::Debug,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = Debug::new_default_print(FromIter::new(vec![1, 2]), "trace");
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![1, 2]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Debug<OE, L, F> {
    source: OE,
    label: L,
    callback: Shared<F>,
}

impl<OE, L, F> Debug<OE, L, F> {
    pub fn new(source: OE, label: L, callback: F) -> Self {
        Self {
            source,
            label,
            callback: Shared::new(callback),
        }
    }
}

pub type DefaultPrintType<L, T, E> = fn(L, DebugEvent<'_, T, E>);

impl<T, E, OE, L> Debug<OE, L, DefaultPrintType<L, T, E>> {
    pub fn new_default_print(source: OE, label: L) -> Self
    where
        L: Display,
        T: std::fmt::Debug,
        E: std::fmt::Debug,
    {
        Self {
            source,
            label,
            callback: Shared::new(|label, event| match event {
                DebugEvent::OnNext(value) => println!("[{}]: OnNext({:?})", label, value),
                DebugEvent::OnTermination(termination) => {
                    println!("[{}]: OnTermination({:?})", label, termination)
                }
                DebugEvent::Subscribed => println!("[{}]: Subscription", label),
                DebugEvent::Disposed => println!("[{}]: Dispose", label),
            }),
        }
    }
}

impl<'or, 'sub, T, E, OE, L, F> Observable<'or, 'sub, T, E> for Debug<OE, L, F>
where
    OE: Observable<'or, 'sub, T, E>,
    L: Clone + NecessarySendSync + 'or + 'sub,
    F: Fn(L, DebugEvent<'_, T, E>) + NecessarySendSync + 'or + 'sub,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        (self.callback)(self.label.clone(), DebugEvent::Subscribed);
        let observer = DebugObserver {
            observer,
            label: self.label.clone(),
            callback: self.callback.clone(),
        };
        self.source.subscribe(observer)
            + CallbackDisposal::new(move || {
                (self.callback)(self.label.clone(), DebugEvent::Disposed);
            })
    }
}

struct DebugObserver<OR, L, F> {
    observer: OR,
    label: L,
    callback: Shared<F>,
}

impl<T, E, OR, L, F> Observer<T, E> for DebugObserver<OR, L, F>
where
    OR: Observer<T, E>,
    L: Clone,
    F: Fn(L, DebugEvent<'_, T, E>),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(self.label.clone(), DebugEvent::OnNext(&value));
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(self.label.clone(), DebugEvent::OnTermination(&termination));
        self.observer.on_termination(termination);
    }
}
