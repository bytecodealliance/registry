use std::{iter::repeat_with, time::Duration};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::Rng;
use warg_crypto::hash::{Sha256};
use warg_transparency::map::Map;

fn create(items: impl Iterator<Item = ([u8; 32], [u8; 32])>) -> Map<Sha256, [u8; 32], [u8; 32]> {
    Map::<Sha256, _, _>::default().extend(items)
}

fn extend(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let mut grp = c.benchmark_group("map");

    grp.sample_size(50);
    grp.warm_up_time(Duration::from_secs(1));

    for size in [16, 128, 1024] {
        grp.throughput(criterion::Throughput::Elements(size as u64));
        grp.bench_with_input(BenchmarkId::new("extend", size), &size, |b, i| {
            b.iter(|| create(repeat_with(|| (rng.gen(), rng.gen())).take(*i)))
        });
    }
}

criterion_group!(benches, extend);
criterion_main!(benches);
