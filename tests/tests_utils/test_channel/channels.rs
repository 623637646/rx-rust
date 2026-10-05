//! [`test_channels`]: the channels of [`test_channel`](super::test_channel), one per subscription.
//!
//! A child module, so that it opens its channels through the private constructors of its parent,
//! which stay out of reach of the tests.

use super::{
    ChannelChecker, ChannelState, ReceiverObservable, ReceiverObservableDisposal, SenderObserver,
    SharedChannel, new_channel,
};
use educe::Educe;
use rx_rust::thread_mode::Shared;
use rx_rust::thread_mode::mutable::MutableHelper;
use rx_rust::{
    observable::{Observable, ObservableTypes, Subscription},
    observer::boxed_observer::IntoBoxedObserver,
    observer::{Flow, Observer, Termination},
};
use std::sync::{Arc, Mutex};

/// A cold source of [`test_channel`](super::test_channel)s: every subscription to the observable
/// opens a channel of its own, numbered from 0 in the order of the subscriptions.
///
/// It is what a test uses when one observable is subscribed more than once: by several observers,
/// or by an operator that resubscribes to its source (`retry`, `repeat`, a connectable). Each
/// subscription is then driven and checked on its own, which a hot subject cannot tell apart: the
/// disposal of one subscription shows as that channel going [`ChannelState::Unsubscribed`] while
/// the others stay [`ChannelState::Subscribed`].
///
/// Every channel is as strict as one of [`test_channel`](super::test_channel): it panics when it is
/// sent to before it was subscribed to, after it ended, or after it was disposed.
pub(crate) fn test_channels<'or, T, E>() -> (Channels<'or, T, E>, ChannelsObservable<'or, T, E>) {
    let channels = Arc::new(Mutex::new(Vec::new()));
    (Channels(channels.clone()), ChannelsObservable(channels))
}

type SharedChannels<'or, T, E> = Arc<Mutex<Vec<SharedChannel<'or, T, E>>>>;

/// The sending and checking end of [`test_channels`], addressing each channel by the index of its
/// subscription. It is [`Clone`], so a test can send from inside a callback of the pipeline.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct Channels<'or, T, E>(SharedChannels<'or, T, E>);

impl<'or, T, E> Channels<'or, T, E> {
    /// The number of subscriptions so far, which is the number of channels.
    pub(crate) fn len(&self) -> usize {
        self.0.with_ref(Vec::len)
    }

    fn channel(&self, index: usize) -> SharedChannel<'or, T, E> {
        let channel = self.0.with_ref(|channels| channels.get(index).cloned());
        // Panic outside the lock, which leaves it usable.
        channel.unwrap_or_else(|| panic!("the channel {index} was never subscribed to"))
    }

    /// Sends a value through the channel of the subscription `index`, answering what its observer
    /// answered. See [`SenderObserver::on_next`].
    pub(crate) fn on_next(&self, index: usize, value: T) -> Flow
    where
        E: Clone,
    {
        SenderObserver {
            channel: self.channel(index),
        }
        .on_next(value)
    }

    /// Ends the channel of the subscription `index`. See [`SenderObserver::on_termination`].
    pub(crate) fn on_termination(&self, index: usize, termination: Termination<E>)
    where
        E: Clone,
    {
        SenderObserver {
            channel: self.channel(index),
        }
        .on_termination(termination)
    }

    /// The state of the channel of the subscription `index`: [`ChannelState::Initialized`] while
    /// that subscription has not happened yet, as for a channel nobody subscribed to.
    pub(crate) fn state(&self, index: usize) -> ChannelState<E>
    where
        E: Clone,
    {
        let channel = self.0.with_ref(|channels| channels.get(index).cloned());
        match channel {
            Some(channel) => ChannelChecker(channel).state(),
            None => ChannelState::Initialized,
        }
    }
}

/// The observable end of [`test_channels`]: each subscription opens a new channel.
#[derive(Educe)]
#[educe(Debug, Clone)]
pub(crate) struct ChannelsObservable<'or, T, E>(SharedChannels<'or, T, E>);

impl<'or, T, E> ObservableTypes for ChannelsObservable<'or, T, E> {
    type Item = T;
    type Error = E;
    type Mode = Shared;
    type D = ReceiverObservableDisposal<'or, T, E>;
}

impl<'or, T, E, OR> Observable<OR> for ChannelsObservable<'or, T, E>
where
    OR: IntoBoxedObserver<'or, T, E, Shared>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        let channel = new_channel();
        // Registered before the subscription, so that a value sent from inside it already finds
        // the channel.
        self.0.with_mut(|channels| channels.push(channel.clone()));
        ReceiverObservable { channel }.subscribe(observer)
    }
}
