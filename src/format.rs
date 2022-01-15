use std::arch::x86_64::{__m128i, __m256i};
use std::arch::x86_64::*;
use crate::util::{debug_m128, debug_m256};

const PATTERN_COMPLETE: &str = "0000-00-00T00:00:00.000Z00:00:00";
const PATTERN_AFTER_YEAR: &str = "-00-00T00:00:00.";

const _: () = {
    assert!(PATTERN_COMPLETE.len() == 32);
    assert!(PATTERN_AFTER_YEAR.len() == 16);
};


#[inline]
pub fn format_simd_mul_to_slice(slice: &mut [u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_mul") };

    let slice = &mut slice[0..24];
    let year = year as i16;
    let month = month as i16;
    let day = day as i16;
    let hour = hour as i16;
    let minute = minute as i16;
    let second = second as i16;
    let millisecond = millisecond as i16;

    unsafe {
        let input = std::arch::x86_64::_mm_setr_epi16(millisecond / 10, second, minute, hour, day, month, year % 100, year / 100);

        // divide by 10 by reciprocal multiplication
        let tens = std::arch::x86_64::_mm_mulhi_epu16(input, std::arch::x86_64::_mm_set1_epi16(52429_u16 as i16));
        let tens = std::arch::x86_64::_mm_srli_epi16(tens, 3);

        // remainder of division by 10
        let tens_times10 = std::arch::x86_64::_mm_mullo_epi16(tens, std::arch::x86_64::_mm_set1_epi16(10));
        let ones = std::arch::x86_64::_mm_sub_epi16(input, tens_times10);

        // merge into bytes
        let fmt = std::arch::x86_64::_mm_or_si128(std::arch::x86_64::_mm_slli_epi16(tens, 8), ones);

        // broadcast to allow room for separators and lanewise shuffle
        let fmt_lo = std::arch::x86_64::_mm_shuffle_epi8(fmt, std::arch::x86_64::_mm_set_epi8(
            -1, -1, -1,  -1, -1, -1, -1, -1, -1, -1, 0, 1, -1, 2, 3, -1,
        ));
        let fmt_hi = std::arch::x86_64::_mm_shuffle_epi8(fmt, std::arch::x86_64::_mm_set_epi8(
            4, 5, -1, 6, 7, -1, 8, 9, -1, 10, 11, -1, 12, 13, 14, 15,
        ));

        // insert hundreds of milliseconds now that we have room
        let fmt_lo = std::arch::x86_64::_mm_insert_epi8(fmt_lo, (millisecond % 10) as i32, 6);

        // add '0' and separator ascii values
        // let pattern = std::arch::x86_64::_mm256_loadu_si256(PATTERN_COMPLETE.as_ptr() as *const __m256i);
        // let pattern_lo = std::arch::x86_64::_mm256_extractf128_si256(pattern, 1);
        // let pattern_hi = std::arch::x86_64::_mm256_extractf128_si256(pattern, 0);
        let pattern_lo = std::arch::x86_64::_mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(16) as *const _);
        let pattern_hi = std::arch::x86_64::_mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(0) as *const _);
        let fmt_lo = std::arch::x86_64::_mm_or_si128(fmt_lo, pattern_lo);
        let fmt_hi = std::arch::x86_64::_mm_or_si128(fmt_hi, pattern_hi);

        std::arch::x86_64::_mm_storeu_si128(slice.as_mut_ptr() as *mut __m128i, fmt_hi);
        std::arch::x86_64::_mm_storel_epi64(slice.as_mut_ptr().offset(16) as *mut __m128i, fmt_lo);

        //slice[22] = ('0' as u8 + ((millisecond % 10) as u8));
    }
    //unsafe { asm!("#LLVM-MCA-END format_simd_mul") };
}

#[inline(always)]
unsafe fn simd_double_dabble(numbers: &[u16; 8]) -> std::arch::x86_64::__m128i {
    let mut res = std::arch::x86_64::_mm_loadu_si128(numbers.as_ptr() as *const _);

    // increment bcd digits which are > 4 by 3
    let lookup_lo = std::arch::x86_64::_mm_setr_epi8(0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3);
    // let lookup_hi = std::arch::x86_64::_mm_setr_epi8(0, 0, 0, 0, 0, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48);
    let lookup_hi = std::arch::x86_64::_mm_slli_epi16(lookup_lo, 4);

    let mask_bcd_lo = std::arch::x86_64::_mm_set1_epi16(0x0F00_u16 as i16);
    let mask_bcd_hi = std::arch::x86_64::_mm_set1_epi16(0xF000_u16 as i16);

    let mask_bcd = std::arch::x86_64::_mm_or_si128(mask_bcd_lo, mask_bcd_hi);

    res = std::arch::x86_64::_mm_slli_epi16(res, 3 + 8 - 7);
    for _i in 3..7 {
        let bcd_lo = res;
        let bcd_hi = std::arch::x86_64::_mm_srli_epi16(res, 4);

        let inc_lo = std::arch::x86_64::_mm_shuffle_epi8(lookup_lo, bcd_lo);
        let inc_hi = std::arch::x86_64::_mm_shuffle_epi8(lookup_hi, bcd_hi);

        let inc = std::arch::x86_64::_mm_and_si128(std::arch::x86_64::_mm_or_si128(inc_lo, inc_hi), mask_bcd);

        res = std::arch::x86_64::_mm_add_epi16(res, inc);
        res = std::arch::x86_64::_mm_slli_epi16(res, 1);
    }

    // 2 bcd coded digits in hi8 of each 16bit lane
    let rlo = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, mask_bcd_lo), 0);
    let rhi = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, mask_bcd_hi), 12);

    // bcd coded digits in each byte
    res = std::arch::x86_64::_mm_or_si128(rlo, rhi);

    res
}

#[inline(always)]
unsafe fn simd_double_dabble_256(numbers: &[u16; 16]) -> std::arch::x86_64::__m256i {
    let mut res = std::arch::x86_64::_mm256_loadu_si256(numbers.as_ptr() as *const _);

    // increment bcd digits which are > 4 by 3
    let lookup_lo = std::arch::x86_64::_mm_setr_epi8(0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3);
    let lookup_lo = std::arch::x86_64::_mm256_broadcastsi128_si256(lookup_lo);
    let lookup_hi = std::arch::x86_64::_mm256_slli_epi16(lookup_lo, 4);
    // let lookup_hi = std::arch::x86_64::_mm_setr_epi8(0, 0, 0, 0, 0, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48);
    // let lookup_hi = std::arch::x86_64::_mm256_broadcastsi128_si256(lookup_hi);
    let mask_bcd_lo = std::arch::x86_64::_mm256_set1_epi16(0x0F00_u16 as i16);
    let mask_bcd_hi = std::arch::x86_64::_mm256_set1_epi16(0xF000_u16 as i16);
    let mask_bcd = std::arch::x86_64::_mm256_or_si256(mask_bcd_lo, mask_bcd_hi);

    res = std::arch::x86_64::_mm256_slli_epi16(res, 3 + 8 - 7);
    for _i in 3..7 {
        let bcd_lo = res;
        let bcd_hi = std::arch::x86_64::_mm256_srli_epi16(res, 4);

        let inc_lo = std::arch::x86_64::_mm256_shuffle_epi8(lookup_lo, bcd_lo);
        let inc_hi = std::arch::x86_64::_mm256_shuffle_epi8(lookup_hi, bcd_hi);

        let inc = std::arch::x86_64::_mm256_and_si256(std::arch::x86_64::_mm256_or_si256(inc_lo, inc_hi), mask_bcd);

        res = std::arch::x86_64::_mm256_add_epi16(res, inc);
        res = std::arch::x86_64::_mm256_slli_epi16(res, 1);
    }

    // 2 bcd coded digits in hi8 of each 16bit lane
    let rlo = std::arch::x86_64::_mm256_srli_epi16(std::arch::x86_64::_mm256_and_si256(res, mask_bcd_lo), 0);
    let rhi = std::arch::x86_64::_mm256_srli_epi16(std::arch::x86_64::_mm256_and_si256(res, mask_bcd_hi), 12);

    // bcd coded digits in each byte
    res = std::arch::x86_64::_mm256_or_si256(rlo, rhi);

    res
}


/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: -MM-ddThh:mm:ss.
#[inline(always)]
unsafe fn format_mmddhhmmss_double_dabble(buffer: *mut u8, month: u16, day: u16, hour: u16, minute: u16, second: u16) {
    let mut res = simd_double_dabble(&[0, 0, 0, second, minute, hour, day, month]);

    res = std::arch::x86_64::_mm_shuffle_epi8(res, std::arch::x86_64::_mm_set_epi8(-1, 9, 8, -1, 7, 6, -1, 5, 4, -1, 3, 2, -1, 1, 0, -1));
    res = std::arch::x86_64::_mm_add_epi8(res, std::arch::x86_64::_mm_loadu_si128(PATTERN_AFTER_YEAR.as_ptr() as *const __m128i));

    std::arch::x86_64::_mm_storeu_si128(buffer as *mut __m128i, res);
}

/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: YYYY-MM-ddThh:mm:ss.
#[inline(always)]
unsafe fn format_yyyymmddhhmm_double_dabble(buffer: *mut u8, year_hi: u16, year_lo: u16, month: u16, day: u16, hour: u16, minute: u16) {
    let mut res = simd_double_dabble(&[year_hi, year_lo, month, day, hour, minute, 0, 0]);

    res = std::arch::x86_64::_mm_shuffle_epi8(res, std::arch::x86_64::_mm_setr_epi8(0, 1, 2, 3, -1, 4, 5, -1, 6, 7, -1, 8, 9, -1, 10, 11));
    res = std::arch::x86_64::_mm_add_epi8(res, std::arch::x86_64::_mm_loadu_si128(PATTERN_COMPLETE.as_ptr() as *const __m128i));

    std::arch::x86_64::_mm_storeu_si128(buffer as *mut __m128i, res);
}

/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: YYYY-MM-ddThh:mm:ss.
#[inline(always)]
unsafe fn format_ssSSS_double_dabble(buffer: *mut u8, second: u16, milli_hi: u16, milli_lo: u16) {
    let mut res = simd_double_dabble(&[milli_hi, milli_lo, second, 0, 0, 0, 0, 0]);

    res = std::arch::x86_64::_mm_shuffle_epi8(res, std::arch::x86_64::_mm_setr_epi8(-1, 4, 5, -1, 1, 2,3, -1, -1, -1, -1, -1, -1, -1, -1, -1));
    res = std::arch::x86_64::_mm_add_epi8(res, std::arch::x86_64::_mm_loadu_si128(PATTERN_COMPLETE.as_ptr().add(16) as *const __m128i));

    // (buffer as *mut i64).write(std::arch::x86_64::_mm_extract_epi64(res, 0));
    std::arch::x86_64::_mm_storel_epi64(buffer as *mut __m128i, res);
}

#[inline]
pub fn format_simd_dd_to_slice(slice: &mut[u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_dd") };

    let slice = &mut slice[0..24];

    // slice[0] = ('0' as u8 + ((year / 1000) as u8));
    // slice[1] = ('0' as u8 + ((year / 100 % 10) as u8));
    // slice[2] = ('0' as u8 + ((year / 10 % 10) as u8));
    // slice[3] = ('0' as u8 + ((year % 10) as u8));

/*
    slice[16] = ':' as u8;
    slice[17] = ('0' as u8 + (second / 10) as u8);
    slice[18] = ('0' as u8 + (second % 10) as u8);
    slice[19] = '.' as u8;

    slice[20] = ('0' as u8 + ((millisecond / 100 % 10) as u8));
    slice[21] = ('0' as u8 + ((millisecond / 10 % 10) as u8));
    slice[22] = ('0' as u8 + ((millisecond % 10) as u8));

    slice[23] = ('Z' as u8);
*/

    unsafe {
        format_yyyymmddhhmm_double_dabble(slice.as_mut_ptr().add(0), (year / 100) as u16, (year % 100) as u16, month as u16, day as u16, hour as u16, minute as u16);
        format_ssSSS_double_dabble(slice.as_mut_ptr().add(16), second as u16, (millisecond / 100) as u16, (millisecond % 100) as u16);
    };

    //unsafe { asm!("#LLVM-MCA-END format_simd_dd") };
}

#[inline]
pub fn format_scalar_to_slice(slice: &mut [u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_scalar") };
    let slice = &mut slice[0..24];

    slice[0] = ('0' as u8 + ((year / 1000) as u8));
    slice[1] = ('0' as u8 + ((year / 100 % 10) as u8));
    slice[2] = ('0' as u8 + ((year / 10 % 10) as u8));
    slice[3] = ('0' as u8 + ((year % 10) as u8));

    slice[4] = '-' as u8;

    slice[5] = ('0' as u8 + ((month / 10) as u8));
    slice[6] = ('0' as u8 + ((month % 10) as u8));

    slice[7] = '-' as u8;

    slice[8] = ('0' as u8 + ((day / 10) as u8));
    slice[9] = ('0' as u8 + ((day % 10) as u8));

    slice[10] = 'T' as u8;

    slice[11] = ('0' as u8 + ((hour / 10) as u8));
    slice[12] = ('0' as u8 + ((hour % 10) as u8));

    slice[13] = ':' as u8;

    slice[14] = ('0' as u8 + ((minute / 10) as u8));
    slice[15] = ('0' as u8 + ((minute % 10) as u8));

    slice[16] = ':' as u8;

    slice[17] = ('0' as u8 + ((second / 10) as u8));
    slice[18] = ('0' as u8 + ((second % 10) as u8));

    slice[19] = '.' as u8;

    slice[20] = ('0' as u8 + ((millisecond / 100 % 10) as u8));
    slice[21] = ('0' as u8 + ((millisecond / 10 % 10) as u8));
    slice[22] = ('0' as u8 + ((millisecond % 10) as u8));

    slice[23] = ('Z' as u8);

    //unsafe { asm!("#LLVM-MCA-END format_scalar") };
}


#[cfg(test)]
pub mod tests {
    use crate::{format_simd_mul_to_slice, format_simd_dd_to_slice, format_scalar_to_slice};

    type FormatToSlice = unsafe fn(&mut [u8], u32, u32, u32, u32, u32, u32, u32);

    fn assert_format(expected: &str, year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32, f: FormatToSlice) {
        let mut buffer: Vec<u8> = Vec::with_capacity(32);

        unsafe { buffer.set_len(24) };

        let slice = &mut buffer.as_mut_slice()[0..24];

        unsafe { f(slice, year, month, day, hour, minute, second, millisecond); }

        let actual = String::from_utf8(buffer).unwrap();

        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_format_scalar() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_scalar_to_slice);
        assert_format("2021-01-01T00:00:00.000Z", 2021, 1, 1, 0, 0, 0, 0, format_scalar_to_slice);
        assert_format("2021-12-31T23:59:60.999Z", 2021, 12, 31, 23, 59, 60, 999, format_scalar_to_slice);
    }

    #[test]
    fn test_format_simd_dd() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_simd_dd_to_slice);
        assert_format("2021-01-01T00:00:00.000Z", 2021, 1, 1, 0, 0, 0, 0, format_simd_dd_to_slice);
        assert_format("2021-12-31T23:59:60.999Z", 2021, 12, 31, 23, 59, 60, 999, format_simd_dd_to_slice);
    }

    #[test]
    fn test_format_simd_mul() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_simd_mul_to_slice);
        assert_format("2021-01-01T00:00:00.000Z", 2021, 1, 1, 0, 0, 0, 0, format_simd_mul_to_slice);
        assert_format("2021-12-31T23:59:60.999Z", 2021, 12, 31, 23, 59, 60, 999, format_simd_mul_to_slice);
    }
}
