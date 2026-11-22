//! The cost per value of the serialized delivery under `subscribe_with_context`: an operator whose
//! context queues each value (`take_until`, `with_latest_from`) or none (`skip_until`), against
//! the same pipelines with no context (`baseline`).
//!
//! ```bash
//! cargo bench --bench serialized_delivery
//! cargo bench --bench serialized_delivery -- take_until   # only the cases whose id contains it
//! ```

mod bench_utils;

use bench_utils::{N, deliver, pads, throughput_group};
use criterion::{Criterion, criterion_group, criterion_main};
use rx_rust::{
    observable::ObservableExt,
    operators::creating::{just::Just, never::Never, range::Range},
};
use std::{
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

/// `send_next` on a context: each value starts and parks a delivery. The `Shared` context is
/// behind a `Mutex`; the observer after it grows with the chain of `map`s, before it does not.
fn take_until(c: &mut Criterion) {
    let mut group = throughput_group(c, "take_until");
    group.bench_function("local", |b| {
        b.iter(|| deliver!(Range::new(0..N).take_until(Never)));
    });
    group.bench_function("shared", |b| {
        b.iter(|| deliver!(Range::new(0..N).into_shared().take_until(Never)));
    });
    // Eight `map`s, each capturing one word: an observer of about 64 bytes.
    group.bench_function("shared+8_thin_maps", |b| {
        b.iter(|| {
            let [k0, k1, k2, k3, k4, k5, k6, k7] = black_box([1_u64, 2, 3, 4, 5, 6, 7, 8]);
            deliver!(
                Range::new(0..N)
                    .into_shared()
                    .take_until(Never)
                    .map(move |value| value ^ k0)
                    .map(move |value| value ^ k1)
                    .map(move |value| value ^ k2)
                    .map(move |value| value ^ k3)
                    .map(move |value| value ^ k4)
                    .map(move |value| value ^ k5)
                    .map(move |value| value ^ k6)
                    .map(move |value| value ^ k7)
            )
        });
    });
    // Four `map`s, each capturing 256 bytes: an observer of about 1 KiB.
    group.bench_function("shared+4_fat_maps", |b| {
        b.iter(|| {
            let (p0, p1, p2, p3) = pads();
            deliver!(
                Range::new(0..N)
                    .into_shared()
                    .take_until(Never)
                    .map(move |value| value ^ p0[(value % 32) as usize])
                    .map(move |value| value ^ p1[(value % 32) as usize])
                    .map(move |value| value ^ p2[(value % 32) as usize])
                    .map(move |value| value ^ p3[(value % 32) as usize])
            )
        });
    });
    // The same work per value, but the observer the context delivers to stays small.
    group.bench_function("shared+4_fat_maps_upstream", |b| {
        b.iter(|| {
            let (p0, p1, p2, p3) = pads();
            deliver!(
                Range::new(0..N)
                    .into_shared()
                    .map(move |value| value ^ p0[(value % 32) as usize])
                    .map(move |value| value ^ p1[(value % 32) as usize])
                    .map(move |value| value ^ p2[(value % 32) as usize])
                    .map(move |value| value ^ p3[(value % 32) as usize])
                    .take_until(Never)
            )
        });
    });
    group.finish();
}

/// An update that queues an event for each value (`with_next_event`).
fn with_latest_from(c: &mut Criterion) {
    let mut group = throughput_group(c, "with_latest_from");
    group.bench_function("shared", |b| {
        b.iter(|| {
            deliver!(
                Range::new(0..N)
                    .into_shared()
                    .with_latest_from(Just::new(0_u64))
            )
        });
    });
    group.finish();
}

/// An update that queues nothing for each value (`without_events`): the values are skipped, so
/// the case counts the updates rather than the deliveries.
fn skip_until(c: &mut Criterion) {
    let mut group = throughput_group(c, "skip_until");
    group.bench_function("shared", |b| {
        b.iter(|| {
            let updates = Arc::new(AtomicU64::new(0));
            let counter = updates.clone();
            let subscription = Range::new(0..N)
                .into_shared()
                .hook_on_next(move |observer, value| {
                    counter.fetch_add(1, Ordering::Relaxed);
                    observer.on_next(value)
                })
                .skip_until(Never)
                .subscribe_with_callback(
                    |value: u64| {
                        black_box(value);
                    },
                    |_| {},
                );
            drop(subscription);
            assert_eq!(
                updates.load(Ordering::Relaxed),
                N,
                "every value was an update"
            );
        });
    });
    group.finish();
}

/// The pipelines with no context at all: what the source, the `map`s and the callback cost on
/// their own.
fn baseline(c: &mut Criterion) {
    let mut group = throughput_group(c, "baseline");
    group.bench_function("map_only", |b| {
        b.iter(|| deliver!(Range::new(0..N).map(|value| value + 1)));
    });
    group.bench_function("4_fat_maps", |b| {
        b.iter(|| {
            let (p0, p1, p2, p3) = pads();
            deliver!(
                Range::new(0..N)
                    .map(move |value| value ^ p0[(value % 32) as usize])
                    .map(move |value| value ^ p1[(value % 32) as usize])
                    .map(move |value| value ^ p2[(value % 32) as usize])
                    .map(move |value| value ^ p3[(value % 32) as usize])
            )
        });
    });
    group.finish();
}

criterion_group!(benches, take_until, with_latest_from, skip_until, baseline);
criterion_main!(benches);
