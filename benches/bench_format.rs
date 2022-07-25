use chrono::{DateTime, Datelike, NaiveDateTime, SecondsFormat, Timelike, Utc};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::format_simd_dd_to_slice;
use packedtime_rs::format_simd_mul_to_slice;
use packedtime_rs::{format_scalar_to_slice, PackedTimestamp};
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

#[inline(never)]
fn bench_timestamp_simd(input: &[i64], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input.iter())
        .for_each(|(out, inp)| {
            let ts = PackedTimestamp::from_timestamp_millis(*inp);
            unsafe {
                format_simd_mul_to_slice(
                    out,
                    ts.year(),
                    ts.month(),
                    ts.day(),
                    ts.hour(),
                    ts.minute(),
                    ts.second(),
                    ts.millisecond(),
                );
            }
        })
}

#[inline(never)]
fn bench_timestamp_scalar(input: &[i64], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input.iter())
        .for_each(|(out, inp)| {
            let ts = PackedTimestamp::from_timestamp_millis(*inp);
            format_scalar_to_slice(
                out,
                ts.year(),
                ts.month(),
                ts.day(),
                ts.hour(),
                ts.minute(),
                ts.second(),
                ts.millisecond(),
            );
        })
}

#[inline(never)]
fn bench_timestamp_chrono(input: &[i64], output: &mut [u8]) {
    output
        .chunks_mut(24)
        .zip(input.iter())
        .for_each(|(out, inp)| {
            let ts = NaiveDateTime::from_timestamp(*inp / 1000, (*inp % 1000) as _);
            let formatted =
                DateTime::<Utc>::from_utc(ts, Utc).to_rfc3339_opts(SecondsFormat::Millis, true);
            out.copy_from_slice(formatted.as_bytes());
        })
}

pub fn bench_format(c: &mut Criterion) {
    const BATCH_SIZE: usize = 1024;

    let mut output = vec![0_u8; 24 * BATCH_SIZE];

    let mut rng = StdRng::seed_from_u64(42);

    let inputs = (0..BATCH_SIZE)
        .map(|_| rng.gen_range(0..4102444800_000_i64))
        .collect::<Vec<i64>>();

    c.benchmark_group("format_timestamp")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * (std::mem::size_of::<i64>() + 24)) as u64,
        ))
        .bench_function("format_simd", |b| {
            b.iter(|| bench_timestamp_simd(&inputs, &mut output));
        })
        .bench_function("format_scalar", |b| {
            b.iter(|| bench_timestamp_scalar(&inputs, &mut output));
        })
        .bench_function("format_chrono", |b| {
            b.iter(|| bench_timestamp_chrono(&inputs, &mut output));
        });

    let inputs = inputs
        .iter()
        .map(|ts| {
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

    c.benchmark_group("format")
        .throughput(Throughput::Bytes(
            (BATCH_SIZE * (std::mem::size_of_val(&inputs[0]) + 24)) as u64,
        ))
        .bench_function("format_simd_dd", |b| {
            b.iter(|| {
                bench_simd_dd(&inputs, &mut output);
            })
        })
        .bench_function("format_simd_mul", |b| {
            b.iter(|| {
                bench_simd_mul(&inputs, &mut output);
            })
        })
        .bench_function("format_scalar", |b| {
            b.iter(|| {
                bench_scalar(&inputs, &mut output);
            })
        })
        .bench_function("format_write_fmt", |b| {
            b.iter(|| {
                bench_write_fmt(&inputs, &mut output);
            })
        });
}

criterion_group!(benches, bench_format);
criterion_main!(benches);
