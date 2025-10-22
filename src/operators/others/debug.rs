use crate::disposable::callback_disposal::CallbackDisposal;
use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;
use std::fmt::Display;

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
/// let observable = Debug::new(FromIter::new(vec![1, 2]), "trace");
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
pub struct Debug<OE, D> {
    source: OE,
    label: D,
}

impl<OE, D> Debug<OE, D> {
    pub fn new(source: OE, label: D) -> Self {
        Self { source, label }
    }
}

impl<'or, 'sub, T, E, OE, D> Observable<'or, 'sub, T, E> for Debug<OE, D>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
    OE: Observable<'or, 'sub, T, E>,
    D: Display + Clone + NecessarySend + 'or + 'sub,
{
    fn subscribe(self, observer: impl Observer<T, E> + NecessarySend + 'or) -> Subscription<'sub> {
        let observer = DebugObserver {
            observer,
            label: self.label.clone(),
        };
        println!("[{}] subscribe", self.label);
        self.source.subscribe(observer)
            + CallbackDisposal::new(move || println!("[{}] dispose", self.label))
    }
}

struct DebugObserver<OR, D> {
    observer: OR,
    label: D,
}

impl<T, E, OR, D> Observer<T, E> for DebugObserver<OR, D>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
    OR: Observer<T, E>,
    D: Display,
{
    fn on_next(&mut self, value: T) {
        println!("[{}] on_next: {:?}", self.label, value);
        self.observer.on_next(value);
    }

    fn on_termination(self, termination: Termination<E>) {
        println!("[{}] on_termination: {:?}", self.label, termination);
        self.observer.on_termination(termination);
    }
}
