use std::fmt::Write;
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::{parse_to_epoch_millis_scalar, parse_to_epoch_millis_simd};

use chrono::NaiveDateTime;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_parse_scalar(input: &[u8], output: &mut [i64]) {
    output.iter_mut().zip(input.chunks(24)).for_each(|(output, input)| {
        let s = unsafe { std::str::from_utf8_unchecked(input) };
        *output = parse_to_epoch_millis_scalar(s).unwrap();
    });
}

#[inline(never)]
fn bench_parse_simd(input: &[u8], output: &mut [i64]) {
    output.iter_mut().zip(input.chunks(24)).for_each(|(output, input)| {
        let s = unsafe { std::str::from_utf8_unchecked(input) };
        *output = parse_to_epoch_millis_simd(s).unwrap();
    });
}

#[inline(never)]
fn bench_parse_chrono(input: &[u8], output: &mut [i64]) {
    output.iter_mut().zip(input.chunks(24)).for_each(|(output, input)| {
        let s = unsafe { std::str::from_utf8_unchecked(input) };
        let dt = chrono::DateTime::parse_from_rfc3339(s).unwrap();
        *output = dt.timestamp_millis();
    });
}

pub fn bench_parse(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let mut input = String::with_capacity(BATCH_SIZE * 24);
    for _i in 0..1024 {
        let ts = rng.gen_range(0..4102444800_000_i64);
        let ndt = NaiveDateTime::from_timestamp(ts / 1000, rng.gen_range(0..1000) * 1_000_000);
        write!(input, "{}T{}Z", ndt.date(), ndt.time()).unwrap();
        // dbg!(&input);
        assert_eq!(input.len() % 24, 0);
    }
    assert_eq!(input.len(), 24 * BATCH_SIZE);

    let mut output = vec![0_i64; BATCH_SIZE];

    c.benchmark_group("parse")
        .throughput(Throughput::Bytes((BATCH_SIZE * (24 + std::mem::size_of::<i64>())) as u64))
        .bench_function("parse_scalar", |b| {
            b.iter(|| bench_parse_scalar(input.as_bytes(), &mut output))
        })
        .bench_function("parse_simd", |b| {
            b.iter(|| bench_parse_simd(input.as_bytes(), &mut output))
        })
        .bench_function("parse_chrono", |b| {
            b.iter(|| bench_parse_chrono(input.as_bytes(), &mut output))
        });
}


criterion_group!(benches, bench_parse);
criterion_main!(benches);
