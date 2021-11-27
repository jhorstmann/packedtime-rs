use criterion::{criterion_group, criterion_main, Criterion};
use packedtime_rs::{parse_to_epoch_millis_scalar, parse_to_epoch_millis_simd, ParseResult};
use static_assertions::_core::str::from_utf8_unchecked;

use std::str::FromStr;
use chrono::FixedOffset;

pub fn bench_parse(c: &mut Criterion) {
    let mut output1 : Vec<i64> = Vec::with_capacity(1024);
    let mut output2 : Vec<i64> = Vec::with_capacity(1024);
    let mut output3 : Vec<i64> = Vec::with_capacity(1024);
    let mut output4 : Vec<iso8601::DateTime> = Vec::with_capacity(1024);

    let mut input : Vec<u8> = Vec::with_capacity(24* 1024);
    for _i in 0..1024 {
        input.extend_from_slice("2345-12-24T17:30:15.123Z".as_bytes())
    }

    c.bench_function("parse_scalar", |b| {
        b.iter(|| {
            output1.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let ts = parse_to_epoch_millis_scalar(s).unwrap();
                output1.push(ts);
            });
        })
    });

    c.bench_function("parse_simd", |b| {
        b.iter(|| {
            output2.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let ts = parse_to_epoch_millis_simd(s).unwrap();
                output2.push(ts);
            });
        })
    });

    c.bench_function("parse_chrono", |b| {
        b.iter(|| {
            output3.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let ts = chrono::DateTime::parse_from_rfc3339(s).unwrap();
                output3.push(ts.timestamp_millis());
            });
        })
    });

    c.bench_function("parse_iso8601", |b| {
        b.iter(|| {
            output4.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let datetime = iso8601::DateTime::from_str(s).unwrap();
                output4.push(datetime);
            });
        })
    });

}


criterion_group!(benches, bench_parse);
criterion_main!(benches);
