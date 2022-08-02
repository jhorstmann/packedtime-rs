use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use packedtime_rs::date_add_month_timestamp_millis;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_date_add_month(input: &[i64], output: &mut [i64], months: i32) {
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_add_month_timestamp_millis(input, months);
        });
}

pub fn bench_date_add(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let input = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64))
        .collect::<Vec<_>>();

    let mut output = vec![0_i64; BATCH_SIZE];

    c.benchmark_group("date_add_month")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * 2 * std::mem::size_of::<i64>()) as u64,
        ))
        .bench_function("date_add_month", |b| {
            b.iter(|| bench_date_add_month(&input, &mut output, 1))
        });
}

criterion_group!(benches, bench_date_add);
criterion_main!(benches);
