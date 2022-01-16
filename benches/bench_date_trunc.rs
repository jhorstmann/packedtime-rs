use chrono::{Datelike, NaiveDateTime, NaiveDate, NaiveTime, Timelike};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::{date_trunc_month_timestamp_millis, date_trunc_year_timestamp_millis};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_date_trunc_year(input: &[i64], output: &mut [i64]) {
    output.iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_year_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_trunc_month(input: &[i64], output: &mut [i64]) {
    output.iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            *output = date_trunc_month_timestamp_millis(input);
        });
}

#[inline(never)]
fn bench_date_trunc_year_chrono(input: &[i64], output: &mut [i64]) {
    output.iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, (input % 1000 * 1_000_000) as u32);
            let truncated = NaiveDateTime::new(NaiveDate::from_ymd(ndt.year(), 1, 1), NaiveTime::from_hms(0, 0, 0));
            *output = truncated.timestamp_millis();
        });
}

#[inline(never)]
fn bench_date_trunc_month_chrono(input: &[i64], output: &mut [i64]) {
    output.iter_mut()
        .zip(input.iter().copied())
        .for_each(|(output, input)| {
            let ndt = NaiveDateTime::from_timestamp(input / 1000, (input % 1000 * 1_000_000) as u32);
            let truncated = NaiveDateTime::new(NaiveDate::from_ymd(ndt.year(), ndt.month(), 1), NaiveTime::from_hms(0, 0, 0));
            *output = truncated.timestamp_millis();
        });
}


pub fn bench_date_trunc(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let input = (0..BATCH_SIZE).map(|_| {
        rng.gen_range(0..4102444800_000_i64)
    }).collect::<Vec<_>>();

    let mut output = vec!(0_i64; BATCH_SIZE);

    c.benchmark_group("date_trunc")
        .throughput(Throughput::Bytes((BATCH_SIZE * 2 * std::mem::size_of::<i64>()) as u64))
        .bench_function("date_trunc_year", |b| {
            b.iter(|| bench_date_trunc_year(&input, &mut output))
        })
        .bench_function("date_trunc_month", |b| {
            b.iter(|| bench_date_trunc_month(&input, &mut output))
        })
        .bench_function("date_trunc_year_chrono", |b| {
            b.iter(|| bench_date_trunc_year_chrono(&input, &mut output))
        })
        .bench_function("date_trunc_month_chrono", |b| {
            b.iter(|| bench_date_trunc_month_chrono(&input, &mut output))
        })
    ;
}


criterion_group!(benches, bench_date_trunc);
criterion_main!(benches);
