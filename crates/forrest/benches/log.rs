use std::{iter::repeat_with, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use forrest::log::{LogBuilder, VecLog};
use rand::Rng;
use warg_crypto::hash::Sha256;

fn run(items: impl Iterator<Item = [u8; 32]>) -> VecLog<Sha256, [u8; 32]> {
    let mut log: VecLog<Sha256, [u8; 32]> = VecLog::default();
    for item in items {
        log.push(&item);
    }
    log
}

fn log_bench(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut grp = c.benchmark_group("log");

    grp.sample_size(50);
    grp.warm_up_time(Duration::from_secs(1));

    for size in [16, 128, 1024] {
        grp.throughput(criterion::Throughput::Elements(size as u64));
        grp.bench_with_input(BenchmarkId::new("push", size), &size, |b, i| {
            b.iter(|| run(repeat_with(|| rng.gen()).take(*i)))
        });
    }

    drop(grp);

    let mut grp = c.benchmark_group("log-big");

    grp.sample_size(10);
    grp.sampling_mode(SamplingMode::Flat);
    grp.warm_up_time(Duration::from_secs(30));

    for size in [1_048_576] {
        grp.throughput(criterion::Throughput::Elements(size as u64));
        grp.bench_with_input(BenchmarkId::new("push", size), &size, |b, i| {
            b.iter(|| black_box(run(repeat_with(|| rng.gen()).take(*i))))
        });
    }
}

criterion_group!(benches, log_bench);
criterion_main!(benches);
