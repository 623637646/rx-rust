use crate::utils::types::NecessarySend;
use crate::{
    disposable::subscription::Subscription,
    observable::Observable,
    observer::{Observer, Termination},
    subject::{publish_subject::PublishSubject, subject_observable::SubjectObservable},
};
use educe::Educe;
use std::{cmp::Ordering, num::NonZeroUsize};

/// Periodically subdivides items from an Observable into Observable windows, each containing a specified number of items.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     disposable::subscription::Subscription,
///     observable::observable_ext::ObservableExt,
///     observer::Termination,
///     operators::{
///         creating::from_iter::FromIter,
///         transforming::window_with_count::WindowWithCount,
///     },
/// };
/// use std::{num::NonZeroUsize, sync::{Arc, Mutex}};
///
/// let windows = Arc::new(Mutex::new(Vec::<Vec<i32>>::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::<Subscription>::new()));
/// let windows_observer = Arc::clone(&windows);
/// let terminations_observer = Arc::clone(&terminations);
/// let inner_subscriptions_observer = Arc::clone(&inner_subscriptions);
///
/// let subscription = WindowWithCount::new(
///     FromIter::new(vec![1, 2, 3, 4]),
///     NonZeroUsize::new(2).unwrap(),
/// )
/// .subscribe_with_callback(
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
/// drop(subscription);
/// inner_subscriptions.lock().unwrap().drain(..).for_each(drop);
///
/// assert_eq!(
///     &*windows.lock().unwrap(),
///     &[vec![1, 2], vec![3, 4], vec![]]
/// );
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct WindowWithCount<OE> {
    source: OE,
    count: NonZeroUsize,
}

impl<OE> WindowWithCount<OE> {
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self { source, count }
    }
}

impl<'or, 'sub, T, E, OE> Observable<'or, 'sub, SubjectObservable<PublishSubject<'or, T, E>>, E>
    for WindowWithCount<OE>
where
    T: Clone + NecessarySend + 'or,
    E: Clone + NecessarySend + 'or,
    OE: Observable<'or, 'sub, T, E>,
{
    fn subscribe(
        self,
        mut observer: impl Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>
        + NecessarySend
        + 'or,
    ) -> Subscription<'sub> {
        let subject = PublishSubject::default();
        observer.on_next(SubjectObservable::new(subject.clone()));

        let observer = WindowWithCountObserver {
            observer,
            subject,
            count: self.count,
            sent_count: 0,
        };
        self.source.subscribe(observer)
    }
}

struct WindowWithCountObserver<'or, T, E, OR> {
    observer: OR,
    subject: PublishSubject<'or, T, E>,
    count: NonZeroUsize,
    sent_count: usize,
}

impl<'or, T, E, OR> Observer<T, E> for WindowWithCountObserver<'or, T, E, OR>
where
    T: Clone,
    E: Clone,
    OR: Observer<SubjectObservable<PublishSubject<'or, T, E>>, E>,
{
    fn on_next(&mut self, value: T) {
        match (self.sent_count + 1).cmp(&self.count.get()) {
            Ordering::Less => {
                self.subject.on_next(value);
                self.sent_count += 1;
            }
            Ordering::Equal => {
                let new_subject = PublishSubject::default();
                let mut old_subject = std::mem::replace(&mut self.subject, new_subject.clone());
                old_subject.on_next(value);
                old_subject.on_termination(Termination::Completed);
                self.observer.on_next(SubjectObservable::new(new_subject));
                self.sent_count = 0;
            }
            Ordering::Greater => unreachable!(),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.subject.on_termination(termination.clone());
        self.observer.on_termination(termination);
    }
}
