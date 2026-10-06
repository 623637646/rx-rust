//! The [`WindowWithCount`] operator, behind
//! [`ObservableExt::window_with_count`](crate::observable::ObservableExt::window_with_count).

use crate::delegate_disposal;
use crate::disposable::{Disposable, DisposableExt, option_disposal::OptionDisposal};
use crate::observer::boxed_observer::ObserverMode;
use crate::utils::MarkerType;
use crate::{
    observable::Subscription,
    observable::{Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    subject::unicast_subject::{self, BoxedUnicastObservable, BoxedUnicastSender},
};
use educe::Educe;
use std::marker::PhantomData;
use std::{cmp::Ordering, num::NonZeroUsize};

/// Subdivides the items of an Observable into Observable windows of a specified number of items.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// A window is emitted before its first item is delivered, and each window can be subscribed to
/// once. Items emitted while a window has no subscriber are buffered and replayed to a later
/// subscriber; dropping a window without subscribing to it discards its items.
///
/// Disposing the subscription of a single window does not necessarily release its observer where
/// it happens: the window releases it on its next item, when it ends, or when the outer
/// subscription is disposed, whichever comes first. See
/// [the unicast subject](crate::subject::unicast_subject) each window is built on.
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
pub struct WindowWithCount<'a, OE> {
    source: OE,
    count: NonZeroUsize,
    /// The observer of a window may borrow for `'a`. Unlike the `_boxed` hooks this cannot be left
    /// to an unboxed default: the window is the `Item`, which [`ObservableTypes`] names without any
    /// observer, and its observer arrives only later, so the window boxes it.
    _marker: MarkerType<&'a ()>,
}

impl<OE> WindowWithCount<'_, OE> {
    /// Creates a [`WindowWithCount`] over `source`;
    /// [`ObservableExt::window_with_count`](crate::observable::ObservableExt::window_with_count) is
    /// the fluent form.
    pub fn new(source: OE, count: NonZeroUsize) -> Self {
        Self {
            source,
            count,
            _marker: PhantomData,
        }
    }
}

delegate_disposal!(
    Disposal<D>,
    OptionDisposal<Subscription<D>>,
    where D: Disposable
);

impl<'a, T, E, OE> ObservableTypes for WindowWithCount<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    E: Clone,
    OE: ObservableTypes<Item = T, Error = E>,
{
    type Item = BoxedUnicastObservable<'a, T, E, OE::Mode>;
    type Error = E;
    type Mode = OE::Mode;
    type Disposal = Disposal<OE::Disposal>;
}

impl<'a, T, E, OE, OR> Observable<OR> for WindowWithCount<'a, OE>
where
    <OE as ObservableTypes>::Mode: ObserverMode,
    OR: Observer<BoxedUnicastObservable<'a, T, E, <OE as ObservableTypes>::Mode>, E>,
    E: Clone,
    OE: Observable<
            WindowWithCountObserver<'a, <OE as ObservableTypes>::Mode, T, E, OR>,
            Item = T,
            Error = E,
        >,
{
    fn subscribe(self, mut observer: OR) -> Subscription<Self::Disposal> {
        let (sender, window) = unicast_subject::new_boxed();
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

pub struct WindowWithCountObserver<'a, M: ObserverMode, T, E, OR> {
    observer: OR,
    sender: BoxedUnicastSender<'a, T, E, M>,
    count: NonZeroUsize,
    sent_count: usize,
}

impl<'a, M, T, E, OR> Observer<T, E> for WindowWithCountObserver<'a, M, T, E, OR>
where
    M: ObserverMode,
    E: Clone,
    OR: Observer<BoxedUnicastObservable<'a, T, E, M>, E>,
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
                let (new_sender, new_window) = unicast_subject::new_boxed();
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
