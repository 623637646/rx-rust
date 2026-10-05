# 0002: Observable types and the thread mode

Status: adopted. The observer is a type parameter of `Observable`, the disposal is not, the thread
mode is a type, schedulers run nameable tasks, and erasure is explicit. The design was first
rejected (see [History](#history)), then reached in two steps.

Reference: the `rust-test` prototype, a separate repository (`623637646/rust-test`), and its
decisions `docs/decisions/0003-disposal-on-observable-types.md` and
`docs/decisions/0004-subject-and-type-erasure.md`. Everything this crate took from it is restated
here, so this record does not need the prototype to be read.

## The problem

The thread mode used to be a build-wide `cfg`: `single-threaded` swapped `Shared` / `Mutable` /
`MaybeSend` in `utils::types` between `Rc` / `RefCell` / no bound and `Arc` / `Mutex` / `Send`,
and compiled the multi-threaded schedulers out. That made the feature non-additive, which Cargo's
feature unification does not tolerate: crate A asking for `single-threaded` and crate B for
`tokio-scheduler` was a compile error, and the final binary's author never got to choose. And in
the multi-threaded build every observer and closure had to be `Send`, even in a chain that never
left its thread.

The root cause was not the features. `Send` sat in trait signatures —
`Observable::subscribe(observer: impl Observer + MaybeSend)`, `Scheduler::spawn_future` — and a
trait method cannot carry a different bound per implementor, so the mode was necessarily global
unless it moved into the type system.

## The design

```rust
pub trait ObservableTypes {
    type Item;
    type Error;
    type Mode: ThreadMode;   // Local or Shared
    type D: Disposable;      // never names the observer
}

pub trait Observable<OR>: ObservableTypes {
    fn subscribe(self, observer: OR) -> Subscription<Self::D>;
}

pub trait ThreadMode: 'static {                     // sealed; Local / Shared are never instantiated
    type Ptr<T>: Clone + MutableHelper<Value = T>;    // Rc<RefCell<T>>   / Arc<Mutex<T>>
    type Weak<T>: Clone;                              // rc::Weak         / sync::Weak
    type Flag: Clone + Default + MutableBoolHelper;  // Rc<Cell<bool>>   / Arc<AtomicBool>
    type Or<M: ThreadMode>: ThreadMode;               // Local.Or<M> = M, Shared.Or<M> = Shared
    fn ptr<T>(value: T) -> Self::Ptr<T>;
    fn downgrade<T>(ptr: &Self::Ptr<T>) -> Self::Weak<T>;
    fn upgrade<T>(weak: &Self::Weak<T>) -> Option<Self::Ptr<T>>;
}
pub struct Local(PhantomData<*const ()>);             // !Send + !Sync
pub struct Shared(());

// In `observer::boxed_observer`: what a mode means for an observer. `thread_mode` (with its locks,
// `thread_mode::mutable`) is the bottom layer and depends on nothing else in the crate.
pub trait ObserverMode: ThreadMode + Sized {
    type BoxedObserver<'a, T, E>: Observer<T, E>;    // BoxedObserver / SendBoxedObserver
    fn boxed<'a, T, E, OR: IntoBoxedObserver<'a, T, E, Self>>(o: OR) -> Self::BoxedObserver<'a, T, E>;
}

pub trait SchedulerTypes { type Mode: ThreadMode; type D: Disposable; }
pub trait Scheduler<TC, P = ()>: SchedulerTypes + Clone {
    fn run_task(&self, task: Task<TC, P>, delay: Option<Duration>) -> Subscription<Self::D>;
}
```

1. **The observer is a trait parameter.** Each implementation states its own bounds for the
   concrete observer it receives: only an operator that boxes its observer (a context, a subject,
   `create`, a boxed observable) or hands it to a scheduler asks for `Send` or `'static`. A chain
   of synchronous operators takes a non-`Send` observer and is monomorphized and inlined as before.
   Each implementation names the observer it subscribes its source with, e.g.
   `OE: Observable<MapObserver<OR, F>, Item = T0>`; those observer types are `pub`, their fields
   are not. They must be public because the non-boxed `create`, `hook_on_subscription` and
   `hook_on_termination` give the downstream observer to a user closure, and a value of a private
   type in user code is an error; the where-clause naming them alone would not need it, since
   privacy does not check the where-clauses of trait impls. `ObservableExt` needs only `ObservableTypes`, so the fluent API did not change.
2. **The disposal lives on `ObservableTypes`, which does not know the observer.** An operator that
   owns two subscriptions, such as `merge`, subscribes each source with an observer whose type
   contains the other source's disposal. If a disposal could depend on its observer, the second
   source's disposal would appear in its own definition.
3. **No `'or`, and `Item` / `Error` are associated types.** `'or` existed only because a context's
   disposal boxed a handle to the context, observer included. The disposal is now a
   `ContextDisposal`: `SerializedDelivery` is split into the observer cell and an
   observer-independent core (the model, the resources, the stop flag), and the disposal holds only
   the core plus a `DeliveryStop` that releases the observer. It needs no box and no lifetime.
4. **The context helpers keep their contract.** A dispose stops the context before it returns, and
   nothing is delivered after it. The observer is released when the last handle to the context
   goes, which the source releases as it is disposed, so in practice still before `dispose`
   returns. A scheduled task that must deliver after its source has terminated (`delay`,
   `observe_on`) holds a `PromotableWeakContext` instead of a weak handle: the operator promotes it
   when its source terminates, so the pending values are not lost.
5. **The thread mode is an associated type.** Each source picks its mode (`Create::local` /
   `Create::shared`, `PublishSubject::local()` / `shared()`; value sources such as `Just` are
   `Local`); an operator over several sources takes `Joined<A, B>`; a scheduler operator takes the
   scheduler's `Mode`. State goes behind `M::Ptr<_>`, so a `Local` chain has no `Arc` and no atomic.
   `Local` itself is `!Send + !Sync`, so the `PhantomData<Local>` in the `Emitter` a `Local`
   `Create` hands its builder makes the emitter `!Send`, and the declaration is enforced where it
   is made rather than by whatever state happens to be downstream. Nothing else carries it: a
   `Local` observable may itself be `Send`; moving it before it is subscribed breaks nothing.
6. **Boxing an observer is a capability, not a marker.** An operator that boxes (a subject,
   `create`, the hooks) asks for `OR: IntoBoxedObserver<'a, T, E, M>`, whose `Shared` impl has the
   `Send` bound as its own where-clause, and boxes with `M::boxed(observer)`. Code generic over the
   mode states `M: ObserverMode` there; a concrete mode always has it. Nothing boxes per hop.
7. **Schedulers run nameable tasks.** A `Task<TC, P>` is a context `TC`, an optional payload `P` (a
   future or a stream) and a `fn` handler; `Scheduler<TC, P>` is implemented per runtime with the
   `Send` bounds that runtime needs (`TC: Send` on Tokio, nothing on the local pool). Operators
   define their task context as a named struct, so the bound is stated on a type, not on an `async`
   block. `SchedulerExt` keeps the closure conveniences (`schedule`, `schedule_periodically`,
   `spawn_future`, …) for code that does not need to be generic over the mode.

   Every runtime with a local executor gets a `Local` scheduler next to its `Shared` one
   (`TokioLocalScheduler`, `SmolLocalScheduler`, `LocalPoolScheduler`; not async-std, whose
   `spawn_local` is unstable). Every scheduler is a type of the crate wrapping the runtime's handle
   (`TokioScheduler` a `Handle`, `ThreadPoolScheduler` a `ThreadPool`), rather than the handle
   itself, so that its documentation, its disposal and its behavior stay ours. A scheduler either
   uses the executor of the calling context (Tokio's current `LocalSet`, smol's global executor,
   async-std's) or names one (`from_local_set`, `from_executor`, `from_spawner`). Unlike the
   prototype, an executor that runs only while it is driven is held **weakly**: a task can reach
   its scheduler through a downstream observer, and the executor holds its tasks, so a strong
   handle is a cycle that keeps an executor nobody drives — and its tasks — alive forever. A
   `ThreadPool` is held strongly: its threads always run, so a disposed task is dropped and breaks
   the cycle. Running a task after the executor is gone panics.
8. **Erasure is explicit and says what it keeps.** `into_boxed` / `into_cloneable_boxed` (no
   `Send`) and `into_send_boxed` / `into_send_cloneable_boxed`, plus the `_for::<OR>` version of
   each for a fixed observer type; the lifetimes bound the observer (`'or`), the disposal (`'sub`)
   and the observable (`'oe`).
9. **`Create` offers both builders.** A closure cannot be generic, so its parameter decides what a
   `Create` subscribes. `Create::local` / `shared` hand it an `Emitter<OR, M>`, the unboxed
   downstream observer: zero-cost, but the `Create` subscribes the one observer type it was
   inferred for, so it must be built and subscribed in one function. `Create::local_boxed` /
   `shared_boxed` hand it a `BoxedObserver` / `SendBoxedObserver`: one allocation per subscription
   and a virtual call per event, and an ordinary value that can be returned, stored, cloned and
   subscribed by any observer, which is what a `retry` / `catch` callback returning a `Create`
   needs.

   Both are one type, `Create<T, E, D, F, M, const BOXED: bool = false>`. `BOXED` only picks the
   `Observable` impl; `Clone`, `Debug` and `ObservableTypes` are written once. Without it the two
   impls (`F: FnOnce(Emitter<OR, M>)` and `F: FnOnce(M::BoxedObserver<'a, ..>)`) overlap, since a
   closure type could implement both signatures. A marker wrapping the builder
   (`BoxingBuilder<'a, F>`) told them apart before, but had to be spelled in every return type;
   named marker types (`Unboxed` / `Boxed`) would have needed a name and a home of their own. The
   boxed form is spelled `Create<.., true>`, with no alias, so there is one name for the type.
   There is no lifetime parameter: `'a` appears only in the boxed impl's bounds, which is allowed
   because `Observable<OR>` has no associated type, so each subscription picks it.
   `hook_on_subscription` / `hook_on_termination` follow the same split. `Emitter` lives in
   `observer::emitter`, since it is not specific to `Create`, and `Emitter::new` is public.
10. **The features are additive.** `single-threaded` and `utils::types` are gone; every scheduler
    feature can be enabled together, and a binary can hold `Local` and `Shared` pipelines side by
    side.

## Rules the type system imposes

1. **An observer that subscribes an observable with itself cannot require `Observable<Self>` on
   its own `Observer` implementation.** Proving that implementation would depend on itself, and
   rustc reports `E0275` (overflow). `retry` and `concat_all` (and so `concat_map`) re-subscribe
   that way. They carry a `utils::resubscribe::Resubscribe`, a function pointer
   `fn(OE, Self) -> Subscription<D>` created where the operator is subscribed — where the bound is
   stated and holds — and call it instead. It is public so that a user-written operator can do the
   same. An observer that subscribes with a *different* observer type (`switch`, `merge_all`,
   `catch`, `concat`) needs nothing special.
2. **A bound that mentions a projection writes it qualified.** In
   `OE: Observable<X<<OE as ObservableTypes>::D>>` the shorthand `OE::D` would need the bounds of
   `OE` while they are being computed (`E0391`); the same holds for `<OE as ObservableTypes>::Mode`,
   and across parameters: `merge`'s two bounds each mention the other's disposal.
3. **A stream or future adapter (`into_stream`, `into_future`) keeps its state in `Shared` pointers
   whatever the source's mode**, because the executor that polls it may require `Send`. Every
   synchronous source is `Local`, and many of them (`FromIter`, `Just`) are `Send`: a pointer
   picked from the mode would make the adapter over them `!Send`. A source that really is bound to
   its thread makes the adapter `!Send` by itself, so the `Shared` pointer never lets one cross.

## History

### First attempt: a mode parameter on the old trait — rejected

The first design put `M: Mode` on `Observable<'or, T, E, M>`, `Scheduler` and every operator,
with `subscribe` still taking `impl Observer`. A 155-line prototype (`Just`, `Map`, `ObserveOn`,
two schedulers) compiled and inferred without annotations, with native error messages. It was
rejected for the shape the type system forced on it:

1. **A `Bound<M>` marker cannot imply `Send`.** `impl<X: Send> Bound<Threaded> for X` is coherent,
   but the solver never reasons backwards from `O: Bound<Threaded>` to `O: Send`, so
   `tokio::spawn` still refuses `O`. The marker had to become a capability that boxes.
2. **Code generic over `M` cannot prove `MapObserver<O, F>: Bound<M>`.** A structural impl
   overlaps the blanket one, so the obligation had to be deferred to an impl where-clause naming
   the internal wrapper, and `subscribe` had to take the already-erased `M::BoxedObserver`: one
   `Box<dyn>` per hop at subscribe time, one virtual call per operator per event — a cost the
   single-threaded users, the ones the design was for, would pay most.
3. **Closures and `async` blocks cannot appear in a where-clause**, so every `async` operator would
   have become a hand-written state machine.

It also leaked `where MapObserver<…>: Bound<M>` into the public API of any user code generic over
the mode. The alternatives then on the table were packaging ones: two crates from one source tree
(`im` / `im-rc` style), or a `--cfg` chosen by the top-level binary.

### Why the design works now

Making the observer a trait parameter (point 1) removes the root cause: each
`Observable` impl states its own bounds for the concrete observer it receives, so the per-mode
`Send` requirement is proved at the one place it arises — where an observer is boxed (obstacle 1,
point 6) or a task is handed to a runtime (obstacle 3, point 7) — and is never written as
`MapObserver<…>: Bound<M>` (obstacle 2). The remaining cost is the named task contexts, which
cover only the scheduler operators.

### The intermediate step

The observer became a trait parameter first, on `Observable<'or, T, E, OR>`, with the thread mode
still a `cfg`. That step kept `'or` (a context's disposal still boxed the observer), kept `T` and
`E` as type parameters (as associated types they hung rustc with the bounds of the time) and left
the context helpers unchanged. Points 3 and 4 above removed all three; the hang does not occur with
the current bounds, on stable or on the MSRV (`rust-version` in `Cargo.toml`).
