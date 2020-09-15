use std::arch::x86_64::{__m128i, __m256i};

use crate::error::*;

#[repr(C)]
#[derive(PartialEq, Clone, Debug, Default)]
struct SimdTimestamp {
    year_hi: u16,
    year_lo: u16,
    month: u16,
    day: u16,
    hour: u16,
    minute: u16,
    pad1: u16,
    pad2: u16,
}

static_assertions::const_assert!(std::mem::size_of::<SimdTimestamp>() == 16);

impl SimdTimestamp {
    fn new(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> Self {
        Self {
            year_hi: year / 100,
            year_lo: year % 100,
            month, day, hour, minute,
            pad1: 0,
            pad2: 0,
        }
    }
}

#[derive(PartialEq, Clone, Debug, Default)]
pub struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u32,
    offset_second: u32
}

impl Timestamp {
    fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8, millisecond: u32) -> Self {
        Self {
            year, month, day, hour, minute, second, millisecond, offset_second: 0,
        }
    }
}

pub fn parse_scalar(str: &str) -> ParseResult<Timestamp> {
    let bytes = str.as_bytes();
    let mut timestamp = Timestamp::default();
    let mut index = 0;

    let year = parse_num4(bytes, &mut index)?;
    expect(bytes, &mut index, b'-')?;
    let month = parse_num2(bytes, &mut index)?;
    expect(bytes, &mut index, b'-')?;
    let day = parse_num2(bytes, &mut index)?;
    expect2(bytes, &mut index, b'T', b' ')?;
    let hour = parse_num2(bytes, &mut index)?;
    expect(bytes, &mut index, b':')?;
    let minute = parse_num2(bytes, &mut index)?;

    let (second, nano) = parse_seconds_and_nanos(bytes, &mut index)?;

    expect(bytes, &mut index, b'Z')?;

    timestamp.year = year as u16;
    timestamp.month = month as u8;
    timestamp.day = day as u8;
    timestamp.hour = hour as u8;
    timestamp.minute = minute as u8;
    timestamp.second = second as u8;
    timestamp.millisecond = nano / 1000_000;
    timestamp.offset_second = 0;

    Ok(timestamp)
}

#[inline(always)]
fn parse_seconds_and_nanos(bytes: &[u8], mut index: &mut usize) -> ParseResult<(u32, u32)> {
    let mut second = 0;
    let mut nano = 0;
    if *index < bytes.len() {
        let ch = bytes[*index];
        if ch == b'.' {
            *index += 1;
            nano = parse_nano(bytes, &mut index)?;

        } else if ch == b':' {
            *index += 1;
            second = parse_num2(bytes, &mut index)?;
            if *index < bytes.len() && bytes[*index] == b'.' {
                *index += 1;
                nano = parse_nano(bytes, &mut index)?;
            }
        }
    }

    Ok((second, nano))
}

#[inline(always)]
fn parse_num2(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    Ok(d1*10 + d2)
}

#[inline(always)]
fn parse_num4(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    let d3 = digit(bytes, &mut i)?;
    let d4 = digit(bytes, &mut i)?;
    Ok(d1*1000 + d2 * 100 + d3*10 + d4)
}

const NANO_MULTIPLIER : [u32; 9] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000];

#[inline(always)]
fn parse_nano(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let mut r = digit(bytes, &mut i)?;
    let mut j = 1;

    while *i < bytes.len() && j < 9 {
        let ch = bytes[*i];
        if ch >= b'0' && ch <= b'9' {
            r = r*10 + (ch - b'0') as u32;
            j += 1;
            *i += 1;
        } else {
            break;
        }
    }

    Ok(r * NANO_MULTIPLIER[9-j])
}

#[inline(always)]
fn expect(bytes: &[u8], i: &mut usize, expected: u8) -> ParseResult<()> {
    let ch = bytes[*i];
    if ch == expected {
        *i += 1;
        Ok(())
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}

#[inline(always)]
fn expect2(bytes: &[u8], i: &mut usize, expected1: u8, expected2: u8) -> ParseResult<u8> {
    let ch = bytes[*i];
    if ch == expected1 || ch == expected2 {
        *i += 1;
        Ok(ch)
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}


#[inline(always)]
fn digit(bytes: &[u8], i: &mut usize) -> ParseResult<u32> {
    let ch = bytes[*i];
    if ch >= b'0' && ch <= b'9' {
        *i += 1;
        Ok((ch - b'0') as u32)
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}


pub fn parse_simd(input: &str) -> ParseResult<Timestamp> {
    //2020-09-09T15:05:45Z"
    //2020-09-09T15:05:45.123456789Z

    if input.len() < 16 {
        return Err(ParseError::InvalidLen(input.len()));
    }

    let bytes = input.as_bytes();
    const MIN_BYTES: &[u8] = "))))-)0-)0S))9))9))".as_bytes();
    const MAX_BYTES: &[u8] = "@@@@-2@-4@U3@;6@;6@".as_bytes();
    const SPACE_SEP_BYTES: &[u8] = "0000-00-00 00:00:00".as_bytes();

    let mut timestamp = SimdTimestamp::default();

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

        if mask != 0xFFFF {
            return Err(ParseError::InvalidChar((!mask).trailing_zeros() as usize));
        }

        let nums = std::arch::x86_64::_mm_sub_epi8(ts_without_seconds, space);
        let nums = std::arch::x86_64::_mm_shuffle_epi8(nums, std::arch::x86_64::_mm_set_epi8(
            -1,-1,-1,-1, 15, 14,12,11,9,8,6,5,3,2,1,0
        ));

        let hundreds = std::arch::x86_64::_mm_and_si128(nums,  std::arch::x86_64::_mm_set1_epi16(0x00FF));
        let hundreds = std::arch::x86_64::_mm_mullo_epi16(hundreds, std::arch::x86_64::_mm_set1_epi16(10));

        let ones = std::arch::x86_64::_mm_srli_epi16(nums,  8);

        let res = std::arch::x86_64::_mm_add_epi16(ones, hundreds);

        let timestamp_ptr: *mut SimdTimestamp = &mut timestamp;
        std::arch::x86_64::_mm_storeu_si128(timestamp_ptr as *mut __m128i, res);
    }

    let mut index = 16;
    let (second, nano) = parse_seconds_and_nanos(bytes, &mut index)?;

    expect(bytes, &mut index, b'Z');

    let mut result = Timestamp::default();
    result.year = timestamp.year_hi * 100 + timestamp.year_lo;
    result.month = timestamp.month as u8;
    result.day = timestamp.day as u8;
    result.hour = timestamp.hour as u8;
    result.minute = timestamp.minute as u8;
    result.second = second as u8;
    result.millisecond = nano / 1000_000;
    result.offset_second = 0;

    Ok(result)
}

#[cfg(test)]
pub mod tests {
    use crate::parse::{parse_simd, parse_scalar, SimdTimestamp, Timestamp};
    use crate::error::ParseError;

    #[test]
    fn test_valid() {
        assert!(parse_simd("1970-01-01T00:00Z").is_ok());
        assert!(parse_simd("1970-01-01T00:00:00Z").is_ok());
        assert!(parse_simd("1970-01-01T00:00:00.000Z").is_ok());

        assert!(parse_simd("1970-01-01 00:00Z").is_ok());
        assert!(parse_simd("1970-01-01 00:00:00Z").is_ok());
        assert!(parse_simd("1970-01-01 00:00:00.000Z").is_ok());
    }

    #[test]
    fn test_invalid_len() {
        assert_eq!(Err(ParseError::InvalidLen(0)), parse_simd(""));
        assert_eq!(Err(ParseError::InvalidLen(1)), parse_simd("X"));
        assert_eq!(Err(ParseError::InvalidLen(4)), parse_simd("2020"));
    }

    #[test]
    fn test_invalid_char() {
        assert_eq!(Err(ParseError::InvalidChar(0)), parse_simd("X020-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(1)), parse_simd("2X20-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(2)), parse_simd("20X0-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse_simd("2020-09-10X12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse_simd("2020-09-10X12:00/"));
        assert_eq!(Err(ParseError::InvalidChar(15)), parse_simd("2020-09-10T12:0X/"));
    }

    #[test]
    fn test_parse_scalar() {
        assert_eq!(Timestamp::new(2345, 12, 24, 17, 30, 15, 123), parse_scalar("2345-12-24T17:30:15.123Z").unwrap());
    }

    #[test]
    fn test_parse() {
        assert_eq!(Timestamp::new(2345, 12, 24, 17, 30, 15, 123), parse_simd("2345-12-24T17:30:15.123Z").unwrap());
    }
}