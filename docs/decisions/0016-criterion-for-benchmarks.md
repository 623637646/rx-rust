# 0016 — Benchmarks on Criterion, one per shared mechanism, compared locally

## Status

Adopted.

## Context

The first benchmark, `benches/serialized_delivery.rs`, was a plain `main` timed with `Instant`:
fifteen samples of a million values per case, the median and the fastest printed in nanoseconds
per value. It was written that way on purpose, with no benchmark framework among the
dev-dependencies. Comparing two builds meant running both, one after the other, more than once,
and judging the difference by eye: nothing said whether a change was larger than the noise.

The benchmarks are there to settle design questions — whether a change to a shared mechanism
makes it faster or slower ([decision 0011](0011-serialized-delivery-in-batches.md) was adopted
with no measurement of the contended case) — not to track the speed of every release.

## Decision

The benchmarks run on Criterion 0.8, a dev-dependency only: the library keeps no required
dependency but educe. Its default features are off, all but `cargo_bench_support`: no plots, no
`rayon`, so it adds little to `Cargo.lock` and to the lint job's build. Its MSRV, 1.86, is below
the crate's.

**Compared locally, against a saved baseline.** `cargo bench --benches -- --save-baseline before`
on the code before a change, `-- --baseline before` after it; Criterion reports whether the
difference is significant. `--benches` matters: every test target is benchmarkable by default,
and libtest rejects Criterion's flags. CI builds the benchmarks — `cargo clippy --all-targets`
in the lint job — and never runs them: numbers from a shared runner say nothing about a change.

**One benchmark per shared mechanism, not per operator.** Most operators are a thin layer over a
machine they share (decisions 0011–0014), so a file is named after the mechanism and drives it
through one or two representative operators: `serialized_delivery`, `subject`
(`SerializedMulticast`), `flatten`, `merge`. A file is added when a design question needs one.

**Time per value.** A throughput case pushes `N` (10 000) values per iteration and declares
`Throughput::Elements(N)`, so that every case, in every file, reads in time per value. A case
runs in both the `Local` and the `Shared` mode where the mode changes the lock on the path, as
`<case>/local` and `<case>/shared`. A case with several threads uses `iter_custom`: the threads
start once per measurement, outside the timing, and each iteration times from a `Barrier` that
releases them to the one they all reach when done. Criterion's defaults for samples, warm-up and
measurement time stay; a filter (`-- switch`) narrows a run instead.

What the files share is `benches/bench_utils/`, used as `tests/tests_utils/` is.

## Rejected

- **The plain `main`.** No statistics, no saved baseline; every comparison was a judgement.
- **A benchmark per operator.** A hundred files measuring the same few machines.
- **Running them in CI to catch regressions.** Needs a dedicated, quiet machine and stored
  history; shared runners vary more than the differences that matter.
- **Comparing with other crates** (rxRust, `futures::Stream`). Their dependencies and MSRVs
  would follow; the benchmarks compare rx-rust with itself.
- **Operators on a scheduler** (`delay`, `debounce`, `observe_on`). The real schedulers are noisy
  and `VirtualTime` mostly measures itself; deferred until a design question needs it.
