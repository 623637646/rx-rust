use crate::utils::types::MaybeSend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Emits a specified item if the source Observable completes without emitting any items.
/// See <https://reactivex.io/documentation/operators/defaultifempty.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         conditional_boolean::default_if_empty::DefaultIfEmpty,
///         creating::from_iter::FromIter,
///     },
/// };
///
/// let mut values = Vec::new();
/// let mut terminations = Vec::new();
///
/// let observable = DefaultIfEmpty::new(FromIter::new(Vec::<i32>::new()), 42);
/// observable.subscribe_with_callback(
///     |value| values.push(value),
///     |termination| terminations.push(termination),
/// );
///
/// assert_eq!(values, vec![42]);
/// assert_eq!(terminations, vec![Termination::Completed]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct DefaultIfEmpty<T, OE> {
    source: OE,
    default_value: T,
}

impl<T, OE> DefaultIfEmpty<T, OE> {
    pub fn new<'or, 'sub, E>(source: OE, item: T) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
    {
        Self {
            source,
            default_value: item,
        }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, T, E> for DefaultIfEmpty<T, OE>
where
    OE: Observable<'or, 'sub, T, E>,
    T: MaybeSend + 'or,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<'sub> {
        let observer = DefaultIfEmptyObserver {
            observer,
            default_value: Some(self.default_value),
        };
        self.source.subscribe(observer)
    }
}

struct DefaultIfEmptyObserver<T, OR> {
    observer: OR,
    default_value: Option<T>, // None means the source's got at least one value already.
}

impl<T, E, OR> Observer<T, E> for DefaultIfEmptyObserver<T, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        self.default_value = None;
        self.observer.on_next(value);
    }

    fn on_termination(mut self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if let Some(default_value) = self.default_value {
                    self.observer.on_next(default_value);
                }
            }
            Termination::Error(_) => {}
        }
        self.observer.on_termination(termination);
    }
}
