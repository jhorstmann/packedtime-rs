use criterion::{criterion_group, criterion_main, Criterion, black_box};
use packedtime_rs::format_scalar_to_slice;
use packedtime_rs::format_simd_dd_to_slice;
use packedtime_rs::format_simd_mul_to_slice;

pub fn bench_format(c: &mut Criterion) {
    let mut output : Vec<u8> = Vec::with_capacity(24 * 1024);
    unsafe {
        output.set_len(24*1024)
    }

    let slice = output.as_mut_slice();

    let (year, month, day, hour, minute, second, milli) = black_box((2021, 09, 11, 12, 15, 30, 456));

    c.bench_function("format_simd_dd", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_simd_dd_to_slice(chunk, year, month, day, hour, minute, second, milli)
            });
        })
    });
    c.bench_function("format_simd_mul", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_simd_mul_to_slice(chunk, year, month, day, hour, minute, second, milli)
            });
        })
    });


    c.bench_function("format_scalar", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_scalar_to_slice(chunk, year, month, day, hour, minute, second, milli)
            });
        })
    });


}


criterion_group!(benches, bench_format);
criterion_main!(benches);
