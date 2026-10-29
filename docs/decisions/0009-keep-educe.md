# 0009: Keep educe

Whether the crate's one required dependency, `educe`, should be replaced by code of its own.

## educe stays the way generic types derive their traits

Status: adopted.

`educe` is the only dependency of `[dependencies]` that is not optional; the runtimes behind the
scheduler features are. It is a proc-macro: it runs at compile time and puts no code of its own in
a binary, so "no required runtime dependency" (README) holds with it. What it costs is the build
of `syn`, `quote`, `proc-macro2` and `enum-ordinalize`, and an MSRV that follows its own.

The crate derives through it everywhere a type is generic: 167 `#[derive(Educe)]`, most of them
`Debug, Clone`, 57 fields left out of `Debug`, 17 explicit `bound(...)`. The convention is in
`AGENTS.md` ("Derives").

### Why

- **The standard derives give the wrong bounds.** `#[derive(Clone)]` bounds every type parameter,
  so `PublishSubject<'or, T, E, M>` — `Clone(bound())` today, cloneable whatever `T`, `E` and `M`
  are — would ask `T: Clone, E: Clone, M: Clone`. The bounds of a public impl are public API:
  switching is a breaking change for every subject, operator and handle derived this way.
- **The standard `Debug` cannot leave a field out.** A closure (`Create`'s `builder`), a boxed
  observer or a mode pointer has no `Debug`; each of those 57 fields would force a hand-written
  impl.
- **Removing it saves little.** No run time and no binary size: only one build of `syn` and its
  companions, which a dependent usually builds anyway for another derive, and which the
  `futures-scheduler` feature needs regardless, through `futures-macro`.
- **The replacement costs more, and fails quietly.** A hand-written bound that is wrong changes the
  public API with nothing to flag it, and an impl must be kept in step with every field added to
  the type.

### Rejected

- **Hand-written impls.** Thousands of lines of boilerplate across 167 types, each a place for a
  bound to drift from the fields.
- **A `macro_rules!` derive of the crate's own.** Parsing generics and where-clauses in a
  declarative macro is fragile, and inferring the bounds from the fields, which is what educe is
  here for, is out of its reach: it would be a reimplementation of educe, worse.
- **A derive crate of our own.** Depends on `syn` and `quote` just the same, and is one more crate
  to maintain and publish.
- **Trimming educe's features** (`default-features = false` with the six derives in use). It shrinks
  the generated macro code a little, but `enum-ordinalize` is not optional in educe and stays; not
  worth a line of `Cargo.toml`.

### What to watch

- **The MSRV follows educe.** 2.0.0 raised it from 1.88 to 1.89 for educe 0.8. An educe upgrade
  that raises the rustc it needs raises the crate's MSRV, a breaking change (`!` in the commit).
- **An upgrade can change bounds.** educe 0.6 bounded a derived `Eq` by `PartialEq` only; 0.8 by
  `Eq`, which made `Termination<f64>` and four other public types stop being `Eq` (CHANGELOG,
  2.0.0). Review an educe upgrade as a possible API change: compare the derived impls' bounds,
  not only that the crate builds.
