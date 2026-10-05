//! The [`Create`] source.

use crate::observer::boxed_observer::{IntoBoxedObserver, ObserverMode};
use crate::observer::emitter::Emitter;
use crate::thread_mode::{Local, Shared, ThreadMode};
use crate::utils::MarkerType;
use crate::{
    disposable::Disposable,
    observable::{Observable, ObservableTypes, Subscription},
    observer::{
        Observer,
        boxed_observer::{BoxedObserver, SendBoxedObserver},
    },
};
use educe::Educe;
use std::marker::PhantomData;

/// Creates an Observable from scratch by means of a builder function.
/// See <https://reactivex.io/documentation/operators/create.html>
///
/// The builder is called with the observer at each subscription, emits to it, and returns the
/// subscription that stops what it started (`Subscription::default()` when it is done on return).
///
/// # Examples
/// ```rust
/// use rx_rust::{
///     observable::{ObservableExt, Subscription},
///     observer::{Observer, Termination},
///     operators::creating::create::Create,
/// };
///
/// let mut values = Vec::new();
/// Create::local(|mut emitter| {
///     // A stopped observer takes nothing more, not even a termination: see `Flow`.
///     if emitter.on_next(42).is_continue() {
///         emitter.on_termination(Termination::<()>::Completed);
///     }
///     Subscription::default()
/// })
/// .map(|value| value + 1)
/// .subscribe_with_callback(|value| values.push(value), |_| {});
/// assert_eq!(values, [43]);
/// ```
///
/// # `local` / `shared` or `local_boxed` / `shared_boxed`
///
/// A closure cannot be generic, so its parameter is one concrete type, and that decides what the
/// observable can be subscribed with:
///
/// - [`local`](Create::local) / [`shared`](Create::shared) hand the builder an [`Emitter`]: the
///   downstream observer itself, unboxed and statically dispatched. The emitter's observer type is
///   inferred from the one subscription it is used with, so such a `Create` subscribes that one
///   observer type only: build and subscribe it in the same function. It cannot be returned without
///   naming the downstream observer, nor subscribed by two different observers.
/// - [`local_boxed`](Create::local_boxed) / [`shared_boxed`](Create::shared_boxed) make a
///   `Create<.., true>`, whose builder gets a [`BoxedObserver`] / [`SendBoxedObserver`], at the
///   cost of one allocation per subscription and a virtual call per event. It subscribes any
///   observer, so it is an ordinary value: return it, store it, clone it, subscribe it several
///   times. `retry` and `catch` callbacks that return a `Create` need this form (or
///   [`into_boxed`](crate::observable::ObservableExt::into_boxed)).
///
/// ```rust
/// use rx_rust::{
///     observable::{ObservableExt, Subscription},
///     observer::{Observer, Termination, boxed_observer::BoxedObserver},
///     operators::creating::create::Create,
///     thread_mode::Local,
/// };
///
/// fn ones<'a>() -> Create<
///     i32,
///     (),
///     (),
///     impl FnOnce(BoxedObserver<'a, i32, ()>) -> Subscription<()> + Clone,
///     Local,
///     true,
/// > {
///     Create::local_boxed(|mut observer| {
///         if observer.on_next(1).is_continue() {
///             observer.on_termination(Termination::Completed);
///         }
///         Subscription::default()
///     })
/// }
///
/// let mut values = Vec::new();
/// ones().map(|value| value * 10).subscribe_with_callback(|value| values.push(value), |_| {});
/// ones().subscribe_with_callback(|value| values.push(value), |_| {});
/// assert_eq!(values, [10, 1]);
/// ```
///
/// The unboxed form subscribes one observer type only:
///
/// ```compile_fail
/// use rx_rust::{
///     observable::{ObservableExt, Subscription},
///     observer::{Observer, Termination},
///     operators::creating::create::Create,
/// };
///
/// let source = Create::local(|mut emitter| {
///     let _ = emitter.on_next(1);
///     emitter.on_termination(Termination::<()>::Completed);
///     Subscription::default()
/// });
/// source.clone().map(|value: i32| value + 1).subscribe_with_callback(|_| {}, |_| {});
/// source.subscribe_with_callback(|_| {}, |_| {}); // A different observer type.
/// ```
///
/// # Thread mode
///
/// Where the builder emits from is known to its author only, so the thread mode is declared here,
/// with no default: `local` when it emits only on the subscribing thread, synchronously, and
/// `shared` when it may emit from another one. A `Local` builder gets an observer that is not
/// `Send` (an [`Emitter`] carrying the `!Send` [`Local`] as a `PhantomData`, or a
/// [`BoxedObserver`]), so moving it to another thread does not compile, whatever the observer
/// downstream is. A `Shared` emitter is as `Send` as the downstream observer; `shared_boxed`
/// requires it to be `Send`.
///
/// ```compile_fail
/// use rx_rust::{
///     observable::{ObservableExt, Subscription},
///     observer::{Observer, Termination},
///     operators::creating::create::Create,
/// };
///
/// let _subscription = Create::local(|mut emitter| {
///     std::thread::spawn(move || {
///         let _ = emitter.on_next(1);
///         emitter.on_termination(Termination::<()>::Completed);
///     });
///     Subscription::default()
/// })
/// .subscribe_with_callback(|_: i32| {}, |_| {});
/// ```
///
/// ```rust
/// use rx_rust::{
///     observable::{ObservableExt, Subscription},
///     observer::{Observer, Termination},
///     operators::creating::create::Create,
/// };
///
/// let _subscription = Create::shared(|mut emitter| {
///     std::thread::spawn(move || {
///         if emitter.on_next(1).is_continue() {
///             emitter.on_termination(Termination::<()>::Completed);
///         }
///     })
///     .join()
///     .unwrap();
///     Subscription::default()
/// })
/// .subscribe_with_callback(|_: i32| {}, |_| {});
/// ```
///
/// # `BOXED`
///
/// Both forms are this one type: `BOXED` is `false` for [`local`](Create::local) /
/// [`shared`](Create::shared), and `true` for [`local_boxed`](Create::local_boxed) /
/// [`shared_boxed`](Create::shared_boxed). It only picks the [`Observable`] impl: the builder gets
/// an [`Emitter`] under `false` and the boxed observer of the mode under `true`. The lifetime that
/// boxed observer may borrow for is not a parameter: it appears only in the builder's signature,
/// and each subscription picks it.
#[derive(Educe)]
#[educe(Debug, Clone(bound(F: Clone)))]
pub struct Create<T, E, D, F, M, const BOXED: bool = false> {
    #[educe(Debug(ignore))]
    builder: F,
    /// What the builder emits and returns, and how; a [`MarkerType`] so that only the builder
    /// decides whether a `Create` is `Send`.
    #[educe(Debug(ignore))]
    _marker: MarkerType<(T, E, D, M)>,
}

impl<T, E, D, F> Create<T, E, D, F, Local> {
    /// Creates a [`Create`] whose builder emits only on the subscribing thread, synchronously, to
    /// the unboxed downstream observer. See the [type documentation](Create) for when to prefer
    /// [`local_boxed`](Create::local_boxed).
    ///
    /// The builder returns the subscription that stops what it started; one that is done when it
    /// returns gives back `Subscription::default()`. A `Subscription` rather than a closure makes
    /// it easy to wrap another observable. `OR` is not stored: it only gives the builder its
    /// expected signature where the `Create` is made.
    pub fn local<OR: Observer<T, E>>(builder: F) -> Self
    where
        D: Disposable,
        F: FnOnce(Emitter<OR, Local>) -> Subscription<D>,
    {
        Self::with_builder(builder)
    }

    /// Like [`local`](Create::local), but the builder gets a [`BoxedObserver`], so the `Create`
    /// subscribes any observer: it can be returned, stored, cloned and subscribed several times.
    pub fn local_boxed<'a>(builder: F) -> Create<T, E, D, F, Local, true>
    where
        D: Disposable,
        F: FnOnce(BoxedObserver<'a, T, E>) -> Subscription<D>,
    {
        Create::with_builder(builder)
    }
}

impl<T, E, D, F> Create<T, E, D, F, Shared> {
    /// Creates a [`Create`] whose builder may emit from another thread, to the unboxed downstream
    /// observer. The observer only needs to be `Send` if the builder actually sends it.
    pub fn shared<OR: Observer<T, E>>(builder: F) -> Self
    where
        D: Disposable,
        F: FnOnce(Emitter<OR, Shared>) -> Subscription<D>,
    {
        Self::with_builder(builder)
    }

    /// Like [`shared`](Create::shared), but the builder gets a [`SendBoxedObserver`], so the
    /// `Create` subscribes any `Send` observer: it can be returned, stored, cloned and subscribed
    /// several times.
    pub fn shared_boxed<'a>(builder: F) -> Create<T, E, D, F, Shared, true>
    where
        D: Disposable,
        F: FnOnce(SendBoxedObserver<'a, T, E>) -> Subscription<D>,
    {
        Create::with_builder(builder)
    }
}

impl<T, E, D, F, M, const BOXED: bool> Create<T, E, D, F, M, BOXED> {
    fn with_builder(builder: F) -> Self {
        Self {
            builder,
            _marker: PhantomData,
        }
    }
}

impl<T, E, D: Disposable, F, M: ThreadMode, const BOXED: bool> ObservableTypes
    for Create<T, E, D, F, M, BOXED>
{
    type Item = T;
    type Error = E;
    type Mode = M;
    /// Whatever the builder returns.
    type D = D;
}

impl<T, E, D, F, M, OR> Observable<OR> for Create<T, E, D, F, M, false>
where
    D: Disposable,
    M: ThreadMode,
    OR: Observer<T, E>,
    F: FnOnce(Emitter<OR, M>) -> Subscription<D>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        (self.builder)(Emitter::new(observer))
    }
}

impl<'a, T, E, D, F, M, OR> Observable<OR> for Create<T, E, D, F, M, true>
where
    D: Disposable,
    M: ObserverMode,
    OR: IntoBoxedObserver<'a, T, E, M>,
    F: FnOnce(M::BoxedObserver<'a, T, E>) -> Subscription<D>,
{
    fn subscribe(self, observer: OR) -> Subscription<Self::D> {
        (self.builder)(M::boxed(observer))
    }
}
