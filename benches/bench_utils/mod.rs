//! What the benchmarks share: the number of values a case pushes, the capture that makes a
//! closure large, and the subscription that counts what a pipeline delivers.
#![allow(dead_code)]

use criterion::{BenchmarkGroup, Criterion, Throughput, measurement::WallTime};
use std::hint::black_box;

/// The values one iteration of a throughput case pushes: the report divides by it, so that every
/// case reads in time per value.
pub(crate) const N: u64 = 10_000;

/// The threads of a contended case.
pub(crate) const THREADS: u64 = 4;

/// A closure capture of this size makes every `map` of a chain this much larger.
pub(crate) type Pad = [u64; 32];

/// Four pads hidden from the optimizer, one per `map` of a fat chain.
pub(crate) fn pads() -> (Pad, Pad, Pad, Pad) {
    black_box(([1; 32], [2; 32], [3; 32], [4; 32]))
}

/// A group whose cases each push `N` values per iteration.
pub(crate) fn throughput_group<'c>(
    c: &'c mut Criterion,
    name: &str,
) -> BenchmarkGroup<'c, WallTime> {
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Elements(N));
    group
}

/// Subscribes to `$observable` with a callback that counts the values, hides each from the
/// optimizer, and checks that all `N` arrived before the subscription is dropped.
#[allow(unused_macros)]
macro_rules! deliver {
    ($observable:expr) => {{
        let mut received = 0_u64;
        let subscription = $observable.subscribe_with_callback(
            |value| {
                ::std::hint::black_box(value);
                received += 1;
            },
            |_| {},
        );
        drop(subscription);
        assert_eq!(
            received,
            $crate::bench_utils::N,
            "every value was delivered"
        );
    }};
}
#[allow(unused_imports)]
pub(crate) use deliver;
