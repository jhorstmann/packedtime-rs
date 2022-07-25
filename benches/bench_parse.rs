use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use packedtime_rs::{
    parse_to_packed_timestamp_scalar, parse_to_packed_timestamp_simd, PackedTimestamp,
};
use std::fmt::Write;
use std::ops::Range;

use chrono::{Datelike, NaiveDateTime, Timelike};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[inline(never)]
fn bench_parse_scalar(input: &[u8], output: &mut [PackedTimestamp], date_len: usize) {
    output
        .iter_mut()
        .zip(input.chunks(date_len))
        .for_each(|(output, input)| {
            let s = unsafe { std::str::from_utf8_unchecked(input) };
            *output = parse_to_packed_timestamp_scalar(s).unwrap();
        });
}

#[inline(never)]
fn bench_parse_simd(input: &[u8], output: &mut [PackedTimestamp], date_len: usize) {
    output
        .iter_mut()
        .zip(input.chunks(date_len))
        .for_each(|(output, input)| {
            let s = unsafe { std::str::from_utf8_unchecked(input) };
            *output = parse_to_packed_timestamp_simd(s).unwrap();
        });
}

#[inline(never)]
fn bench_parse_chrono(input: &[u8], output: &mut [PackedTimestamp], date_len: usize) {
    output
        .iter_mut()
        .zip(input.chunks(date_len))
        .for_each(|(output, input)| {
            let s = unsafe { std::str::from_utf8_unchecked(input) };
            let dt = chrono::DateTime::parse_from_rfc3339(s).unwrap();
            *output = PackedTimestamp::new(
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second(),
                dt.timestamp_millis() as u32,
                dt.offset().local_minus_utc() / 60,
            );
        });
}

pub fn bench_parse(c: &mut Criterion) {
    const DATE_LEN_UTC: usize = 24;
    const DATE_LEN_WITH_OFFSET: usize = 29;

    const BATCH_SIZE: usize = 512;
    const TS_RANGE: Range<i64> = 0..4102444800_000_i64;

    let mut rng = StdRng::seed_from_u64(42);

    let mut input_utc = String::with_capacity(BATCH_SIZE * DATE_LEN_UTC);
    let mut input_with_offset = String::with_capacity(BATCH_SIZE * DATE_LEN_WITH_OFFSET);
    for _i in 0..BATCH_SIZE {
        let ts = rng.gen_range(TS_RANGE);
        let ndt = NaiveDateTime::from_timestamp(ts / 1000, rng.gen_range(0..1000) * 1_000_000);
        let offset = rng.gen_range(0..12_i32);
        let offset_sign = if rng.gen_bool(0.25) { '-' } else { '+' };

        write!(input_utc, "{}T{}Z", ndt.date(), ndt.time()).unwrap();
        write!(
            input_with_offset,
            "{}T{}{}{:02}:00",
            ndt.date(),
            ndt.time(),
            offset_sign,
            offset.abs()
        )
        .unwrap();

        assert_eq!(input_utc.len() % DATE_LEN_UTC, 0);
        assert_eq!(input_with_offset.len() % DATE_LEN_WITH_OFFSET, 0);
    }

    assert_eq!(input_utc.len(), DATE_LEN_UTC * BATCH_SIZE);
    assert_eq!(input_with_offset.len(), DATE_LEN_WITH_OFFSET * BATCH_SIZE);

    let mut output = vec![PackedTimestamp::from_value(0); BATCH_SIZE];

    c.benchmark_group("parse_utc")
        .throughput(Throughput::Bytes(
            (input_utc.len() + BATCH_SIZE * std::mem::size_of::<i64>()) as u64,
        ))
        .bench_function("parse_scalar", |b| {
            b.iter(|| bench_parse_scalar(input_utc.as_bytes(), &mut output, DATE_LEN_UTC))
        })
        .bench_function("parse_simd", |b| {
            b.iter(|| bench_parse_simd(input_utc.as_bytes(), &mut output, DATE_LEN_UTC))
        })
        .bench_function("parse_chrono", |b| {
            b.iter(|| bench_parse_chrono(input_utc.as_bytes(), &mut output, DATE_LEN_UTC))
        });

    c.benchmark_group("parse_offset")
        .throughput(Throughput::Bytes(
            (input_with_offset.len() + BATCH_SIZE * std::mem::size_of::<i64>()) as u64,
        ))
        .bench_function("parse_scalar", |b| {
            b.iter(|| {
                bench_parse_scalar(
                    input_with_offset.as_bytes(),
                    &mut output,
                    DATE_LEN_WITH_OFFSET,
                )
            })
        })
        .bench_function("parse_simd", |b| {
            b.iter(|| {
                bench_parse_simd(
                    input_with_offset.as_bytes(),
                    &mut output,
                    DATE_LEN_WITH_OFFSET,
                )
            })
        })
        .bench_function("parse_chrono", |b| {
            b.iter(|| {
                bench_parse_chrono(
                    input_with_offset.as_bytes(),
                    &mut output,
                    DATE_LEN_WITH_OFFSET,
                )
            })
        });
}

criterion_group!(benches, bench_parse);
criterion_main!(benches);
