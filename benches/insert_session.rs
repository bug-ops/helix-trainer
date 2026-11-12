//! Cost of one typed key in a long Insert session with ten cursors
//!
//! The session records every change, so the cost of a key must not grow with
//! the number of keys typed before it. The acceptance ratio is the time of
//! key 8000 over the time of key 200, at most 1.5.
//!
//! Run with: cargo bench --bench `insert_session`

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use helix_trainer::helix::simulator::AnyModeSimulator;
use std::hint::black_box;

const CURSORS: usize = 10;

/// A simulator in Insert mode with ten cursors after `typed` keys.
fn session_after(typed: usize) -> AnyModeSimulator {
    let text = ["abcdefghij"; CURSORS].join("\n");
    let mut sim = AnyModeSimulator::new(text);
    for _ in 1..CURSORS {
        sim.execute_command("C").expect("C copies the cursor down");
    }
    sim.execute_command("i").expect("i enters Insert mode");
    for _ in 0..typed {
        sim.execute_command("x").expect("typing succeeds");
    }
    sim
}

fn typed_key(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_session_key");
    group.sample_size(20);
    for key in [200_usize, 2_000, 8_000] {
        group.bench_with_input(BenchmarkId::from_parameter(key), &key, |b, &key| {
            b.iter_batched(
                || session_after(key - 1),
                |mut sim| {
                    sim.execute_command(black_box("x"))
                        .expect("typing succeeds");
                    sim
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, typed_key);
criterion_main!(benches);
