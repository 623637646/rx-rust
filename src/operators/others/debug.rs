use crate::disposable::callback_disposal::CallbackDisposal;
use crate::utils::types::NecessarySendSync;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::fmt::Display;

#[derive(Educe)]
#[educe(Debug, Clone, PartialEq, Eq)]
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
pub struct Debug<OE, C, F> {
    source: OE,
    context: C,
    callback: F,
}

impl<OE, C, F> Debug<OE, C, F> {
    pub fn new<T, E>(source: OE, context: C, callback: F) -> Self
    where
        F: Fn(C, DebugEvent<'_, T, E>),
    {
        Self {
            source,
            context,
            callback,
        }
    }
}

pub type DefaultPrintType<C, T, E> = fn(C, DebugEvent<'_, T, E>);

impl<T, E, OE, C> Debug<OE, C, DefaultPrintType<C, T, E>> {
    pub fn new_default_print(source: OE, label: C) -> Self
    where
        C: Display,
        T: std::fmt::Debug,
        E: std::fmt::Debug,
    {
        Self {
            source,
            context: label,
            callback: |label, event| match event {
                DebugEvent::OnNext(value) => println!("[{}]: OnNext({:?})", label, value),
                DebugEvent::OnTermination(termination) => {
                    println!("[{}]: OnTermination({:?})", label, termination)
                }
                DebugEvent::Subscribed => println!("[{}]: Subscription", label),
                DebugEvent::Disposed => println!("[{}]: Dispose", label),
            },
        }
    }
}

impl<'or, 'sub, T, E, OE, C, F> Observable<'or, 'sub, T, E> for Debug<OE, C, F>
where
    OE: Observable<'or, 'sub, T, E>,
    C: Clone + NecessarySendSync + 'or + 'sub,
    F: Fn(C, DebugEvent<'_, T, E>) + Clone + NecessarySendSync + 'or + 'sub,
{
    fn subscribe(
        self,
        observer: impl Observer<T, E> + NecessarySendSync + 'or,
    ) -> Subscription<'sub> {
        (self.callback)(self.context.clone(), DebugEvent::Subscribed);
        let observer = DebugObserver {
            observer,
            context: self.context.clone(),
            callback: self.callback.clone(),
        };
        self.source.subscribe(observer)
            + CallbackDisposal::new(move || {
                (self.callback)(self.context.clone(), DebugEvent::Disposed);
            })
    }
}

struct DebugObserver<OR, C, F> {
    observer: OR,
    context: C,
    callback: F,
}

impl<T, E, OR, C, F> Observer<T, E> for DebugObserver<OR, C, F>
where
    OR: Observer<T, E>,
    C: Clone,
    F: Fn(C, DebugEvent<'_, T, E>),
{
    fn on_next(&mut self, value: T) {
        (self.callback)(self.context.clone(), DebugEvent::OnNext(&value));
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        (self.callback)(
            self.context.clone(),
            DebugEvent::OnTermination(&termination),
        );
        self.observer.on_termination(termination);
    }
}
