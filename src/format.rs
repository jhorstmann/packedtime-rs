#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const PATTERN_COMPLETE: &str = "0000-00-00T00:00:00.000Z00:00:00";
const PATTERN_AFTER_YEAR: &str = "-00-00T00:00:00.";

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(PATTERN_COMPLETE.len() == 32);
    assert!(PATTERN_AFTER_YEAR.len() == 16);
};

#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3,sse4.1")]
#[doc(hidden)] // used in benchmarks
pub unsafe fn format_simd_mul_to_slice(
    slice: &mut [u8],
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_mul") };

    let slice = &mut slice[0..24];
    let year = year as i16;
    let month = month as i16;
    let day = day as i16;
    let hour = hour as i16;
    let minute = minute as i16;
    let second = second as i16;
    let millisecond = millisecond as i16;

    let input = _mm_setr_epi16(
        millisecond / 10,
        second,
        minute,
        hour,
        day,
        month,
        year % 100,
        year / 100,
    );

    // divide by 10 by reciprocal multiplication
    let tens = _mm_mulhi_epu16(input, _mm_set1_epi16(52429_u16 as i16));
    let tens = _mm_srli_epi16(tens, 3);

    // remainder of division by 10
    let tens_times10 = _mm_mullo_epi16(tens, _mm_set1_epi16(10));
    let ones = _mm_sub_epi16(input, tens_times10);

    // merge into bytes
    let fmt = _mm_or_si128(_mm_slli_epi16(tens, 8), ones);

    // broadcast to allow room for separators and lanewise shuffle
    let fmt_lo = _mm_shuffle_epi8(
        fmt,
        _mm_set_epi8(-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 0, 1, -1, 2, 3, -1),
    );
    let fmt_hi = _mm_shuffle_epi8(
        fmt,
        _mm_set_epi8(4, 5, -1, 6, 7, -1, 8, 9, -1, 10, 11, -1, 12, 13, 14, 15),
    );

    // insert hundreds of milliseconds now that we have room
    // this is the only instruction in this method that requires sse4.1
    let fmt_lo = _mm_insert_epi8(fmt_lo, (millisecond % 10) as i32, 6);

    // add '0' and separator ascii values
    // let pattern = _mm256_loadu_si256(PATTERN_COMPLETE.as_ptr() as *const __m256i);
    // let pattern_lo = _mm256_extractf128_si256(pattern, 1);
    // let pattern_hi = _mm256_extractf128_si256(pattern, 0);
    let pattern_lo = _mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(16) as *const _);
    let pattern_hi = _mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(0) as *const _);
    let fmt_lo = _mm_or_si128(fmt_lo, pattern_lo);
    let fmt_hi = _mm_or_si128(fmt_hi, pattern_hi);

    _mm_storeu_si128(slice.as_mut_ptr() as *mut __m128i, fmt_hi);
    _mm_storel_epi64(slice.as_mut_ptr().offset(16) as *mut __m128i, fmt_lo);

    //slice[22] = ('0' as u8 + ((millisecond % 10) as u8));
    //unsafe { asm!("#LLVM-MCA-END format_simd_mul") };
}

#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3")]
unsafe fn simd_double_dabble(numbers: &[u16; 8]) -> std::arch::x86_64::__m128i {
    let mut res = _mm_loadu_si128(numbers.as_ptr() as *const _);

    // increment bcd digits which are > 4 by 3
    let lookup_lo = _mm_setr_epi8(0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3);
    // let lookup_hi = _mm_setr_epi8(0, 0, 0, 0, 0, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48);
    let lookup_hi = _mm_slli_epi16(lookup_lo, 4);

    let mask_bcd_lo = _mm_set1_epi16(0x0F00_u16 as i16);
    let mask_bcd_hi = _mm_set1_epi16(0xF000_u16 as i16);

    let mask_bcd = _mm_or_si128(mask_bcd_lo, mask_bcd_hi);

    res = _mm_slli_epi16(res, 3 + 8 - 7);
    for _i in 3..7 {
        let bcd_lo = res;
        let bcd_hi = _mm_srli_epi16(res, 4);

        let inc_lo = _mm_shuffle_epi8(lookup_lo, bcd_lo);
        let inc_hi = _mm_shuffle_epi8(lookup_hi, bcd_hi);

        let inc = _mm_and_si128(_mm_or_si128(inc_lo, inc_hi), mask_bcd);

        res = _mm_add_epi16(res, inc);
        res = _mm_slli_epi16(res, 1);
    }

    // 2 bcd coded digits in hi8 of each 16bit lane
    let rlo = _mm_srli_epi16(_mm_and_si128(res, mask_bcd_lo), 0);
    let rhi = _mm_srli_epi16(_mm_and_si128(res, mask_bcd_hi), 12);

    // bcd coded digits in each byte
    res = _mm_or_si128(rlo, rhi);

    res
}

#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn simd_double_dabble_256(numbers: &[u16; 16]) -> __m256i {
    let mut res = _mm256_loadu_si256(numbers.as_ptr() as *const _);

    // increment bcd digits which are > 4 by 3
    let lookup_lo = _mm_setr_epi8(0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3);
    let lookup_lo = _mm256_broadcastsi128_si256(lookup_lo);
    let lookup_hi = _mm256_slli_epi16(lookup_lo, 4);
    // let lookup_hi = _mm_setr_epi8(0, 0, 0, 0, 0, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48);
    // let lookup_hi = _mm256_broadcastsi128_si256(lookup_hi);
    let mask_bcd_lo = _mm256_set1_epi16(0x0F00_u16 as i16);
    let mask_bcd_hi = _mm256_set1_epi16(0xF000_u16 as i16);
    let mask_bcd = _mm256_or_si256(mask_bcd_lo, mask_bcd_hi);

    res = _mm256_slli_epi16(res, 3 + 8 - 7);
    for _i in 3..7 {
        let bcd_lo = res;
        let bcd_hi = _mm256_srli_epi16(res, 4);

        let inc_lo = _mm256_shuffle_epi8(lookup_lo, bcd_lo);
        let inc_hi = _mm256_shuffle_epi8(lookup_hi, bcd_hi);

        let inc = _mm256_and_si256(_mm256_or_si256(inc_lo, inc_hi), mask_bcd);

        res = _mm256_add_epi16(res, inc);
        res = _mm256_slli_epi16(res, 1);
    }

    // 2 bcd coded digits in hi8 of each 16bit lane
    let rlo = _mm256_srli_epi16(_mm256_and_si256(res, mask_bcd_lo), 0);
    let rhi = _mm256_srli_epi16(_mm256_and_si256(res, mask_bcd_hi), 12);

    // bcd coded digits in each byte
    res = _mm256_or_si256(rlo, rhi);

    res
}

/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: -MM-ddThh:mm:ss.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3")]
unsafe fn format_mmddhhmmss_double_dabble(
    buffer: *mut u8,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
) {
    let mut res = simd_double_dabble(&[0, 0, 0, second, minute, hour, day, month]);

    res = _mm_shuffle_epi8(
        res,
        _mm_set_epi8(-1, 9, 8, -1, 7, 6, -1, 5, 4, -1, 3, 2, -1, 1, 0, -1),
    );
    res = _mm_add_epi8(
        res,
        _mm_loadu_si128(PATTERN_AFTER_YEAR.as_ptr() as *const __m128i),
    );

    _mm_storeu_si128(buffer as *mut __m128i, res);
}

/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: YYYY-MM-ddThh:mm:ss.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3")]
unsafe fn format_yyyymmddhhmm_double_dabble(
    buffer: *mut u8,
    year_hi: u16,
    year_lo: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
) {
    let mut res = simd_double_dabble(&[year_hi, year_lo, month, day, hour, minute, 0, 0]);

    res = _mm_shuffle_epi8(
        res,
        _mm_setr_epi8(0, 1, 2, 3, -1, 4, 5, -1, 6, 7, -1, 8, 9, -1, 10, 11),
    );
    res = _mm_add_epi8(
        res,
        _mm_loadu_si128(PATTERN_COMPLETE.as_ptr() as *const __m128i),
    );

    _mm_storeu_si128(buffer as *mut __m128i, res);
}

/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: YYYY-MM-ddThh:mm:ss.
#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3")]
unsafe fn format_ss_sss_double_dabble(buffer: *mut u8, second: u16, milli_hi: u16, milli_lo: u16) {
    let mut res = simd_double_dabble(&[milli_hi, milli_lo, second, 0, 0, 0, 0, 0]);

    res = _mm_shuffle_epi8(
        res,
        _mm_setr_epi8(-1, 4, 5, -1, 1, 2, 3, -1, -1, -1, -1, -1, -1, -1, -1, -1),
    );
    res = _mm_add_epi8(
        res,
        _mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(16) as *const __m128i),
    );

    // (buffer as *mut i64).write(_mm_extract_epi64(res, 0));
    _mm_storel_epi64(buffer as *mut __m128i, res);
}

#[inline]
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2,ssse3")]
#[doc(hidden)] // used in benchmarks
pub unsafe fn format_simd_dd_to_slice(
    slice: &mut [u8],
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_dd") };

    let slice = &mut slice[0..24];

    format_yyyymmddhhmm_double_dabble(
        slice.as_mut_ptr().add(0),
        (year / 100) as u16,
        (year % 100) as u16,
        month as u16,
        day as u16,
        hour as u16,
        minute as u16,
    );
    format_ss_sss_double_dabble(
        slice.as_mut_ptr().add(16),
        second as u16,
        (millisecond / 100) as u16,
        (millisecond % 100) as u16,
    );

    //unsafe { asm!("#LLVM-MCA-END format_simd_dd") };
}

#[inline]
#[doc(hidden)] // used in benchmarks
pub fn format_scalar_to_slice(
    slice: &mut [u8],
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_scalar") };
    let slice = &mut slice[0..24];

    slice[0] = (b'0' + ((year / 1000) as u8));
    slice[1] = (b'0' + ((year / 100 % 10) as u8));
    slice[2] = (b'0' + ((year / 10 % 10) as u8));
    slice[3] = (b'0' + ((year % 10) as u8));

    slice[4] = b'-';

    slice[5] = (b'0' + ((month / 10) as u8));
    slice[6] = (b'0' + ((month % 10) as u8));

    slice[7] = b'-';

    slice[8] = (b'0' + ((day / 10) as u8));
    slice[9] = (b'0' + ((day % 10) as u8));

    slice[10] = b'T';

    slice[11] = (b'0' + ((hour / 10) as u8));
    slice[12] = (b'0' + ((hour % 10) as u8));

    slice[13] = b':';

    slice[14] = (b'0' + ((minute / 10) as u8));
    slice[15] = (b'0' + ((minute % 10) as u8));

    slice[16] = b':';

    slice[17] = (b'0' + ((second / 10) as u8));
    slice[18] = (b'0' + ((second % 10) as u8));

    slice[19] = b'.';

    slice[20] = (b'0' + ((millisecond / 100 % 10) as u8));
    slice[21] = (b'0' + ((millisecond / 10 % 10) as u8));
    slice[22] = (b'0' + ((millisecond % 10) as u8));

    slice[23] = b'Z';

    //unsafe { asm!("#LLVM-MCA-END format_scalar") };
}

pub fn format_to_rfc3339_utc_bytes(
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
) -> [u8; 24] {
    let mut buffer = [0_u8; 24];
    #[cfg(all(not(miri), target_feature = "sse4.1"))]
    unsafe {
        format_simd_mul_to_slice(
            &mut buffer,
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        );
    }
    #[cfg(not(all(not(miri), target_feature = "sse4.1")))]
    {
        format_scalar_to_slice(
            &mut buffer,
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        );
    }
    buffer
}

#[cfg(test)]
type FormatToSlice = unsafe fn(&mut [u8], u32, u32, u32, u32, u32, u32, u32);

#[cfg(test)]
fn assert_format(
    expected: &str,
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    millisecond: u32,
    f: FormatToSlice,
) {
    let mut buffer = vec![0; 24];

    unsafe {
        f(buffer.as_mut_slice(), year, month, day, hour, minute, second, millisecond);
    }

    let actual = String::from_utf8(buffer).unwrap();

    assert_eq!(expected, &actual);
}

#[cfg(test)]
mod scalar_tests {
    use crate::format::assert_format;
    use crate::format_scalar_to_slice;

    #[test]
    fn test_format_scalar() {
        assert_format(
            "2021-09-10T23:45:31.987Z",
            2021,
            9,
            10,
            23,
            45,
            31,
            987,
            format_scalar_to_slice,
        );
        assert_format(
            "2021-01-01T00:00:00.000Z",
            2021,
            1,
            1,
            0,
            0,
            0,
            0,
            format_scalar_to_slice,
        );
        assert_format(
            "2021-12-31T23:59:60.999Z",
            2021,
            12,
            31,
            23,
            59,
            60,
            999,
            format_scalar_to_slice,
        );
    }
}

#[cfg(test)]
#[cfg(all(
    not(miri),
    target_arch = "x86_64",
    target_feature = "sse2",
    target_feature = "ssse3",
    target_feature = "sse4.1"
))]
mod simd_tests {
    use crate::format::assert_format;
    use crate::{format_simd_dd_to_slice, format_simd_mul_to_slice};

    #[test]
    fn test_format_simd_dd() {
        assert_format(
            "2021-09-10T23:45:31.987Z",
            2021,
            09,
            10,
            23,
            45,
            31,
            987,
            format_simd_dd_to_slice,
        );
        assert_format(
            "2021-01-01T00:00:00.000Z",
            2021,
            1,
            1,
            0,
            0,
            0,
            0,
            format_simd_dd_to_slice,
        );
        assert_format(
            "2021-12-31T23:59:60.999Z",
            2021,
            12,
            31,
            23,
            59,
            60,
            999,
            format_simd_dd_to_slice,
        );
    }

    #[test]
    fn test_format_simd_mul() {
        assert_format(
            "2021-09-10T23:45:31.987Z",
            2021,
            09,
            10,
            23,
            45,
            31,
            987,
            format_simd_mul_to_slice,
        );
        assert_format(
            "2021-01-01T00:00:00.000Z",
            2021,
            1,
            1,
            0,
            0,
            0,
            0,
            format_simd_mul_to_slice,
        );
        assert_format(
            "2021-12-31T23:59:60.999Z",
            2021,
            12,
            31,
            23,
            59,
            60,
            999,
            format_simd_mul_to_slice,
        );
    }
}
