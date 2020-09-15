use criterion::{criterion_group, criterion_main, Criterion};
use packedtime_rs::{parse_scalar, Timestamp, parse_simd};
use static_assertions::_core::str::from_utf8_unchecked;

pub fn bench_parse(c: &mut Criterion) {
    let mut output1 : Vec<Timestamp> = Vec::with_capacity(1024);
    let mut output2 : Vec<Timestamp> = Vec::with_capacity(1024);

    let mut input : Vec<u8> = Vec::with_capacity(24* 1024);
    for _i in 0..1024 {
        input.extend_from_slice("2345-12-24T17:30:15.123Z".as_bytes())
    }

    c.bench_function("parse_scalar", |b| {
        b.iter(|| {
            output1.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let ts = parse_scalar(s).unwrap();
                output1.push(ts);
            });
        })
    });

    c.bench_function("parse_simd", |b| {
        b.iter(|| {
            output2.clear();
            input.chunks(24).for_each(|chunk| {
                let s = unsafe {std::str::from_utf8_unchecked(chunk) };
                let ts = parse_simd(s).unwrap();
                output2.push(ts);
            });
        })
    });

}


criterion_group!(benches, bench_parse);
criterion_main!(benches);
