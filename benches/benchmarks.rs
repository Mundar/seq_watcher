use std::collections::HashMap;
use seq_watcher::SequenceWatcher;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn iterator_comparison<'a, I: Iterator<Item = &'a SequenceWatcher<u8>>>(iter: I) {
    for w in iter {
        w.check(&0);
    }
}

fn version_benchmarks(c: &mut Criterion) {
    let watcher = SequenceWatcher::from(&[b'A', b'B']);
    let mut group = c.benchmark_group("SequenceWatcher::check changes");
    group.bench_function("SequenceWatcher::check", |b| b.iter(|| (black_box(watcher.check(&b'C')))));
    group.finish();
}

const SW_COUNT: usize = 20;
fn theoretical_benchmarks(c: &mut Criterion) {
    let mut v: Vec<SequenceWatcher<u8>> = Vec::with_capacity(SW_COUNT);
    let mut hm: HashMap<Vec<u8>, SequenceWatcher<u8>> = HashMap::with_capacity(SW_COUNT);
    for w in 0..SW_COUNT {
        v.push(SequenceWatcher::new(&[(w as u8)+1]));
        hm.insert(vec![(w as u8)+1], SequenceWatcher::new(&[(w as u8)+1]));
    }
    let mut group = c.benchmark_group("Vectors and HashMaps of SequenceWatcher structures");
    group.bench_function("Vector", |b| b.iter(|| (black_box(iterator_comparison(v.iter())))));
    group.bench_function("HashMap", |b| b.iter(|| (black_box(iterator_comparison(hm.values())))));
    group.finish();
}

criterion_group!(benches, theoretical_benchmarks, version_benchmarks);
criterion_main!(benches);
