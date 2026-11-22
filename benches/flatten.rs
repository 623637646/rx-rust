//! The cost per value of the state machine that flattens an observable of observables,
//! `combining/internal/flatten.rs`, under four of the operators built on it. The inners are
//! synchronous, so every operator finds no inner running when the next arrives: what the cases
//! compare is the machine and its observers, not the stores of the inners that find no place.
//! `exhaust_all` takes the path of `merge_all` there and is left out.
//!
//! ```bash
//! cargo bench --bench flatten
//! cargo bench --bench flatten -- switch   # only the cases whose id contains it
//! ```

mod bench_utils;

use bench_utils::{N, deliver, throughput_group};
use criterion::{Criterion, criterion_group, criterion_main};
use rx_rust::{
    observable::ObservableExt,
    operators::creating::{just::Just, range::Range},
};

/// The inners of `few_long_inners`, each emitting `N / INNERS` values.
const INNERS: u64 = 10;

/// Each operator of `[$flatten]`, in the `Local` and the `Shared` mode, over the outer observable
/// `$outer` makes from `$range`, a `Range` over `$values`.
macro_rules! cases {
    ($group:ident, $values:expr, |$range:ident| $outer:expr, [$($flatten:ident),+]) => {$(
        $group.bench_function(concat!(stringify!($flatten), "/local"), |b| {
            b.iter(|| {
                let $range = Range::new($values);
                deliver!($outer.$flatten())
            });
        });
        $group.bench_function(concat!(stringify!($flatten), "/shared"), |b| {
            b.iter(|| {
                let $range = Range::new($values).into_shared();
                deliver!($outer.$flatten())
            });
        });
    )+};
}

/// `N` inners of one value each: mostly the subscription and release of an inner.
fn many_inners(c: &mut Criterion) {
    let mut group = throughput_group(c, "flatten/many_inners");
    cases!(
        group,
        0..N,
        |values| values.map(Just::new),
        [merge_all, concat_all, switch, concat_eager_all]
    );
    group.finish();
}

/// `INNERS` inners of `N / INNERS` values each: mostly a value through an inner observer.
fn few_long_inners(c: &mut Criterion) {
    let mut group = throughput_group(c, "flatten/few_long_inners");
    let length = N / INNERS;
    cases!(
        group,
        0..INNERS,
        |inners| inners.map(move |inner| Range::new(inner * length..(inner + 1) * length)),
        [merge_all, concat_all, switch, concat_eager_all]
    );
    group.finish();
}

criterion_group!(benches, many_inners, few_long_inners);
criterion_main!(benches);
