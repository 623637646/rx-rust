//! The cost per value of the subjects' multicast delivery, `SerializedMulticast`: a
//! `PublishSubject` sent to by one thread or by several at once, and the replay a
//! `ReplaySubject` sends a new subscriber.
//!
//! ```bash
//! cargo bench --bench subject
//! cargo bench --bench subject -- publish/4_threads   # only the cases whose id contains it
//! ```

mod bench_utils;

use bench_utils::{N, THREADS, pads, throughput_group};
use criterion::{Criterion, criterion_group, criterion_main};
use rx_rust::{
    observable::ObservableExt,
    observer::Observer,
    subject::{publish_subject::PublishSubject, replay_subject::ReplaySubject},
};
use std::{
    convert::Infallible,
    hint::black_box,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

/// Subscribes `$subscribers` counting callbacks to `$subject`, sends it `N` values, and checks
/// that every subscriber received each of them.
macro_rules! publish {
    ($subject:expr, $subscribers:expr) => {{
        // Declared before the subject, which may hold the subscribers until it is dropped.
        let received = AtomicU64::new(0);
        let subject = $subject;
        let mut sender = subject.clone();
        let subscriptions: Vec<_> = (0..$subscribers)
            .map(|_| {
                subject.clone().subscribe_with_callback(
                    |value| {
                        black_box(value);
                        received.fetch_add(1, Ordering::Relaxed);
                    },
                    |_| {},
                )
            })
            .collect();
        for value in 0..N {
            let _ = sender.on_next(value);
        }
        drop(subscriptions);
        assert_eq!(received.load(Ordering::Relaxed), N * $subscribers);
    }};
}

/// A `PublishSubject` sent `N` values.
fn publish(c: &mut Criterion) {
    let mut group = throughput_group(c, "publish");
    group.bench_function("1_subscriber/local", |b| {
        b.iter(|| publish!(PublishSubject::<_, Infallible, _>::local(), 1));
    });
    group.bench_function("1_subscriber/shared", |b| {
        b.iter(|| publish!(PublishSubject::<_, Infallible, _>::shared(), 1));
    });
    // The subject delivers to its list of subscribers, so the chain sits behind a box and does not
    // grow what the delivery moves.
    group.bench_function("1_subscriber+4_fat_maps", |b| {
        b.iter(|| {
            let mut received = 0;
            let subject = PublishSubject::<_, Infallible, _>::shared();
            let mut sender = subject.clone();
            let (p0, p1, p2, p3) = pads();
            let subscription = subject
                .map(move |value: u64| value ^ p0[(value % 32) as usize])
                .map(move |value| value ^ p1[(value % 32) as usize])
                .map(move |value| value ^ p2[(value % 32) as usize])
                .map(move |value| value ^ p3[(value % 32) as usize])
                .subscribe_with_callback(
                    |value| {
                        black_box(value);
                        received += 1;
                    },
                    |_| {},
                );
            for value in 0..N {
                let _ = sender.on_next(value);
            }
            drop((subscription, sender));
            assert_eq!(received, N);
        });
    });
    // The fan-out: each value is delivered eight times.
    group.bench_function("8_subscribers", |b| {
        b.iter(|| publish!(PublishSubject::<_, Infallible, _>::shared(), 8));
    });
    // `THREADS` threads send at once: a value that finds a delivery running is queued behind it,
    // and the handover between two deliverers is contended. The threads are started once per
    // measurement, outside the timing, and each iteration times from the barrier that releases
    // them to the one they all reach when done.
    group.bench_function(format!("{THREADS}_threads"), |b| {
        b.iter_custom(|iterations| {
            let barrier = Barrier::new(THREADS as usize + 1);
            thread::scope(|scope| {
                let channels: Vec<_> = (0..THREADS)
                    .map(|thread| {
                        let (channel, subjects) = mpsc::channel();
                        let barrier = &barrier;
                        scope.spawn(move || {
                            for mut sender in subjects {
                                barrier.wait();
                                for value in (thread..N).step_by(THREADS as usize) {
                                    let _ =
                                        Observer::<u64, Infallible>::on_next(&mut sender, value);
                                }
                                drop(sender);
                                barrier.wait();
                            }
                        });
                        channel
                    })
                    .collect();
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let subject = PublishSubject::<_, Infallible, _>::shared();
                    let received = Arc::new(AtomicU64::new(0));
                    let counter = received.clone();
                    let subscription = subject.clone().subscribe_with_callback(
                        move |value: u64| {
                            black_box(value);
                            counter.fetch_add(1, Ordering::Relaxed);
                        },
                        |_| {},
                    );
                    for channel in &channels {
                        channel
                            .send(subject.clone())
                            .expect("the thread is running");
                    }
                    barrier.wait();
                    let start = Instant::now();
                    barrier.wait();
                    elapsed += start.elapsed();
                    drop(subscription);
                    assert_eq!(received.load(Ordering::Relaxed), N);
                }
                elapsed
            })
        });
    });
    group.finish();
}

/// A new subscriber of a `ReplaySubject` that holds `N` values: what the replay costs per value.
/// The subject is filled once; each iteration subscribes and drops the subscription.
fn replay(c: &mut Criterion) {
    let mut group = throughput_group(c, "replay");
    macro_rules! subscribe_with_buffered {
        ($name:literal, $subject:expr) => {
            group.bench_function($name, |b| {
                let subject = $subject;
                let mut sender = subject.clone();
                for value in 0..N {
                    let _ = sender.on_next(value);
                }
                b.iter(|| {
                    let received = Arc::new(AtomicU64::new(0));
                    let counter = received.clone();
                    let subscription = subject.clone().subscribe_with_callback(
                        move |value| {
                            black_box(value);
                            counter.fetch_add(1, Ordering::Relaxed);
                        },
                        |_| {},
                    );
                    drop(subscription);
                    assert_eq!(received.load(Ordering::Relaxed), N);
                });
            });
        };
    }
    subscribe_with_buffered!(
        "subscribe_with_N_buffered/local",
        ReplaySubject::<_, Infallible, _>::local(None)
    );
    subscribe_with_buffered!(
        "subscribe_with_N_buffered/shared",
        ReplaySubject::<_, Infallible, _>::shared(None)
    );
    group.finish();
}

criterion_group!(benches, publish, replay);
criterion_main!(benches);
