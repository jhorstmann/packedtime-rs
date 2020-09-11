use criterion::{criterion_group, criterion_main, Criterion};
use packedtime_rs::format_scalar_to_slice;
use packedtime_rs::format_simd_dd_to_slice;
use packedtime_rs::format_simd_mul_to_slice;

pub fn bench_format(c: &mut Criterion) {
    let mut output : Vec<u8> = Vec::with_capacity(24 * 1024);
    unsafe {
        output.set_len(24*1024)
    }

    let slice = output.as_mut_slice();

    c.bench_function("format_simd_dd", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_simd_dd_to_slice(chunk, 2021, 09, 11, 12, 15, 30, 456)
            });
        })
    });
    c.bench_function("format_simd_mul", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_simd_mul_to_slice(chunk, 2021, 09, 11, 12, 15, 30, 456)
            });
        })
    });


    c.bench_function("format_scalar", |b| {
        b.iter(|| {
            slice.chunks_mut(24).for_each(|chunk| {
                format_scalar_to_slice(chunk, 2021, 09, 11, 12, 15, 30, 456)
            });
        })
    });


}


criterion_group!(benches, bench_format);
criterion_main!(benches);
