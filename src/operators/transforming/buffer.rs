use crate::utils::subscribe_with_context::{
    BoundSubscriptionDisposal, ModelUpdate, SubscriptionContext,
    subscribe_with_context_bound_subscription,
};
use crate::utils::types::MaybeSend;
use crate::{
    disposable::Disposable,
    observable::Observable,
    observable::Subscription,
    observer::{Observer, Termination},
};
use educe::Educe;

/// Periodically gathers items from an Observable into bundles and emits these bundles as `Vec<T>`, when a `boundary` Observable emits an item.
///
/// Terminating the `boundary` terminates the outer Observable: completing it emits the pending
/// bundle when non-empty and then completes, while an error from it discards the pending bundle
/// and errors. Note that this differs from
/// [`Window`](crate::operators::transforming::window::Window), where completing the `boundary`
/// only stops rotation; a buffer has no consumer to release items to, so leaving it open after
/// the `boundary` completed would accumulate items without bound.
/// See <https://reactivex.io/documentation/operators/buffer.html>
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
///   error is delivered to both the current window and the outer Observable. A buffer has no
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
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let values = Arc::new(Mutex::new(Vec::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
///
/// let mut source: PublishSubject<'_, i32, Infallible> = PublishSubject::default();
/// let mut boundary: PublishSubject<'_, (), Infallible> = PublishSubject::default();
/// let values_observer = Arc::clone(&values);
/// let terminations_observer = Arc::clone(&terminations);
///
/// let subscription = Buffer::new(source.clone(), boundary.clone()).subscribe_with_callback(
///     move |value| values_observer.lock().unwrap().push(value),
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
///
/// assert_eq!(&*values.lock().unwrap(), &[vec![1, 2], vec![3]]);
/// assert_eq!(
///     &*terminations.lock().unwrap(),
///     &[Termination::Completed]
/// );
/// ```
#[derive(Educe)]
#[educe(Debug, Clone)]
pub struct Buffer<OE, OE1> {
    source: OE,
    boundary: OE1,
}

impl<OE, OE1> Buffer<OE, OE1> {
    pub fn new<'or, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), E>,
    {
        Self { source, boundary }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, Vec<T>, E> for Buffer<OE, OE1>
where
    T: MaybeSend + 'or,
    E: MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), E>,
    OE1::D: MaybeSend + 'or,
{
    type D = BoundSubscriptionDisposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<Vec<T>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        subscribe_with_context_bound_subscription(observer, Vec::new(), |context| {
            let subscription_1 = self.boundary.subscribe(BoundaryObserver(context.clone()));
            let subscription_2 = self.source.subscribe(BufferObserver(context));
            subscription_1.preceded_by_bound(subscription_2)
        })
    }
}

struct BufferObserver<T, E, OR, D: Disposable>(SubscriptionContext<Vec<T>, E, OR, Vec<T>, D>);

impl<T, E, OR, D> Observer<T, E> for BufferObserver<T, E, OR, D>
where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.try_update_model(|values| {
            values.push(value);
            ModelUpdate::empty()
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

struct BoundaryObserver<T, E, OR, D: Disposable>(SubscriptionContext<Vec<T>, E, OR, Vec<T>, D>);

impl<T, E, OR, D> Observer<(), E> for BoundaryObserver<T, E, OR, D>
where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    fn on_next(&mut self, _: ()) {
        let _ = self.0.try_update_model(|values| {
            ModelUpdate::empty()
                .with_next_event(std::mem::replace(values, Vec::with_capacity(values.len())))
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        terminate(self.0, termination);
    }
}

fn terminate<T, E, OR, D>(
    context: SubscriptionContext<Vec<T>, E, OR, Vec<T>, D>,
    termination: Termination<E>,
) where
    OR: Observer<Vec<T>, E>,
    D: Disposable,
{
    match termination {
        completion @ Termination::Completed => {
            let _ = context.try_update_model(|values| {
                if values.is_empty() {
                    ModelUpdate::empty().with_termination_event(completion)
                } else {
                    ModelUpdate::empty()
                        .with_next_and_termination_events(std::mem::take(values), completion)
                }
            });
        }
        error @ Termination::Error(_) => context.send_termination(error),
    }
}
