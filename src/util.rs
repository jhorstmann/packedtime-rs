use std::arch::x86_64::{__m128i, __m256i};

pub(crate) fn debug_m128(reg: __m128i) {
    let lo: u64 = unsafe { std::arch::x86_64::_mm_extract_epi64(reg, 0) as u64 };
    let hi: u64 = unsafe { std::arch::x86_64::_mm_extract_epi64(reg, 1) as u64 };

    eprintln!("{:016X}{:016X}", hi, lo);
}

pub(crate) fn debug_m256(reg: __m256i) {
    let a: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 0) as u64 };
    let b: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 1) as u64 };
    let c: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 2) as u64 };
    let d: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 3) as u64 };

    eprintln!("{:016X}{:016X}{:016X}{:016X}", d, c, b, a);
}
