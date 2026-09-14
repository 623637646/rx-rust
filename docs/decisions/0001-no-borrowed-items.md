# 0001: No borrowed items (`type Item<'a>` GAT)

Status: deferred — not planned.

Reference: [rxrust](https://github.com/rxrust/rxrust)'s `ObservableType::Item<'a>`,
`subject_mut_ref` and `multicast_mut_ref` (`src/observable.rs`, `src/subject.rs`).

## What it is

rxrust defines the item type as a generic associated type with a lifetime (GAT, stable since Rust
1.65):

```rust
pub trait ObservableType {
    type Item<'a> where Self: 'a;   // 'a can differ on every emission
    type Err;
}
fn subscribe<F>(self, f: F) where F: for<'a> FnMut(Self::Item<'a>);
```

An observable can then **lend** data it owns to the observer as `&'a T` / `&'a mut T` instead of
handing over ownership. The HRTB (`for<'a>`) requires the closure to work for every `'a`, which is
a promise that it will not keep the reference past the call, so a subject can **re-borrow** the same
`&mut` to N subscribers in turn:

```rust
let c = source.multicast_mut_ref(subject);
c.fork().subscribe(|v: &mut i32| *v += 1);
c.fork().subscribe(|v: &mut i32| *v *= 2);
```

This is the same distinction as `Iterator` vs. a lending iterator. In our `Observable<'or, T, E>`,
`T` is a fixed type parameter: `T = &'x mut Foo` can emit references too, but `'x` is fixed at
subscription time and the closure is allowed to keep the reference until `'x` ends, so the same
`&'x mut` cannot go to two subscribers, and a subject cannot lend out its own internal value.

## What it would buy

- Zero-copy, no `Clone`: our `PublishSubject` and friends require `T: Clone, E: Clone`; rxrust's
  `_mut_ref` family does not.
- In-place mutation pipelines: several subscribers modify the same value in turn (the rxrust author
  also writes the Ribir GUI framework, where broadcasting `&mut State` to widgets is the main
  motivation).

## Why not

1. **`map` is half-finished on stable.** The stable signature can only be
   `F: for<'a> FnMut(Self::Item<'a>) -> Out`, where `Out` cannot mention `'a`, so
   `map(|s: &String| s.as_str())` — a borrow producing a borrow — cannot be written. Doing it
   requires nightly's `fn_traits` (rxrust ships two `map`s behind `#[cfg(feature = "nightly")]`).
   Full complexity paid, discounted benefit.
2. **HRTB closure inference is fragile.** `|v| v.len()` frequently fails to infer the higher-ranked
   signature and users have to annotate `|v: &str|`. This is a long-standing compiler issue the
   library cannot work around.
3. **The API and type surface double.** Borrowed items cannot go into an ordinary
   `Box<dyn Observer<T>>`; they need a separate `Box<dyn for<'m> DynObserver<&'m mut T, E>>`.
   rxrust therefore has `BoxedObserver` / `BoxedObserverMutRef` / `BoxedCoreObservableMutRef` /
   `...MutRefClone` and paired APIs — `subject` vs `subject_mut_ref`, `multicast` vs
   `multicast_mut_ref`, `group_by` vs `group_by_mut_ref`.
   `where Self: Observable<Item<'a> = &'a mut Item> + 'a` shows up in error messages.
4. **Only useful for synchronous, same-thread pass-through chains.** Any operator that stores values
   — `buffer`, `replay`, `take_last`, `zip`, `combine_latest`, `delay`, `observe_on`, everything
   that goes through a scheduler — cannot hold an `Item<'a>` and must require
   `for<'a> Item<'a>: 'static` or owned values. Past an async boundary, borrowed items are all but
   unusable.
5. **It is a crate-wide breaking change.** `Observable<'or, T, E>` becomes a GAT, every operator
   signature changes with it, and the interaction with `Flow` and
   `Observer::on_next(&mut self, value: T)` has to be worked out.
6. **It is not part of Rx semantics.** In ReactiveX the stream carries values; RxJava / RxSwift
   items are always owned objects. Borrowed items are a Rust-specific extension rxrust made for
   Ribir.

## Alternatives under the current design

- Zero-copy: use `Rc<T>` / `Arc<T>` (`Shared<T>`, chosen by the build mode) as the item; `Clone` is
  a refcount bump.
- Referencing data that outlives the subscription: `T = &'x T` is already supported.
- Broadcasting in-place mutation: use `Shared<Mutable<T>>` as the item and let subscribers modify it
  through `with_mut` — consistent with the existing lock discipline, no trait changes. The only
  thing that is genuinely impossible is "a subject re-borrows its internal value to multiple
  subscribers", and there is no demand for it.
