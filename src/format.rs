use std::arch::x86_64::{__m128i, __m256i};
use std::arch::x86_64::*;
use crate::util::{debug_m128, debug_m256};

const PATTERN_COMPLETE: &str = "0000-00-00T00:00:00.000Z00:00:00";
const PATTERN_AFTER_YEAR: &str = "-00-00T00:00:00.";

static_assertions::const_assert!(PATTERN_COMPLETE.len() == 32);
static_assertions::const_assert!(PATTERN_AFTER_YEAR.len() == 16);

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
        //let fmt = std::arch::x86_64::_mm256_or_si256(fmt, std::arch::x86_64::_mm256_loadu_si256(PATTERN_COMPLETE.as_ptr() as *const __m256i));
        let pattern = std::arch::x86_64::_mm256_loadu_si256(PATTERN_COMPLETE.as_ptr() as *const __m256i);
        let fmt_lo = std::arch::x86_64::_mm_or_si128(fmt_lo, std::arch::x86_64::_mm256_extractf128_si256(pattern, 1));
        let fmt_hi = std::arch::x86_64::_mm_or_si128(fmt_hi, std::arch::x86_64::_mm256_extractf128_si256(pattern, 0));

        //let mask = std::arch::x86_64::_mm256_set_epi32(0, -1, -1, -1, -1, -1, -1, -1);
        //std::arch::x86_64::_mm256_maskstore_epi32(slice.as_mut_ptr() as *mut i32, mask, fmt);
        std::arch::x86_64::_mm_storeu_si128(slice.as_mut_ptr() as *mut __m128i, fmt_hi);
        std::arch::x86_64::_mm_storel_epi64(slice.as_mut_ptr().offset(16) as *mut __m128i, fmt_lo);

        //slice[22] = ('0' as u8 + ((millisecond % 10) as u8));
    }
    //unsafe { asm!("#LLVM-MCA-END format_simd_mul") };
}



/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: -MM-ddThh:mm:ss.
unsafe fn format_mmddhhmmss_double_dabble(buffer: *mut u8, month: i16, day: i16, hour: i16, minute: i16, second: i16) {
    let mut res = std::arch::x86_64::_mm_set_epi16(0, 0, 0, second, minute, hour, day, month);

    // magic double-dabble
    res = std::arch::x86_64::_mm_slli_epi16(res, 3+8-6);
    for _i in 3..6 {
        let mask3 = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0x8800_u16 as i16)), 3);
        let mask2 = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0x4400_u16 as i16)), 2);
        let mask1 = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0x2200_u16 as i16)), 1);
        let mask0 = std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0x1100));

        // increment bcd digits which are > 4 by 3
        // > 4 means either bit 4 is set or (bit 3 and at least one of bit 2 or bit 1 is set)
        let mask = std::arch::x86_64::_mm_or_si128(mask3, std::arch::x86_64::_mm_and_si128(mask2, std::arch::x86_64::_mm_or_si128(mask1, mask0)));
        let inc = std::arch::x86_64::_mm_add_epi16(mask, std::arch::x86_64::_mm_slli_epi16(mask, 1));

        res = std::arch::x86_64::_mm_add_epi16(res, inc);
        res = std::arch::x86_64::_mm_slli_epi16(res, 1);
    }

    // bcd coded digits in hi8 of each 16bit lane

    let rlo = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0x0F00_u16 as i16)), 0);
    let rhi = std::arch::x86_64::_mm_srli_epi16(std::arch::x86_64::_mm_and_si128(res, std::arch::x86_64::_mm_set1_epi16(0xF000_u16 as i16)), 12);

    res = std::arch::x86_64::_mm_or_si128(rlo, rhi);
    res = std::arch::x86_64::_mm_shuffle_epi8(res, std::arch::x86_64::_mm_set_epi8(-1, 9, 8, -1, 7, 6, -1, 5, 4, -1, 3, 2, -1, 1, 0, -1));
    res = std::arch::x86_64::_mm_add_epi8(res, std::arch::x86_64::_mm_loadu_si128(PATTERN_AFTER_YEAR.as_ptr() as *const __m128i));

    std::arch::x86_64::_mm_storeu_si128(buffer as *mut __m128i, res);
}

pub fn format_simd_dd_to_slice(slice: &mut[u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_dd") };

    let slice = &mut slice[0..24];

    slice[0] = ('0' as u8 + ((year / 1000) as u8));
    slice[1] = ('0' as u8 + ((year / 100 % 10) as u8));
    slice[2] = ('0' as u8 + ((year / 10 % 10) as u8));
    slice[3] = ('0' as u8 + ((year % 10) as u8));

    slice[20] = ('0' as u8 + ((millisecond / 100 % 10) as u8));
    slice[21] = ('0' as u8 + ((millisecond / 10 % 10) as u8));
    slice[22] = ('0' as u8 + ((millisecond % 10) as u8));

    slice[23] = ('Z' as u8);

    unsafe {
        format_mmddhhmmss_double_dabble(slice.as_mut_ptr().offset(4), month as i16, day as i16, hour as i16, minute as i16, second as i16);
    };

    //unsafe { asm!("#LLVM-MCA-END format_simd_dd") };
}

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
