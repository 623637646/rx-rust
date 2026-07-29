use crate::{
    disposable::{
        Disposable, DisposableExt, chain_disposal::ChainDisposal, option_disposal::OptionDisposal,
    },
    observable::{Observable, Subscription},
    observer::{BoxedObserverExt, Observer, Termination, boxed_observer::BoxedObserver},
    utils::{
        subscribe_with_context::{
            self, EventBatch, ModelState, ModelUpdate, SubscriptionContext,
            subscribe_with_context_bound_subscription_retain_state_on_stop,
        },
        types::{MaybeSend, MutableBool, MutableBoolHelper, Shared},
    },
};
use educe::Educe;
use slotmap::{DefaultKey, SlotMap};
use std::convert::Infallible;

/// Periodically subdivides items from an Observable into Observable windows.
///
/// A new window is emitted whenever the `boundary` Observable emits an item.
/// Completing the `boundary` stops future window rotation without terminating the current window
/// or the outer Observable.
/// See <https://reactivex.io/documentation/operators/window.html>
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::ObservableExt,
///     observer::{Observer, Termination},
///     operators::transforming::window::Window,
///     subject::publish_subject::PublishSubject,
/// };
/// use std::{convert::Infallible, sync::{Arc, Mutex}};
///
/// let windows = Arc::new(Mutex::new(Vec::<Vec<i32>>::new()));
/// let terminations = Arc::new(Mutex::new(Vec::new()));
/// let inner_subscriptions = Arc::new(Mutex::new(Vec::new()));
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
    pub fn new<'or, T, E>(source: OE, boundary: OE1) -> Self
    where
        OE: Observable<'or, T, E>,
        OE1: Observable<'or, (), Infallible>,
    {
        Self { source, boundary }
    }
}

impl<'or, T, E, OE, OE1> Observable<'or, InnerObservable<'or, T, E, OE::D, OE1::D>, E>
    for Window<OE, OE1>
where
    T: MaybeSend + 'or,
    E: Clone + MaybeSend + 'or,
    OE: Observable<'or, T, E>,
    OE::D: MaybeSend + 'or,
    OE1: Observable<'or, (), Infallible>,
    OE1::D: MaybeSend + 'or,
{
    type D = subscribe_with_context::BoundSubscriptionDisposal<'or>;

    fn subscribe(
        self,
        observer: impl Observer<InnerObservable<'or, T, E, OE::D, OE1::D>, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let observer = DelegateObserver {
            outer_observer: observer.into_boxed(),
            inner_observer: None,
        };
        let model = Model::new();
        subscribe_with_context_bound_subscription_retain_state_on_stop(
            observer,
            model,
            |context| {
                let mut boundary_observer = BoundaryObserver(context.clone());
                boundary_observer.on_next(());
                let boundary_subscription = self.boundary.subscribe(boundary_observer);
                let source_subscription = self.source.subscribe(SourceObserver(context));
                boundary_subscription.preceded_by_bound(source_subscription)
            },
            |model| std::mem::take(&mut model.buffered_windows),
        )
    }
}

struct BufferedWindow<T, E> {
    values: Vec<T>,
    termination: Option<Termination<E>>,
}

impl<T, E> BufferedWindow<T, E> {
    fn open() -> Self {
        Self {
            values: Vec::new(),
            termination: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WindowState {
    /// No window currently accepts source values.
    Vacant,
    /// The window was emitted, but no inner observer has subscribed yet.
    Pending(DefaultKey),
    /// The inner observer is attached, so source values can be forwarded directly.
    Subscribed(DefaultKey),
}

type BufferedWindows<T, E> = SlotMap<DefaultKey, BufferedWindow<T, E>>;

struct Model<T, E> {
    buffered_windows: BufferedWindows<T, E>,
    window_state: WindowState,
}

impl<T, E> Model<T, E> {
    fn new() -> Self {
        Self {
            buffered_windows: SlotMap::new(),
            window_state: WindowState::Vacant,
        }
    }

    fn open_window(&mut self) -> (DefaultKey, WindowState) {
        let key = self.buffered_windows.insert(BufferedWindow::open());
        let previous_state = std::mem::replace(&mut self.window_state, WindowState::Pending(key));
        (key, previous_state)
    }

    fn buffered_window_mut(&mut self, key: DefaultKey) -> &mut BufferedWindow<T, E> {
        self.buffered_windows
            .get_mut(key)
            .expect("buffered window must exist")
    }
}

type WindowContext<'or, T, E, D1, D2> = SubscriptionContext<
    DelegateAction<'or, T, E, D1, D2>,
    E,
    DelegateObserver<'or, T, E, D1, D2>,
    Model<T, E>,
    ChainDisposal<D1, D2>,
    BufferedWindows<T, E>,
>;

struct SourceObserver<'or, T, E, D1, D2>(WindowContext<'or, T, E, D1, D2>)
where
    D1: Disposable,
    D2: Disposable;

impl<'or, T, E, D1, D2> Observer<T, E> for SourceObserver<'or, T, E, D1, D2>
where
    E: Clone,
    D1: Disposable,
    D2: Disposable,
{
    fn on_next(&mut self, value: T) {
        let _ = self.0.try_update_model(|model| match model.window_state {
            WindowState::Subscribed(_) => ModelUpdate::empty()
                .with_next_event(DelegateAction::ForwardValue(value))
                .without_drop_outside(),
            WindowState::Pending(key) => {
                model.buffered_window_mut(key).values.push(value);
                ModelUpdate::empty().without_events().without_drop_outside()
            }
            WindowState::Vacant => ModelUpdate::empty()
                .without_events()
                .with_drop_outside(value),
        });
    }

    fn on_termination(self, termination: Termination<E>) {
        let _ = self.0.try_update_model(|model| {
            match std::mem::replace(&mut model.window_state, WindowState::Vacant) {
                WindowState::Pending(key) => {
                    model.buffered_window_mut(key).termination = Some(termination.clone());
                    ModelUpdate::empty().with_termination_event(termination)
                }
                WindowState::Subscribed(_) => ModelUpdate::empty()
                    .with_next_and_termination_events(
                        DelegateAction::TerminateInner(termination.clone()),
                        termination,
                    ),
                WindowState::Vacant => ModelUpdate::empty().with_termination_event(termination),
            }
        });
    }
}

struct BoundaryObserver<'or, T, E, D1, D2>(WindowContext<'or, T, E, D1, D2>)
where
    D1: Disposable,
    D2: Disposable;

impl<'or, T, E, D1, D2> Observer<(), Infallible> for BoundaryObserver<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn on_next(&mut self, _: ()) {
        let _ = self.0.try_update_model(|model| {
            let (key, previous_state) = model.open_window();

            let emit_window = DelegateAction::EmitWindow(InnerObservable {
                context: Some(self.0.clone()),
                key,
            });
            let events = match previous_state {
                WindowState::Pending(key) => {
                    model.buffered_window_mut(key).termination = Some(Termination::Completed);
                    EventBatch::Next(emit_window)
                }
                WindowState::Subscribed(_) => EventBatch::NextBatch(vec![
                    DelegateAction::TerminateInner(Termination::Completed),
                    emit_window,
                ]),
                WindowState::Vacant => EventBatch::Next(emit_window),
            };

            ModelUpdate::empty().with_events(events)
        });
    }

    fn on_termination(self, _: Termination<Infallible>) {}
}

pub struct InnerObservable<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    context: Option<WindowContext<'or, T, E, D1, D2>>,
    key: DefaultKey,
}

impl<T, E, D1, D2> Drop for InnerObservable<'_, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn drop(&mut self) {
        let Some(context) = self.context.take() else {
            return;
        };
        let key = self.key;
        context.update_model_or_retained_state(|model| {
            let buffered_window = match model {
                ModelState::Active(model) => {
                    let buffered_window = model.buffered_windows.remove(key);
                    if model.window_state == WindowState::Pending(key) {
                        model.window_state = WindowState::Vacant;
                    }
                    buffered_window
                }
                ModelState::Stopped(buffered_windows) => buffered_windows.remove(key),
            };
            ModelUpdate::empty().with_drop_outside(buffered_window)
        });
    }
}

impl<'or, T, E, D1, D2> Observable<'or, T, E> for InnerObservable<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    type D = OptionDisposal<InnerDisposal<'or, T, E, D1, D2>>;

    fn subscribe(
        mut self,
        observer: impl Observer<T, E> + MaybeSend + 'or,
    ) -> Subscription<Self::D> {
        let context = self
            .context
            .take()
            .expect("inner observable context must exist");
        let key = self.key;
        let is_disposed = Shared::new(MutableBool::new(false));
        let result = context.update_model_or_retained_state(|model| match model {
            ModelState::Active(model) => {
                if model.window_state == WindowState::Pending(key) {
                    let buffered_window = model
                        .buffered_windows
                        .remove(key)
                        .expect("pending window must exist");
                    debug_assert!(buffered_window.termination.is_none());
                    model.window_state = WindowState::Subscribed(key);

                    let attach_inner_observer = DelegateAction::AttachInnerObserver(
                        observer.into_boxed(),
                        is_disposed.clone(),
                    );
                    if buffered_window.values.is_empty() {
                        ModelUpdate::new(None).with_next_event(attach_inner_observer)
                    } else {
                        let mut actions = Vec::with_capacity(buffered_window.values.len() + 1);
                        actions.push(attach_inner_observer);
                        actions.extend(
                            buffered_window
                                .values
                                .into_iter()
                                .map(DelegateAction::ForwardValue),
                        );
                        ModelUpdate::new(None).with_events(EventBatch::NextBatch(actions))
                    }
                } else {
                    ModelUpdate::new(Some((
                        observer,
                        model
                            .buffered_windows
                            .remove(key)
                            .expect("buffered window must exist"),
                    )))
                    .without_events()
                }
            }
            ModelState::Stopped(buffered_windows) => ModelUpdate::new(Some((
                observer,
                buffered_windows
                    .remove(key)
                    .expect("buffered window must exist"),
            )))
            .without_events(),
        });
        if let Some((mut observer, buffered_window)) = result {
            for item in buffered_window.values {
                observer.on_next(item);
            }
            if let Some(termination) = buffered_window.termination {
                observer.on_termination(termination);
            }
            OptionDisposal::none().into_subscription()
        } else {
            InnerDisposal {
                context,
                key,
                is_disposed,
            }
            .into_option()
            .into_subscription()
        }
    }
}

pub struct InnerDisposal<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    context: WindowContext<'or, T, E, D1, D2>,
    key: DefaultKey,
    is_disposed: Shared<MutableBool>,
}

impl<'or, T, E, D1, D2> Disposable for InnerDisposal<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn dispose(self) {
        self.is_disposed.write(true);
        let _ = self.context.try_update_model(|model| {
            if model.window_state == WindowState::Subscribed(self.key) {
                model.window_state = WindowState::Vacant;
                ModelUpdate::empty().with_next_event(DelegateAction::DetachInnerObserver)
            } else {
                ModelUpdate::empty().without_events()
            }
        });
    }
}

enum DelegateAction<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    ForwardValue(T),
    EmitWindow(InnerObservable<'or, T, E, D1, D2>),
    AttachInnerObserver(BoxedObserver<'or, T, E>, Shared<MutableBool>),
    TerminateInner(Termination<E>),
    DetachInnerObserver,
}

struct AttachedInnerObserver<'or, T, E> {
    observer: BoxedObserver<'or, T, E>,
    is_disposed: Shared<MutableBool>,
}

struct DelegateObserver<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    outer_observer: BoxedObserver<'or, InnerObservable<'or, T, E, D1, D2>, E>,
    inner_observer: Option<AttachedInnerObserver<'or, T, E>>,
}

impl<'or, T, E, D1, D2> Observer<DelegateAction<'or, T, E, D1, D2>, E>
    for DelegateObserver<'or, T, E, D1, D2>
where
    D1: Disposable,
    D2: Disposable,
{
    fn on_next(&mut self, action: DelegateAction<'or, T, E, D1, D2>) {
        match action {
            DelegateAction::ForwardValue(value) => {
                let inner_observer = self
                    .inner_observer
                    .as_mut()
                    .expect("subscribed window must have an inner observer");
                if !inner_observer.is_disposed.read() {
                    inner_observer.observer.on_next(value);
                }
            }
            DelegateAction::EmitWindow(inner_observable) => {
                debug_assert!(self.inner_observer.is_none());
                self.outer_observer.on_next(inner_observable);
            }
            DelegateAction::AttachInnerObserver(observer, is_disposed) => {
                debug_assert!(self.inner_observer.is_none());
                self.inner_observer = Some(AttachedInnerObserver {
                    observer,
                    is_disposed,
                });
            }
            DelegateAction::TerminateInner(termination) => {
                let inner_observer = self
                    .inner_observer
                    .take()
                    .expect("subscribed window must have an inner observer");
                if !inner_observer.is_disposed.read() {
                    inner_observer.observer.on_termination(termination);
                }
            }
            DelegateAction::DetachInnerObserver => {
                let inner_observer = self
                    .inner_observer
                    .take()
                    .expect("subscribed window must have an inner observer");
                drop(inner_observer);
            }
        };
    }

    fn on_termination(self, termination: Termination<E>) {
        debug_assert!(self.inner_observer.is_none());
        self.outer_observer.on_termination(termination);
    }
}
