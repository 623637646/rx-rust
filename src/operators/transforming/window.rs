use crate::utils::types::{Mutable, NecessarySend, Shared};
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
    utils::subscribe_unsub_after_termination::subscribe_unsub_after_termination,
};
use crate::{safe_lock, safe_lock_observer, safe_lock_option_observer};
use educe::Educe;

/// Periodically subdivides items from an Observable into Observable windows, each window being emitted when a `boundary` Observable emits an item.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::subscription::Subscription,
///     observable::observable_ext::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::window::Window,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let windows = Arc::new(Mutex::new(Vec::<Vec<i32>>::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::<Subscription>::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let windows_observer = Arc::clone(&windows);
/// let terminations_observer = Arc::clone(&terminations);
/// let inner_subscriptions_observer = Arc::clone(&inner_subscriptions);
///
/// let subscription = Window::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     move |window| {
///         let index = {
///             let mut windows = windows_observer.lock().unwrap();
///             windows.push(Vec::new());
///             windows.len() - 1
///         };
///         let windows_for_values = Arc::clone(&windows_observer);
///         let sub = window.subscribe_with_callback(
///             move |value| {
///                 windows_for_values.lock().unwrap()[index].push(value);
///             },
///             |_| {},
///         );
///         inner_subscriptions_observer.lock().unwrap().push(sub);
///     },
///     move |termination| terminations_observer
///         .lock()
///         .unwrap()
///         .push(termination),
/// );
///
/// source.on_next(1);
/// source.on_next(2);
/// boundary.on_next(());
/// source.on_next(3);
/// source.on_termination(Termination::Completed);
/// drop(subscription);
/// inner_subscriptions.lock().unwrap().drain(..).for_each(drop);
///
/// assert_eq!(
///     &*windows.lock().unwrap(),
///     &[vec![1, 2], vec![3]]
/// );
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Window<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Window<OE, OE1> {
    pub fn new<'or, 'sub, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, 'sub, T, E>,
        OE1: Observable<'or, 'sub, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, 'sub, T, E, OE, OE1>
    Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E> for Window<OE, OE1>
where
    T: Clone + NecessarySend + 'or,
    E: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
    OE1: Observable<'or, 'sub, (), E>,
    'sub: 'or,
{
    fn subscribe(
        self,
        observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E> + NecessarySend + 'or,
    ) -> Subscription<'sub> {
        subscribe_unsub_after_termination(observer, |mut observer| {
            let subject = PublishSubject::default();
            observer.on_next(SubjectObservable::new(subject.clone()));

            let subject = Shared::new(Mutable::new(subject));
            let observer = Shared::new(Mutable::new(Some(observer)));
            let window_observer = WindowObserver {
                observer: observer.clone(),
                subject: subject.clone(),
            };
            let boundary_observer = BoundaryObserver { observer, subject };
            let subscription_1 = self.boundary.subscribe(boundary_observer);
            let subscription_2 = self.source.subscribe(window_observer);
            subscription_1 + subscription_2
        })
    }
}

struct WindowObserver<'or, T, E, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    subject: Shared<Mutable<PublishSubject<'or, T, E>>>,
}

impl<'or, T, E, OR> Observer<T, E> for WindowObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, value: T) {
        safe_lock_observer!(on_next: self.subject, value);
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock!(clone: self.subject).on_termination(termination.clone());
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}

struct BoundaryObserver<'or, T, E, OR> {
    observer: Shared<Mutable<Option<OR>>>,
    subject: Shared<Mutable<PublishSubject<'or, T, E>>>,
}

impl<'or, T, E, OR> Observer<(), E> for BoundaryObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, _: ()) {
        let new_subject = PublishSubject::default();
        let old_subject = safe_lock!(mem_replace: self.subject, new_subject.clone());
        old_subject.on_termination(Termination::Completed);
        safe_lock_option_observer!(on_next: self.observer, SubjectObservable::new(new_subject));
    }

    fn on_termination(self, termination: Termination<E>) {
        safe_lock!(clone: self.subject).on_termination(termination.clone());
        safe_lock_option_observer!(on_termination: self.observer, termination);
    }
}
