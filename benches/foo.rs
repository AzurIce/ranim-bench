use criterion::{Criterion, criterion_group, criterion_main};

fn foo() {}

fn bench_foo(c: &mut Criterion) {
    let mut group = c.benchmark_group("foo");
    group.bench_function("foo_1", |b| b.iter(|| foo()));
    group.bench_function("foo_2", |b| b.iter(|| foo()));
}

criterion_group!(benches, bench_foo);
criterion_main!(benches);
