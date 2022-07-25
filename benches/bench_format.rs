use chrono::{Datelike, NaiveDateTime, Timelike};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::format_scalar_to_slice;
use packedtime_rs::format_simd_dd_to_slice;
use packedtime_rs::format_simd_mul_to_slice;
use std::io::{Cursor, Write};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_write_fmt(input_parts: &[(u32, u32, u32, u32, u32, u32, u32)], output: &mut [u8]) {
    let mut cursor = Cursor::new(output);
    input_parts
        .iter()
        .copied()
        .for_each(move |(year, month, day, hour, minute, second, milli)| {
            cursor
                .write_fmt(format_args!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hour, minute, second, milli
                ))
                .unwrap()
        });
}

#[inline(never)]
fn bench_scalar(input_parts: &[(u32, u32, u32, u32, u32, u32, u32)], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input_parts.iter().copied())
        .for_each(|(ouput, (year, month, day, hour, minute, second, milli))| {
            format_scalar_to_slice(ouput, year, month, day, hour, minute, second, milli)
        });
}

#[inline(never)]
fn bench_simd_mul(input_parts: &[(u32, u32, u32, u32, u32, u32, u32)], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input_parts.iter().copied())
        .for_each(
            |(ouput, (year, month, day, hour, minute, second, milli))| unsafe {
                format_simd_mul_to_slice(ouput, year, month, day, hour, minute, second, milli)
            },
        );
}

#[inline(never)]
fn bench_simd_dd(input_parts: &[(u32, u32, u32, u32, u32, u32, u32)], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input_parts.iter().copied())
        .for_each(
            |(ouput, (year, month, day, hour, minute, second, milli))| unsafe {
                format_simd_dd_to_slice(ouput, year, month, day, hour, minute, second, milli)
            },
        );
}

pub fn bench_format(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut rng = StdRng::seed_from_u64(42);

    let input = (0..BATCH_SIZE)
        .map(|_| {
            let ts = rng.gen_range(0..4102444800_000_i64);
            let ndt = NaiveDateTime::from_timestamp(ts / 1000, 0);
            (
                ndt.year() as u32,
                ndt.month(),
                ndt.day(),
                ndt.hour(),
                ndt.minute(),
                ndt.second(),
                (ts % 1000) as u32,
            )
        })
        .collect::<Vec<_>>();

    let mut output = vec![0_u8; 24 * BATCH_SIZE];

    c.benchmark_group("format")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * (std::mem::size_of_val(&input[0]) + 24)) as u64,
        ))
        .bench_function("format_simd_dd", |b| {
            b.iter(|| {
                bench_simd_dd(&input, &mut output);
            })
        })
        .bench_function("format_simd_mul", |b| {
            b.iter(|| {
                bench_simd_mul(&input, &mut output);
            })
        })
        .bench_function("format_scalar", |b| {
            b.iter(|| {
                bench_scalar(&input, &mut output);
            })
        })
        .bench_function("format_write_fmt", |b| {
            b.iter(|| {
                bench_write_fmt(&input, &mut output);
            })
        });
}

criterion_group!(benches, bench_format);
criterion_main!(benches);
