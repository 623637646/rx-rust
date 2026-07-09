use crate::utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination;
use crate::utils::subscribe_with_shared_model::{
    Context, ModificationResult, subscribe_with_shared_model,
};
use crate::utils::types::MaybeSend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Discards items emitted by a source Observable until a second Observable emits an item.
/// See <https://reactivex.io/documentation/operators/skipuntil.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::conditional_boolean::skip_until::SkipUntil,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut gate: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = SkipUntil::new(source.clone(), gate.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// gate.on_next(());
/// source.on_next(2);
/// source.on_termination(Termination::Completed);
/// drop(subscription);
///
/// assert_eq!(&*values.lock().unwrap(), &[2]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct SkipUntil<OE, OE1> {
    source: OE,
    start: OE1,
}

impl<OE, OE1> SkipUntil<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, start: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, start }
    }
}

impl<'or, 'sub, T, E, OE, OE1> Observable<'or, 'sub, T, E> for SkipUntil<OE, OE1>
where
    'sub: 'or,
    'or: 'sub,
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
{
    fn subscribe(self, observer: impl Observer<T, E> + MaybeSend + 'or) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |observer| {
            let model = Model { started: false };
            subscribe_with_shared_model(observer, model, |context| {
                let subscription_1 = self.start.subscribe(StartObserver {
                    context: context.clone(),
                    started: false,
                });
                let subscription_2 = self.source.subscribe(SkipUntilObserver(context));
                subscription_1 + subscription_2
            })
        })
    }
}

struct Model {
    started: bool,
}

struct SkipUntilObserver<T, E, OR>(Context<T, E, OR, Model>);

impl<T, E, OR> Observer<T, E> for SkipUntilObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.modify_model(|model| {
            if model.started {
                ModificationResult::new_send_next(value)
            } else {
                ModificationResult::new_without_result()
            }
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        self.0.send_termination(termination);
    }
}

struct StartObserver<T, E, OR> {
    context: Context<T, E, OR, Model>,
    started: bool,
}

impl<T, E, OR> Observer<(), E> for StartObserver<T, E, OR>
where
    OR: Observer<T, E>,
{
    fn on_next(&mut self, _: ()) {
        if !self.started {
            self.started = true;
            let _ = self.context.modify_model(|model| {
                model.started = true;
                ModificationResult::new_without_result().ignore_drop_outside()
            });
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        match termination {
            Termination::Completed => {
                if !self.started {
                    self.context.send_termination(termination);
                }
            }
            Termination::Error(_) => {
                self.context.send_termination(termination);
            }
        }
    }
}
