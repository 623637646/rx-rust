use crate::disposable::{DisposableExt, option_disposal::OptionDisposal};
use crate::utils::types::MaybeSend;
use crate::{
    observable::Observable,
    observable::Subscription,
    observer::{Flow, Observer, Termination},
    subject::unicast_subject::{UnicastObservable, UnicastSender, unicast_subject},
};
use educe::Educe;
use std::{cmp::Ordering, num::NonZeroUsize};

/// Periodically subdivides items from an Observable into Observable windows, each containing a specified number of items.
///
/// A window is emitted before its first item is delivered, and each window can be subscribed to
/// once. Items emitted while a window has no subscriber are buffered and replayed to a later
/// subscriber; dropping a window without subscribing to it discards its items.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
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
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::new()));
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

impl<'or, T, E, OE> Observable<'or, UnicastObservable<'or, T, E>, E> for WindowWithCount<OE>
where
    T: MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
{
    type D = OptionDisposal<Subscription<OE::D>>;

    fn subscribe(
        self,
        mut observer: impl Observer<UnicastObservable<'or, T, E>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let (sender, window) = unicast_subject();
        if observer.on_next(window).is_stop() {
            // The first window ended the stream, so the source is never subscribed to.
            return OptionDisposal::none().into_subscription();
        }

        let observer = WindowWithCountObserver {
            observer,
            sender,
            count: self.count,
            sent_count: 0,
        };
        self.source
            .subscribe(observer)
            .into_option()
            .into_subscription()
    }
}

struct WindowWithCountObserver<'or, T, E, OR> {
    observer: OR,
    sender: UnicastSender<'or, T, E>,
    count: NonZeroUsize,
    sent_count: usize,
}

impl<'or, T, E, OR> Observer<T, E> for WindowWithCountObserver<'or, T, E, OR>
where
    E: Clone,
    OR: Observer<UnicastObservable<'or, T, E>, E>,
{
    fn on_next(&mut self, value: T) -> Flow {
        // The consumer of one window stops that window, not the operator: only what the observer
        // of the windows themselves answers can stop the source.
        match (self.sent_count + 1).cmp(&self.count.get()) {
            Ordering::Less => {
                let _ = self.sender.on_next(value);
                self.sent_count += 1;
                Flow::Continue
            }
            Ordering::Equal => {
                let (new_sender, new_window) = unicast_subject();
                let mut old_sender = std::mem::replace(&mut self.sender, new_sender);
                let _ = old_sender.on_next(value);
                old_sender.on_termination(Termination::Completed);
                self.sent_count = 0;
                self.observer.on_next(new_window)
            }
            Ordering::Greater => unreachable!(),
        }
    }

    fn on_termination(self, termination: Termination<E>) {
        self.sender.on_termination(termination.clone());
        self.observer.on_termination(termination);
    }
}
