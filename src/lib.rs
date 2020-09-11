//#![feature(asm)]

#[derive(Debug, PartialEq)]
enum ParseError {
    InvalidLen(usize),
    InvalidChar(usize),

}

type ParseResult<T> = std::result::Result<T, ParseError>;

use std::arch::x86_64::{__m128i, __m256i};
use std::fmt::{Display, Debug};
use static_assertions::_core::fmt::Formatter;

//   MMMMdddddhhhhhmmmmmmssssss
// MMMMdddddhhhhhmmmmmmssssss00
// 3210765432107654321076543210

const OFFSET_BITS: u32 = 12;
const MILLI_BITS: u32 = 10;
const SECOND_BITS: u32 = 6;
const MINUTE_BITS: u32 = 6;
const HOUR_BITS: u32 = 5;
const DAY_BITS: u32 = 5;
const MONTH_BITS: u32 = 4;
const YEAR_BITS: u32 =
    64 - (MONTH_BITS + DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS);

const MIN_YEAR_INTERNAL: i32 = -(1 << (YEAR_BITS - 1));
const MAX_YEAR_INTERNAL: i32 = (1 << (YEAR_BITS - 1)) - 1;
const MIN_YEAR: i32 = -9999;
const MAX_YEAR: i32 = 9999;

const MIN_OFFSET_MINUTES_INTERNAL: i32 = -(1 << (OFFSET_BITS - 1));
const MAX_OFFSET_MINUTES_INTERNAL: i32 = (1 << (OFFSET_BITS - 1)) - 1;

const MAX_OFFSET_HOURS: i32 = 18;
const MIN_OFFSET_HOURS: i32 = -18;
const MIN_OFFSET_MINUTES: i32 = MIN_OFFSET_HOURS * 60;
const MAX_OFFSET_MINUTES: i32 = MAX_OFFSET_HOURS * 60;

static_assertions::const_assert!(MIN_YEAR_INTERNAL < MIN_YEAR || MAX_YEAR_INTERNAL > MAX_YEAR);
static_assertions::const_assert!(
    MIN_OFFSET_MINUTES_INTERNAL < MIN_OFFSET_MINUTES
        || MAX_OFFSET_MINUTES_INTERNAL > MAX_OFFSET_MINUTES
);

#[derive(PartialEq, Clone, Copy)]
pub struct Packedtime(u64);

impl Packedtime {
    pub fn new_utc(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        milli: u32,
    ) -> Self {
        let value = (((((((year as u64) << MONTH_BITS | month as u64) << DAY_BITS | day as u64)
            << HOUR_BITS
            | hour as u64)
            << MINUTE_BITS
            | minute as u64)
            << SECOND_BITS
            | second as u64)
            << MILLI_BITS
            | milli as u64)
            << OFFSET_BITS;
        Self(value)
    }

    pub fn year(&self) -> u32 {
        (self.0 >> (MONTH_BITS + DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) as u32
    }

    pub fn month(&self) -> u32 {
        ((self.0 >> (DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MONTH_BITS) - 1)) as u32
    }

    pub fn day(&self) -> u32 {
        ((self.0 >> (HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << DAY_BITS) - 1)) as u32
    }

    pub fn hour(&self) -> u32 {
        ((self.0 >> (MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << HOUR_BITS) - 1)) as u32
    }

    pub fn minute(&self) -> u32 {
        ((self.0 >> (SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MINUTE_BITS) - 1)) as u32
    }

    pub fn second(&self) -> u32 {
        ((self.0 >> (MILLI_BITS + OFFSET_BITS)) & ((1 << SECOND_BITS) - 1)) as u32
    }

    pub fn millisecond(&self) -> u32 {
        ((self.0 >> (OFFSET_BITS)) & ((1 << MILLI_BITS) - 1)) as u32
    }

    pub fn format(&self) -> String {
        let mut buffer: Vec<u8> = Vec::with_capacity(24);

        unsafe {buffer.set_len(24) };

        let slice = &mut buffer.as_mut_slice()[0..24];

        format_simd_dd_to_slice(slice, self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond());

        #[cfg(not(debug_assertions))]
        return unsafe { String::from_utf8_unchecked(buffer) };
        #[cfg(debug_assertions)]
        return String::from_utf8(buffer).unwrap();
    }
}

impl Display for Packedtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond()))
    }
}

impl Debug for Packedtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <Packedtime as Display>::fmt(&self, f)
    }
}


fn debug_m128(reg: __m128i) {
    let lo: u64 = unsafe { std::arch::x86_64::_mm_extract_epi64(reg, 0) as u64 };
    let hi: u64 = unsafe { std::arch::x86_64::_mm_extract_epi64(reg, 1) as u64 };

    eprintln!("{:016X}{:016X}", hi, lo);
}

fn debug_m256(reg: __m256i) {
    let a: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 0) as u64 };
    let b: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 1) as u64 };
    let c: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 2) as u64 };
    let d: u64 = unsafe { std::arch::x86_64::_mm256_extract_epi64(reg, 3) as u64 };

    eprintln!("{:016X}{:016X}{:016X}{:016X}", d, c, b, a);
}

pub fn format_simd_mul_to_slice(slice: &mut [u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_mul") };

    let slice = &mut slice[0..24];
    let year = year as i32;
    let month = month as i32;
    let day = day as i32;
    let hour = hour as i32;
    let minute = minute as i32;
    let second = second as i32;
    let millisecond = millisecond as i32;

    unsafe {
        let input = std::arch::x86_64::_mm256_setr_epi32(millisecond / 10, second, minute, hour, day, month, year % 100, year / 100);

        // divide by 10 by reciprocal multiplication
        let tens = std::arch::x86_64::_mm256_mullo_epi32(input, std::arch::x86_64::_mm256_set1_epi32(52429));
        let tens = std::arch::x86_64::_mm256_srli_epi32(tens, 19);

        //let tens_times10 = std::arch::x86_64::_mm256_mullo_epi32(tens, std::arch::x86_64::_mm256_set1_epi32(10));

        let tens_times2 =  std::arch::x86_64::_mm256_add_epi32(tens, tens);
        let tens_times3 =  std::arch::x86_64::_mm256_add_epi32(tens_times2, tens);
        let tens_times5 =  std::arch::x86_64::_mm256_add_epi32(tens_times2, tens_times3);
        let tens_times10 =  std::arch::x86_64::_mm256_add_epi32(tens_times5, tens_times5);

        let ones = std::arch::x86_64::_mm256_sub_epi32(input, tens_times10);

        let fmt = std::arch::x86_64::_mm256_or_si256(std::arch::x86_64::_mm256_slli_epi32(tens, 16), ones);
        let fmt = std::arch::x86_64::_mm_packus_epi16(std::arch::x86_64::_mm256_extractf128_si256(fmt, 0), std::arch::x86_64::_mm256_extractf128_si256(fmt, 1));
        let fmt = std::arch::x86_64::_mm256_broadcastsi128_si256(fmt);
        let fmt = std::arch::x86_64::_mm256_shuffle_epi8(fmt, std::arch::x86_64::_mm256_set_epi8(
            -1, -1, -1,  -1, -1, -1, -1, -1, -1, -1, 0, 1, -1, 2, 3, -1,
            4, 5, -1, 6, 7, -1, 8, 9, -1, 10, 11, -1, 12, 13, 14, 15,
        ));

        let fmt = std::arch::x86_64::_mm256_add_epi8(fmt, std::arch::x86_64::_mm256_loadu_si256("0000-00-00T00:00:00.000Z".as_ptr() as *const __m256i));

        let mask = std::arch::x86_64::_mm256_insert_epi64(std::arch::x86_64::_mm256_set1_epi64x(-1), 0, 7);
        std::arch::x86_64::_mm256_maskstore_epi32(slice.as_mut_ptr() as *mut i32, mask, fmt);

        slice[22] = ('0' as u8 + ((millisecond % 10) as u8));
    }
    //unsafe { asm!("#LLVM-MCA-END format_simd_mul") };
}



/// formats the timestamp into the output buffer including separator chars, starting with the dash before the month and ending with a dot after the seconds.
/// Example: -MM-ddThh:mm:ss.
unsafe fn format_mmddhhmmss_double_dabble(buffer: *mut u8, month: i16, day: i16, hour: i16, minute: i16, second: i16) {
    let mut res = std::arch::x86_64::_mm_set_epi16(0, 0, 0, second, minute, hour, day, month);

    // magic double-dabble
    res = std::arch::x86_64::_mm_slli_epi16(res, 3);
    for _i in 3..8 {
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
    res = std::arch::x86_64::_mm_add_epi8(res, std::arch::x86_64::_mm_loadu_si128("-00-00T00:00:00.".as_ptr() as *const __m128i));

    std::arch::x86_64::_mm_storeu_si128(buffer as *mut __m128i, res);
}

pub fn format_simd_dd_to_slice(slice: &mut[u8], year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32) {
    //unsafe { asm!("#LLVM-MCA-BEGIN format_simd_dd") };

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



fn parse_seconds_and_millis(bytes: &[u8]) -> ParseResult<(u32, u32)> {
    unimplemented!()
}

fn parse(input: &str) -> ParseResult<Packedtime> {
    //2020-09-09T15:05:45Z"
    //2020-09-09T15:05:45.123456789Z

    if input.len() < 16 {
        return Err(ParseError::InvalidLen(input.len()));
    }

    let bytes = input.as_bytes();
    const MIN_BYTES: &[u8] = "))))-)0-)0S))9))9))".as_bytes();
    const MAX_BYTES: &[u8] = "@@@@-2@-4@U3@;6@;6@".as_bytes();
    const SPACE_SEP_BYTES: &[u8] = "0000-00-00 00:00:00".as_bytes();
    //const NUM_MASK : &[u8] = [0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0x00, 0xFF, 0xFF].as_ref();
    // 0123-01-23T45:670123-01-23T45:67
    // 0123-01-23T45:670123-01-23T45:67
    //const SHUFFLE : &[u8] = [   ,255,255,3,2,1, 0, 255, 255].as_ref();

    unsafe {
        let ts_without_seconds = std::arch::x86_64::_mm_loadu_si128(bytes.as_ptr() as *const __m128i);
        let min = std::arch::x86_64::_mm_loadu_si128(MIN_BYTES.as_ptr() as *const __m128i);
        let max = std::arch::x86_64::_mm_loadu_si128(MAX_BYTES.as_ptr() as *const __m128i);
        let space = std::arch::x86_64::_mm_loadu_si128(SPACE_SEP_BYTES.as_ptr() as *const __m128i);

        let gt = std::arch::x86_64::_mm_cmpgt_epi8(ts_without_seconds, min);
        let lt = std::arch::x86_64::_mm_cmplt_epi8(ts_without_seconds, max);

        let space_sep = std::arch::x86_64::_mm_cmpeq_epi8(ts_without_seconds, space);
        let mask = std::arch::x86_64::_mm_or_si128(std::arch::x86_64::_mm_and_si128(gt, lt), space_sep);
        let mask = std::arch::x86_64::_mm_movemask_epi8(mask);


        eprintln!("{:x}", mask);
        if mask != 0xFFFF {
            return Err(ParseError::InvalidChar((!mask).trailing_zeros() as usize));
        }
        /*
                let nums = std::arch::x86_64::_mm_and_si128(ts_without_seconds, std::arch::x86_64::_mm_loadu_si128(NUM_MASK.as_ptr() as *const __m128i));
                let numsperm = std::arch::x86_64::_mm256_shuffle_epi8(ts_without_seconds, std::arch::x86_64::_mm_loadu_si128(NUM_MASK.as_ptr() as *const __m128i));

                let nums = std::arch::x86_64::_mm256_broadcastsi128_si256(nums);
                let nums = std::arch::x86_64::_mm256_sra_epi16(nums;

        */
    }


    Ok(Packedtime(0))
}




#[cfg(test)]
pub mod tests {
    use crate::{Packedtime, parse, ParseError, format_simd_mul_to_slice, format_simd_dd_to_slice, format_scalar_to_slice};

    #[test]
    fn test_valid() {
        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01T00:00Z"));
        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01T00:00:00Z"));
        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01T00:00:00.000Z"));

        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01 00:00Z"));
        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01 00:00:00Z"));
        assert_eq!(Ok(Packedtime(0)), parse("1970-01-01 00:00:00.000Z"));
    }

    #[test]
    fn test_invalid_len() {
        assert_eq!(Err(ParseError::InvalidLen(0)), parse(""));
        assert_eq!(Err(ParseError::InvalidLen(1)), parse("X"));
        assert_eq!(Err(ParseError::InvalidLen(4)), parse("2020"));
    }

    #[test]
    fn test_invalid_char() {
        //assert_eq!(Err(ParseError::InvalidChar(0)), parse("XXXX/XX/XX&XX/XX/XX_"));
        //assert_eq!(Err(ParseError::InvalidChar(16)), parse("2020-09-10T12:XX:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(0)), parse("X020-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(1)), parse("2X20-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(2)), parse("20X0-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse("2020-09-10X12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse("2020-09-10X12:00/"));
        assert_eq!(Err(ParseError::InvalidChar(15)), parse("2020-09-10T12:0X/"));
    }

    #[test]
    fn test_packed() {
        let ts = Packedtime::new_utc(2020, 9, 10, 17, 30, 15, 123);
        assert_eq!(2020, ts.year());
        assert_eq!(9, ts.month());
        assert_eq!(10, ts.day());
        assert_eq!(17, ts.hour());
        assert_eq!(30, ts.minute());
        assert_eq!(15, ts.second());
        assert_eq!(123, ts.millisecond());
    }

    #[test]
    fn test_format() {
        assert_eq!("2020-12-24T17:30:15.010Z".to_owned(), Packedtime::new_utc(2020, 12, 24, 17, 30, 15, 10).format());
        assert_eq!("2020-09-10T17:30:15.123Z".to_owned(), Packedtime::new_utc(2020, 9, 10, 17, 30, 15, 123).format());
    }

    type FormatToSlice =  unsafe fn(&mut [u8], u32, u32, u32, u32, u32, u32, u32);

    fn assert_format(expected: &str, year: u32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millisecond: u32, f: FormatToSlice) {
        let actual = unsafe {
            let mut buffer: Vec<u8> = Vec::with_capacity(32);

            unsafe {buffer.set_len(24) };

            let slice = &mut buffer.as_mut_slice()[0..24];

            f(slice, year, month, day, hour, minute, second, millisecond);

            String::from_utf8(buffer).unwrap()
        };
        assert_eq!(expected, &actual);
    }

    #[test]
    fn test_format_scalar() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_scalar_to_slice);
    }

    #[test]
    fn test_format_simd_dd() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_simd_dd_to_slice);
    }

    #[test]
    fn test_format_simd_mul() {
        assert_format("2021-09-10T23:45:31.987Z", 2021, 09, 10, 23, 45, 31, 987, format_simd_mul_to_slice);
    }


}