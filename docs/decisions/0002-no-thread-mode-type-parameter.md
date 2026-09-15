# 0002: No thread-mode type parameter (`Observable<'or, T, E, M>`)

Status: rejected.

## The problem it would solve

The thread mode is a build-wide `cfg`: `single-threaded` swaps `Shared` / `Mutable` / `MaybeSend`
in `utils::types` and `utils::mutable` between `Rc` / `RefCell` / no bound and `Arc` / `Mutex` /
`Send`, and the multi-threaded scheduler modules are compiled out under it. That makes the feature
non-additive, which Cargo's feature unification does not tolerate: if crate A depends on `rx-rust`
with `single-threaded` and crate B with `tokio-scheduler`, the union of features is a compile
error, and the final binary's author never gets to choose. A second consequence is that in the
multi-threaded build every observer and closure must be `Send`, even in a chain that never leaves
the current thread.

The root cause is not the features. `Send` sits in trait signatures —
`Observable::subscribe(observer: impl Observer + MaybeSend)`, `Scheduler::spawn_future` — and a
trait method cannot carry a different bound per implementor, so the mode is necessarily a global
property unless it is moved into the type system. This decision is about the type-system version.

## What it is

Replace the `cfg` with a type parameter `M: Mode` on `Observable`, `Scheduler` and every operator:

```rust
pub trait Mode: 'static {
    type Shared<T>: Clone + Deref<Target = T>;              // Rc      / Arc
    type Mutable<T>: MutableHelper<T>;                       // RefCell / Mutex
    type BoxedObserver<'or, T: 'or>: Observer<T> + 'or;      // Box<dyn Observer> / Box<dyn Observer + Send>
    type BoxedTask: Task + 'static;                          // Box<dyn Task>     / Box<dyn Task + Send>
}
pub struct Local;
pub struct Threaded;
```

Both modes then coexist in one binary, every chain picks its own, and the scheduler features become
plain additive features providing `Scheduler<Threaded>` and/or `Scheduler<Local>` impls (Tokio
could offer both, through `spawn` and `spawn_local`).

A 155-line prototype (`Just`, `Map`, `ObserveOn`, two schedulers) was built to check that the
design compiles and infers. It does. The shape it had to take is the reason it was rejected.

## What the type system forces

Three obstacles, each with exactly one workaround.

1. **A `Bound<M>` marker cannot imply `Send`.** `impl<X: Send> Bound<Threaded> for X` and
   `impl<X> Bound<Local> for X` are coherent, but the solver never reasons backwards from
   `O: Bound<Threaded>` to `O: Send`, so `tokio::spawn` still refuses `O`. `Bound<M>` therefore has
   to be a capability, not a marker: `fn erase_observer(self) -> M::BoxedObserver`, whose
   `Threaded` impl has `X: Send` as a where-clause and can box into `dyn Observer + Send`. After
   erasure the type is concrete and `Send` is derived structurally.

2. **Code generic over `M` cannot prove `MapObserver<O, F>: Bound<M>`.** A structural impl
   (`impl<M, O: Bound<M>, F: Bound<M>> Bound<M> for MapObserver<O, F>`) overlaps the blanket impl
   at `M = Threaded`, and coherence rejects it. The only way out is to defer the obligation to the
   call site as an impl where-clause that names the internal wrapper:
   ```rust
   impl<'or, T, R, OE, F, M: Mode> Observable<'or, R, M> for Map<M, T, OE, F>
   where
       OE: Observable<'or, T, M>,
       MapObserver<M::BoxedObserver<'or, R>, F>: Bound<M>,   // decided where M is concrete
   ```
   Consequently `Observable::subscribe` must take the already-erased `M::BoxedObserver`, not
   `impl Observer`: the wrapper has to be boxed before it is handed upstream, at every hop.

3. **Closures and `async` blocks cannot appear in a where-clause.** Anything that crosses a thread
   — `spawn_future(async move { ... })` — has to become a nameable struct implementing a `Task`
   trait so that `Deliver<M, T, O>: Bound<M>` can be written. Delays, sleeps and
   `RecursionAction` loops move into the concrete `Scheduler<Threaded>` / `Scheduler<Local>`
   impls, where the task type is concrete; operators stop containing `async` at all.

## What was verified in the prototype

- One generic impl per operator serves both modes (`Just`, `Map`, `ObserveOn` were each written
  once).
- Inference needs no annotations in the middle of a chain: operator structs carry `M`, and the
  terminal `subscribe` (`Threaded`) / `subscribe_local` (`Local`) on `ObservableExt` fixes it, so
  `just(1).map(f).observe_on(tokio).subscribe(o)` and
  `just(1).map(move |x| x + *rc).subscribe_local(o)` both work unannotated.
- Errors are the native ones: an `Rc` in a `Threaded` chain reports
  "`Rc<i32>` cannot be sent between threads safely … within `MapObserver<…>`"; a Tokio handle in
  a `Local` chain reports "`Handle: Scheduler<Local>` is not implemented".

## Why not

1. **Every hop boxes and every event takes a virtual call per operator.** Today `impl Observer`
   monomorphizes the whole chain and inlines across operators; obstacle 2 makes
   `subscribe(M::BoxedObserver)` the trait signature, so `map → filter → take` becomes three
   `Box<dyn>` allocations at subscribe time and three indirect calls per `on_next`. There is no
   escape: a non-boxing GAT wrapper would need a per-mode bound on its type parameter, which GATs
   cannot express. The users who care most about this cost are exactly the single-threaded ones.
2. **Every `async` operator is rewritten as a state-machine struct** (obstacle 3), and
   `subscribe_with_context` and the other helpers gain `M`. That is deeper than swapping
   `MaybeSend` for `Bound<M>` in the ~120 files that mention it.
3. **Where-clauses leak internal types into the public API.**
   `where MapObserver<M::BoxedObserver<…>, F>: Bound<M>` shows up in docs and error messages, and
   any user function generic over `M` has to repeat the same clause for every operator it uses.
   Users who pick one mode are unaffected, but the generic case is what the design exists for.
4. **It is a crate-wide breaking change**: `Observable` grows a parameter, `subscribe` splits in
   two, custom `Observable` implementors receive `M::BoxedObserver`, `Scheduler` changes signature,
   `BoxedObservable` / `BoxedObserver` carry `M`.
5. **It does not relax `Send` inside a `Threaded` chain.** `Threaded::BoxedObserver` is
   `dyn Observer + Send` for every operator, whether or not that operator crosses a thread; the only
   way to drop `Send` from a synchronous chain is to build it in `Local` mode. Coexistence of the
   two modes is the most this design can offer on that point, and a per-operator relaxation runs
   into the same trait-signature uniformity that started this.

## Alternatives under the current design

The feature-unification problem is a packaging problem and can be solved at that level without
touching the API:

- **Two crates from one source tree** (`im` / `im-rc` style): `rx-rust` always multi-threaded,
  `rx-rust-local` always single-threaded, both with only additive scheduler features. The two are
  distinct types and coexist in one binary. This also lifts the current exclusion of Tokio from the
  single-threaded build, which can use `spawn_local`.
- **A `--cfg` instead of a feature** (`RUSTFLAGS="--cfg rx_single_threaded"`, the Cargo Book's
  recommendation for unavoidable exclusivity): the top-level binary decides, no intermediate crate
  can. Cheaper, but a dependency written against the single-threaded assumptions still fails to
  build when the top level picks multi-threaded; it moves the failure, not the incompatibility.
