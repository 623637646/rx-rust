//! The cost per value of the merge of a source set, the `Driver` of
//! `combining/internal/merge.rs`: the tuple and the list forms, many positions, a limit that keeps
//! the cursor waiting, and `Concat` over a list, a merge of one source at a time.
//!
//! ```bash
//! cargo bench --bench merge
//! cargo bench --bench merge -- max_concurrency   # only the cases whose id contains it
//! ```

mod bench_utils;

use bench_utils::{N, deliver, throughput_group};
use criterion::{Criterion, criterion_group, criterion_main};
use rx_rust::{
    observable::{ObservableExt, ObservableTypes},
    operators::{
        combining::{concat::Concat, merge::Merge},
        creating::{just::Just, range::Range},
        others::into_shared::IntoShared,
    },
};
use std::num::NonZeroUsize;

/// The sources of the cases with four, each emitting `N / SOURCES` values.
const SOURCES: u64 = 4;

/// A source left in the `Local` mode of `Range` and `Just`.
fn local<OE>(source: OE) -> OE {
    source
}

/// A source moved to the `Shared` mode, which the merge then takes.
fn shared<OE: ObservableTypes>(source: OE) -> IntoShared<OE> {
    source.into_shared()
}

/// The `position`-th of the `SOURCES` ranges that together emit `0..N`.
fn part(position: u64) -> Range<std::ops::Range<u64>> {
    let length = N / SOURCES;
    Range::new(position * length..(position + 1) * length)
}

/// The case `$name` in the `Local` and the `Shared` mode: `$merge` builds the merge, calling
/// `$mode` on each source.
macro_rules! cases {
    ($group:ident, $name:literal, |$mode:ident| $merge:expr) => {
        $group.bench_function(concat!($name, "/local"), |b| {
            b.iter(|| {
                let $mode = local;
                deliver!($merge)
            });
        });
        $group.bench_function(concat!($name, "/shared"), |b| {
            b.iter(|| {
                let $mode = shared;
                deliver!($merge)
            });
        });
    };
}

fn merge(c: &mut Criterion) {
    let mut group = throughput_group(c, "merge");
    cases!(group, "tuple/4_sources", |mode| Merge::new((
        mode(part(0)),
        mode(part(1)),
        mode(part(2)),
        mode(part(3)),
    )));
    cases!(group, "list/4_sources", |mode| Merge::new_from_iter(
        (0..SOURCES).map(|position| mode(part(position)))
    ));
    // `N` positions of one value each: the slots and the cursor of a long list.
    cases!(group, "list/N_sources", |mode| Merge::new_from_iter(
        (0..N).map(|value| mode(Just::new(value)))
    ));
    // Two sources at a time: the cursor waits in the model for one of them to complete.
    cases!(group, "list/4_sources/max_concurrency_2", |mode| {
        Merge::new_from_iter((0..SOURCES).map(|position| mode(part(position))))
            .max_concurrency(NonZeroUsize::new(2).expect("two is not zero"))
    });
    group.finish();
}

/// `Concat` over a list: a `Merge` of one source at a time.
fn concat(c: &mut Criterion) {
    let mut group = throughput_group(c, "concat");
    cases!(group, "list/4_sources", |mode| Concat::new_from_iter(
        (0..SOURCES).map(|position| mode(part(position)))
    ));
    group.finish();
}

criterion_group!(benches, merge, concat);
criterion_main!(benches);
