use chrono::NaiveDateTime;
use chronoutil::shift_months;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};

use packedtime_rs::{
    date_add_month_timestamp_millis, date_add_month_timestamp_millis_float,
    date_diff_month_timestamp_millis, date_diff_month_timestamp_millis_float,
    date_diff_year_timestamp_millis, date_diff_year_timestamp_millis_float,
};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_date_add_month(input: &[i64], output: &mut [i64], months: i32) {
    assert_eq!(input.len(), output.len());
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_add_month_timestamp_millis(input, months);
        });
}

#[inline(never)]
fn bench_date_add_month_float(input: &[f64], output: &mut [f64], months: i32) {
    assert_eq!(input.len(), output.len());
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_add_month_timestamp_millis_float(input, months);
        });
}

#[inline(never)]
fn bench_date_diff_month(start: &[i64], end: &[i64], output: &mut [i32]) {
    assert_eq!(start.len(), end.len());
    assert_eq!(start.len(), output.len());
    output
        .iter_mut()
        .zip(start.iter().copied().zip(end.iter().copied()))
        .for_each(|(output, (start, end))| {
            *output = date_diff_month_timestamp_millis(start, end);
        });
}

#[inline(never)]
fn bench_date_diff_month_float(start: &[f64], end: &[f64], output: &mut [i32]) {
    assert_eq!(start.len(), end.len());
    assert_eq!(start.len(), output.len());
    output
        .iter_mut()
        .zip(start.iter().copied().zip(end.iter().copied()))
        .for_each(|(output, (start, end))| {
            *output = date_diff_month_timestamp_millis_float(start, end);
        });
}

#[inline(never)]
fn bench_date_diff_year(start: &[i64], end: &[i64], output: &mut [i32]) {
    assert_eq!(start.len(), end.len());
    assert_eq!(start.len(), output.len());
    output
        .iter_mut()
        .zip(start.iter().copied().zip(end.iter().copied()))
        .for_each(|(output, (start, end))| {
            *output = date_diff_year_timestamp_millis(start, end);
        });
}

#[inline(never)]
fn bench_date_diff_year_float(start: &[f64], end: &[f64], output: &mut [i32]) {
    assert_eq!(start.len(), end.len());
    assert_eq!(start.len(), output.len());
    output
        .iter_mut()
        .zip(start.iter().copied().zip(end.iter().copied()))
        .for_each(|(output, (start, end))| {
            *output = date_diff_year_timestamp_millis_float(start, end);
        });
}

#[inline(never)]
fn bench_date_add_month_chronoutil(input: &[i64], output: &mut [i64], months: i32) {
    assert_eq!(input.len(), output.len());
    output
        .iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt =
                NaiveDateTime::from_timestamp(input / 1000, (input % 1000 * 1_000_000) as u32);

            *output = shift_months(ndt, months).timestamp_millis();
        });
}

pub fn bench_date_add(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let input = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64))
        .collect::<Vec<_>>();

    let input2 = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64))
        .collect::<Vec<_>>();

    let input_float = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64) as f64)
        .collect::<Vec<_>>();

    let input_float2 = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64) as f64)
        .collect::<Vec<_>>();

    let mut output = vec![0_i64; BATCH_SIZE];
    let mut output_float = vec![0_f64; BATCH_SIZE];
    let mut output_diff = vec![0_i32; BATCH_SIZE];

    c.benchmark_group("date_add_month")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * 2 * std::mem::size_of::<i64>()) as u64,
        ))
        .bench_function("date_add_month", |b| {
            b.iter(|| bench_date_add_month(&input, &mut output, 1))
        })
        .bench_function("date_add_month_float", |b| {
            b.iter(|| bench_date_add_month_float(&input_float, &mut output_float, 1))
        })
        .bench_function("date_add_month_chronoutil", |b| {
            b.iter(|| bench_date_add_month_chronoutil(&input, &mut output, 1))
        });

    c.benchmark_group("date_diff_month")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * 2 * std::mem::size_of::<i64>() + BATCH_SIZE * std::mem::size_of::<i32>())
                as u64,
        ))
        .bench_function("date_diff_month", |b| {
            b.iter(|| bench_date_diff_month(&input, &input2, &mut output_diff))
        })
        .bench_function("date_diff_month_float", |b| {
            b.iter(|| bench_date_diff_month_float(&input_float, &input_float2, &mut output_diff))
        });

    c.benchmark_group("date_diff_year")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * 2 * std::mem::size_of::<i64>() + BATCH_SIZE * std::mem::size_of::<i32>())
                as u64,
        ))
        .bench_function("date_diff_year", |b| {
            b.iter(|| bench_date_diff_year(&input, &input2, &mut output_diff))
        })
        .bench_function("date_diff_year_float", |b| {
            b.iter(|| bench_date_diff_year_float(&input_float, &input_float2, &mut output_diff))
        });
}

criterion_group!(benches, bench_date_add);
criterion_main!(benches);
