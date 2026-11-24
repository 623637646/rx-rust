//! Subscribing the sources of a cursor one at a time: what every operator over a source set is
//! built on.
//!
//! A [`SourceSubscriber`] is the operator's part — which observer a source gets, where its
//! subscription goes, whether to go on — and [`SubscribeEach`] the cursor's: it hands each
//! remaining source to the subscriber, in the one order every operator follows. Each cursor
//! implements it directly — a tuple of `Option`s, whose sources it takes in place, and an
//! [`Enumerate`] of an iterator, which asks the subscriber whether the stream still runs after
//! each `Iterator::next` — and both run the steps of one source in [`subscribe_one`].

use crate::{
    disposable::dispose_on_drop::DisposeOnDrop,
    observable::{Observable, ObservableTypes},
    observer::Observer,
    utils::positional::{
        position::{Position, TupleIndex},
        tuple::for_each_arity,
    },
};
use std::{convert::Infallible, iter::Enumerate, ops::ControlFlow};

/// Subscribes the source `OE` at the position `K`, in the steps [`SubscribeEach`] runs in order:
/// [`still_running`](Self::still_running) after an iterator was pulled,
/// [`observer`](Self::observer), the subscription itself, then [`store`](Self::store).
///
/// An operator has one impl over `K: Position` for tuples and lists alike, or one per tuple
/// position (`When`), so that the bounds of each source sit on its own impl. `B` is the reason to
/// break; `Infallible` means the subscriber takes every source.
///
/// The subscriber is borrowed mutably, as an `FnMut` is: what it gathers is a plain field. One
/// that is a shared handle, cloned into the observers it makes, is implemented for `&Handle`.
pub(crate) trait SourceSubscriber<K: Position, OE: ObservableTypes, B = Infallible> {
    /// The observer the source at `K` is subscribed with.
    type Observer;

    /// Whether the stream still runs, asked once a source came out of `Iterator::next`: that is
    /// user code, which can end the stream through a source already subscribed, and the source it
    /// returned is then dropped instead of being subscribed. It is asked for no source of a
    /// tuple, which is taken directly; a subscriber whose cursor runs no user code — the sources
    /// were collected first — answers `Continue` without looking.
    fn still_running(&self) -> ControlFlow<B>;

    /// Makes the observer of the source at `position`, which is subscribed with it next; or
    /// answers `None` when the source is no longer wanted, which is then dropped, and the
    /// traversal goes on with the next.
    fn observer(&mut self, position: K) -> Option<Self::Observer>;

    /// Stores the subscription of the source at `position`, and says whether to go on: the
    /// source may have ended the stream as it was subscribed.
    fn store(&mut self, position: K, subscription: DisposeOnDrop<OE::Disposal>) -> ControlFlow<B>;
}

/// A cursor whose remaining sources the subscriber `S` can subscribe: the bound an operator
/// writes on its [`SourceSet::Cursor`](super::SourceSet::Cursor).
///
/// It holds when `S` is a [`SourceSubscriber`] of every source left, at its position.
pub(crate) trait SubscribeEach<S, B = Infallible> {
    /// Subscribes the remaining sources in order until the subscriber breaks or none remain.
    ///
    /// The source it breaks on is consumed: calling again resumes after it, which is how an
    /// operator takes its sources a few at a time. A subscriber that cannot break (`B` is
    /// `Infallible`) is handed every source: `let ControlFlow::Continue(()) = …;` says so.
    fn subscribe_each(&mut self, subscriber: &mut S) -> ControlFlow<B>;
}

/// The steps of one source, once the cursor has it: the observer, the subscription, the store.
/// The one place that orders them.
fn subscribe_one<S, K, OE, B>(subscriber: &mut S, position: K, source: OE) -> ControlFlow<B>
where
    S: SourceSubscriber<K, OE, B>,
    K: Position,
    OE: Observable<S::Observer>,
    S::Observer: Observer<<OE as ObservableTypes>::Item, <OE as ObservableTypes>::Error>,
{
    let Some(observer) = subscriber.observer(position) else {
        return ControlFlow::Continue(());
    };
    let subscription = source.subscribe(observer);
    subscriber.store(position, subscription)
}

/// The sources of an iterator, at their dynamic positions. Each came out of `Iterator::next`,
/// user code that may have ended the stream, so the subscriber is asked again whether it still
/// runs before the source is subscribed; a source pulled after that is dropped.
impl<I, S, B> SubscribeEach<S, B> for Enumerate<I>
where
    I: Iterator,
    I::Item: Observable<<S as SourceSubscriber<usize, I::Item, B>>::Observer>,
    S: SourceSubscriber<usize, I::Item, B>,
    <S as SourceSubscriber<usize, I::Item, B>>::Observer:
        Observer<<I::Item as ObservableTypes>::Item, <I::Item as ObservableTypes>::Error>,
{
    fn subscribe_each(&mut self, subscriber: &mut S) -> ControlFlow<B> {
        self.try_for_each(|(position, source)| {
            subscriber.still_running()?;
            subscribe_one(subscriber, position, source)
        })
    }
}

/// [`SubscribeEach`] for a tuple of `Option`s, called by [`for_each_arity!`] once per arity: each
/// source is taken out of its element at its static position, so the subscriber has an impl, and
/// bounds, per position. Taken directly, no source runs user code before it is subscribed.
macro_rules! impl_subscribe_each {
    ($(($position:tt, $source:ident, $($_names:tt)*)),+) => {
        impl<S, B, $($source),+> SubscribeEach<S, B> for ($(Option<$source>,)+)
        where
            $(
                $source: ObservableTypes
                    + Observable<
                        <S as SourceSubscriber<TupleIndex<$position>, $source, B>>::Observer,
                    >,
                S: SourceSubscriber<TupleIndex<$position>, $source, B>,
                <S as SourceSubscriber<TupleIndex<$position>, $source, B>>::Observer: Observer<
                    <$source as ObservableTypes>::Item,
                    <$source as ObservableTypes>::Error,
                >,
            )+
        {
            fn subscribe_each(&mut self, subscriber: &mut S) -> ControlFlow<B> {
                $(
                    if let Some(source) = self.$position.take() {
                        subscribe_one(subscriber, TupleIndex::<$position>, source)?;
                    }
                )+
                ControlFlow::Continue(())
            }
        }
    };
}

for_each_arity!(impl_subscribe_each);

#[cfg(test)]
mod tests;
