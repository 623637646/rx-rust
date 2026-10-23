//! The [`Buffer`] operator, behind
//! [`ObservableExt::buffer`](crate::observable::ObservableExt::buffer).

use crate::{
    delegate_disposal,
    disposable::{Disposable, chain_disposal::ChainDisposal, dispose_on_drop::DisposeOnDrop},
    observable::{ChainDisposalOf, JoinedModeOf, Observable, ObservableTypes},
    observer::{Flow, Observer, Termination},
    thread_mode::ThreadMode,
    utils::{
        serialized_delivery::UpdateOutcome,
        subscribe_with_context::{self, SubscriptionContext, subscribe_with_context},
        take_leaving_room,
    },
};
use educe::Educe;

/// Gathers items from an observable into bundles and emits these bundles as `Vec<T>`, whenever a
/// `boundary` observable emits an item.
/// See <https://reactivex.io/documentation/operators/buffer.html>
///
/// Terminating the `boundary` terminates the outer Observable: completing it emits the pending
/// bundle when non-empty and then completes, while an error from it discards the pending bundle
/// and errors. Note that this differs from
/// [`Window`](crate::operators::transforming::window::Window), where completing the `boundary`
/// only stops rotation; a buffer has no consumer to release items to, so leaving it open after
/// the `boundary` completed would accumulate items without bound.
///
/// # Relation to `Window`
///
/// Conceptually a buffer is a window whose items are collected into a `Vec`, that is
/// `source.window(boundary).concat_map(|window| window.to_vec())`. It is implemented on its own
/// rather than as that composition, because the two are not equivalent:
///
/// - Completing the `boundary` terminates a buffer, as described above, but only stops the
///   rotation of a window.
/// - Completing the `source` with an empty pending bundle emits nothing for a buffer, whereas the
///   composition emits a trailing empty `Vec`, because collecting the empty open window yields
///   the initial value.
/// - [`Window`](crate::operators::transforming::window::Window) requires `E: Clone`, since an
///   error is delivered to both the current window and the outer observable. A buffer has no
///   window to deliver to and so places no such bound on `E`.
///
/// The composition also pays for machinery a buffer does not need: a subject per window, and one
/// queued action per item. Where the three points above do not matter, the equivalence holds; the
/// `test_equivalent_to_window_and_collect` test in `tests/buffer.rs` pins it down, alongside two
/// tests that pin down the divergences.
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::buffer::Buffer,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::convert::Infallible;
///
/// let mut values = Vec::new();
/// let mut source = PublishSubject::<i32, Infallible, _>::local();
/// let mut boundary = PublishSubject::<(), Infallible, _>::local();
///
/// let subscription = Buffer::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     |value| values.push(value),
///     |termination| assert_eq!(termination, Termination::Completed),
/// );
///
/// let _ = source.on_next(1);
/// let _ = source.on_next(2);
/// let _ = boundary.on_next(());
/// let _ = source.on_next(3);
/// source.on_termination(Termination::Completed); // Emits the pending buffer.
///
/// drop((subscription, boundary));
/// assert_eq!(values, [vec![1, 2], vec![3]]);
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Buffer<OE, OE1> {
    /// Creates a [`Buffer`] over `source`;
    /// [`ObservableExt::buffer`](crate::observable::ObservableExt::buffer) is the fluent form.
    pub fn new<T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: ObservableTypes<Item = T, Error = E>,
        OE1: ObservableTypes<Item = (), Error = E>,
    {
        Self { source, boundary }
    }
}

delegate_disposal!(
    Disposal<M, T, E, D, D1>,
    subscribe_with_context::Disposal<M, Vec<T>, E, Vec<T>, ChainDisposal<D, D1>>,
    where M: ThreadMode, D: Disposable, D1: Disposable
);

impl<T, E, OE, OE1> ObservableTypes for Buffer<OE, OE1>
where
    OE: ObservableTypes<Item = T, Error = E>,
    OE1: ObservableTypes<Item = (), Error = E>,
{
    type Item = Vec<T>;
    type Error = E;
    type Mode = JoinedModeOf<OE, OE1>;
    type Disposal = Disposal<JoinedModeOf<OE, OE1>, T, E, OE::Disposal, OE1::Disposal>;
}

impl<T, E, OE, OE1, OR> Observable<OR> for Buffer<OE, OE1>
where
    OR: Observer<Vec<T>, E>,
    OE: Observable<
            BufferObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = T,
            Error = E,
        >,
    OE1: Observable<
            BoundaryObserver<JoinedModeOf<OE, OE1>, T, E, OR, ChainDisposalOf<OE, OE1>>,
            Item = (),
            Error = E,
        >,
{
    fn subscribe(self, observer: OR) -> DisposeOnDrop<Self::Disposal> {
        subscribe_with_context(observer, Vec::new(), |context| {
            let subscription_1 = self.boundary.subscribe(BoundaryObserver(context.clone()));
            let subscription_2 = self.source.subscribe(BufferObserver(context));
            subscription_1.preceded_by_wrapped(subscription_2)
        })
        .map_inner_into()
    }
}

#[doc(hidden)]
pub struct BufferObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<T, E> for BufferObserver<M, T, E, OR, D>
where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) -> Flow {
        self.0.update_flow(|values| {
            values.push(value);
            UpdateOutcome::empty()
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

#[doc(hidden)]
pub struct BoundaryObserver<M: ThreadMode, T, E, OR, D: Disposable>(
    SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
);

impl<M: ThreadMode, T, E, OR, D> Observer<(), E> for BoundaryObserver<M, T, E, OR, D>
where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) -> Flow {
        self.0.update_flow(|values| {
            UpdateOutcome::empty().with_next_event(take_leaving_room(values, values.len()))
        })
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

fn terminate<M: ThreadMode, T, E, OR, D>(
    context: SubscriptionContext<M, Vec<T>, E, OR, Vec<T>, D>,
    termination: Termination<E>,
) where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    match termination {
        completion @ Termination::Completed => {
            let _ = context.update(|values| {
                if values.is_empty() {
                    UpdateOutcome::empty().with_termination_event(completion)
                } else {
                    UpdateOutcome::empty()
                        .with_next_and_termination_events(std::mem::take(values), completion)
                }
            });
        }
        error @ Termination::Error(_) => {
            context.send_termination(error);
        }
    }
}
