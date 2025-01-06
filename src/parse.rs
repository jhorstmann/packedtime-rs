use crate::datetime::DateTimeComponents;
use crate::error::*;
use crate::{EpochDays, PackedTimestamp};

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

const _: () = {
    assert!(std::mem::size_of::<SimdTimestamp>() == 16);
};

impl SimdTimestamp {
    fn new(year: u16, month: u16, day: u16, hour: u16, minute: u16) -> Self {
        Self {
            year_hi: year / 100,
            year_lo: year % 100,
            month,
            day,
            hour,
            minute,
            pad1: 0,
            pad2: 0,
        }
    }
}

#[inline(always)]
fn ts_to_epoch_millis(ts: &DateTimeComponents) -> i64 {
    let epoch_day = EpochDays::from_ymd(ts.year, ts.month as i32, ts.day as i32).days() as i64;

    let h = ts.hour as i64;
    let m = ts.minute as i64;
    let s = ts.second as i64;
    let offset_minute = ts.offset_minute as i64;
    let seconds = epoch_day * 24 * 60 * 60 + h * 60 * 60 + m * 60 + s - offset_minute * 60;

    seconds * 1000 + ts.millisecond as i64
}

#[doc(hidden)]
pub fn parse_to_epoch_millis_scalar(input: &str) -> ParseResult<i64> {
    let ts = parse_scalar(input.as_bytes())?;
    Ok(ts_to_epoch_millis(&ts))
}

#[doc(hidden)]
pub fn parse_to_packed_timestamp_scalar(input: &str) -> ParseResult<PackedTimestamp> {
    let ts = parse_scalar(input.as_bytes())?;
    Ok(PackedTimestamp::new(
        ts.year,
        ts.month as u32,
        ts.day as u32,
        ts.hour as u32,
        ts.minute as u32,
        ts.second as u32,
        ts.millisecond,
        ts.offset_minute,
    ))
}

pub(crate) fn parse_scalar(bytes: &[u8]) -> ParseResult<DateTimeComponents> {
    if bytes.len() < 16 {
        return Err(ParseError::InvalidLen(bytes.len()));
    }

    let mut timestamp = DateTimeComponents::default();
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

    let offset = parse_utc_or_offset_minutes(bytes, &mut index)?;

    timestamp.year = year as i32;
    timestamp.month = month as u8;
    timestamp.day = day as u8;
    timestamp.hour = hour as u8;
    timestamp.minute = minute as u8;
    timestamp.second = second as u8;
    timestamp.millisecond = nano / 1_000_000;
    timestamp.offset_minute = offset;

    Ok(timestamp)
}

#[inline(always)]
fn parse_seconds_and_nanos(bytes: &[u8], index: &mut usize) -> ParseResult<(u32, u32)> {
    let mut second = 0;
    let mut nano = 0;
    if *index < bytes.len() {
        let ch = bytes[*index];
        if ch == b'.' {
            *index += 1;
            nano = parse_nano(bytes, index)?;
        } else if ch == b':' {
            *index += 1;
            second = parse_num2(bytes, index)?;
            if *index < bytes.len() && bytes[*index] == b'.' {
                *index += 1;
                nano = parse_nano(bytes, index)?;
            }
        }
    }

    Ok((second, nano))
}

#[inline(never)]
fn parse_seconds_and_nanos_and_offset_minutes_slow_path(bytes: &[u8], index: &mut usize) -> ParseResult<(u32, u32, i32)> {
    let (seconds, nanos) = parse_seconds_and_nanos(bytes, index)?;
    let offset_minutes = parse_utc_or_offset_minutes(bytes, index)?;
    Ok((seconds, nanos, offset_minutes))
}

#[inline(never)]
fn skip_nanos_and_parse_offset_minutes_slow_path(bytes: &[u8], index: &mut usize) -> ParseResult<i32> {
    skip_fractional_millis(bytes, index);
    let offset_minutes = parse_utc_or_offset_minutes(bytes, index)?;
    Ok(offset_minutes)
}

#[inline(always)]
fn parse_utc_or_offset_minutes(bytes: &[u8], index: &mut usize) -> ParseResult<i32> {
    if *index >= bytes.len() {
        return Err(ParseError::InvalidLen(*index));
    }
    let first = bytes[*index];
    if first == b'Z' {
        *index += 1;
        if *index != bytes.len() {
            Err(ParseError::TrailingChar(*index))
        } else {
            Ok(0)
        }
    } else if first == b'+' {
        *index += 1;
        Ok(parse_offset_minutes(bytes, index)? as i32)
    } else if first == b'-' {
        *index += 1;
        Ok(-(parse_offset_minutes(bytes, index)? as i32))
    } else {
        Err(ParseError::InvalidChar(*index))
    }
}

#[inline(always)]
fn parse_offset_minutes(bytes: &[u8], index: &mut usize) -> ParseResult<u32> {
    let offset_hour = parse_num2(bytes, index)?;
    expect(bytes, index, b':')?;
    let offset_minute = parse_num2(bytes, index)?;

    Ok(offset_hour * 60 + offset_minute)
}

#[inline(always)]
fn parse_num2(bytes: &[u8], i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, i)?;
    let d2 = digit(bytes, i)?;
    Ok(d1 * 10 + d2)
}

#[inline(always)]
fn parse_num4(bytes: &[u8], i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, i)?;
    let d2 = digit(bytes, i)?;
    let d3 = digit(bytes, i)?;
    let d4 = digit(bytes, i)?;
    Ok(d1 * 1000 + d2 * 100 + d3 * 10 + d4)
}

const NANO_MULTIPLIER: [u32; 9] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000];

#[inline(always)]
fn parse_nano(bytes: &[u8], i: &mut usize) -> ParseResult<u32> {
    let mut r = digit(bytes, i)?;
    let mut j = 1;

    while *i < bytes.len() && j < 9 {
        let ch = bytes[*i];
        if ch >= b'0' && ch <= b'9' {
            r = r * 10 + (ch - b'0') as u32;
            j += 1;
            *i += 1;
        } else {
            break;
        }
    }

    Ok(r * NANO_MULTIPLIER[9 - j])
}

#[inline(always)]
fn skip_fractional_millis(bytes: &[u8], i: &mut usize) {
    let mut j = 0;

    while *i < bytes.len() && j < 6 {
        let ch = bytes[*i];
        if ch >= b'0' && ch <= b'9' {
            j += 1;
            *i += 1;
        } else {
            break;
        }
    }
}

#[inline(always)]
fn expect(bytes: &[u8], i: &mut usize, expected: u8) -> ParseResult<()> {
    if *i >= bytes.len() {
        return Err(ParseError::InvalidLen(*i));
    }
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
    if *i >= bytes.len() {
        return Err(ParseError::InvalidLen(*i));
    }
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
    if *i >= bytes.len() {
        return Err(ParseError::InvalidLen(*i));
    }
    let ch = bytes[*i];
    if ch >= b'0' && ch <= b'9' {
        *i += 1;
        Ok((ch - b'0') as u32)
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}

// only public for benchmarks
#[doc(hidden)]
#[inline]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
pub fn parse_to_epoch_millis_simd(input: &str) -> ParseResult<i64> {
    let ts = parse_simd(input.as_bytes())?;
    Ok(ts_to_epoch_millis(&ts))
}

// only public for benchmarks
#[doc(hidden)]
#[inline]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
pub fn parse_to_packed_timestamp_simd(input: &str) -> ParseResult<PackedTimestamp> {
    let ts = parse_simd(input.as_bytes())?;
    Ok(PackedTimestamp::new(
        ts.year,
        ts.month as u32,
        ts.day as u32,
        ts.hour as u32,
        ts.minute as u32,
        ts.second as u32,
        ts.millisecond,
        ts.offset_minute,
    ))
}

#[inline]
#[cfg(target_arch = "x86_64")]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
unsafe fn parse_simd_yyyy_mm_dd_hh_mm(bytes: *const u8) -> ParseResult<SimdTimestamp> {
    use std::arch::x86_64::*;

    const MIN_BYTES: &[u8] = "))))-)0-)0S))9))9))".as_bytes();
    const MAX_BYTES: &[u8] = "@@@@-2@-4@U3@;6@;6@".as_bytes();
    const SPACE_SEP_BYTES: &[u8] = "0000-00-00 00:00:00".as_bytes();
    const REM_MIN_BYTES: &[u8] = "9-)Y*9))))))))))".as_bytes();
    const REM_MAX_BYTES: &[u8] = ";/@[.;@@@@@@@@@@".as_bytes();

    let mut timestamp = SimdTimestamp::default();
    let ts_without_seconds = _mm_loadu_si128(bytes as *const __m128i);
    let min = _mm_loadu_si128(MIN_BYTES.as_ptr() as *const __m128i);
    let max = _mm_loadu_si128(MAX_BYTES.as_ptr() as *const __m128i);
    let space = _mm_loadu_si128(SPACE_SEP_BYTES.as_ptr() as *const __m128i);

    let gt = _mm_cmpgt_epi8(ts_without_seconds, min);
    let lt = _mm_cmplt_epi8(ts_without_seconds, max);

    let space_sep = _mm_cmpeq_epi8(ts_without_seconds, space);
    let mask = _mm_or_si128(_mm_and_si128(gt, lt), space_sep);
    let mask = _mm_movemask_epi8(mask);

    if mask != 0xFFFF {
        return Err(ParseError::InvalidChar((!mask).trailing_zeros() as usize));
    }

    let nums = _mm_sub_epi8(ts_without_seconds, space);
    let nums = _mm_shuffle_epi8(nums, _mm_set_epi8(-1, -1, -1, -1, 15, 14, 12, 11, 9, 8, 6, 5, 3, 2, 1, 0));

    let hundreds = _mm_and_si128(nums, _mm_set1_epi16(0x00FF));
    let hundreds = _mm_mullo_epi16(hundreds, _mm_set1_epi16(10));

    let ones = _mm_srli_epi16::<8>(nums);

    let res = _mm_add_epi16(ones, hundreds);

    let timestamp_ptr: *mut SimdTimestamp = &mut timestamp;
    _mm_storeu_si128(timestamp_ptr as *mut __m128i, res);

    Ok(timestamp)
}

#[inline]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
pub(crate) fn parse_simd(bytes: &[u8]) -> ParseResult<DateTimeComponents> {
    if bytes.len() < 16 {
        return Err(ParseError::InvalidLen(bytes.len()));
    }

    let timestamp = unsafe { parse_simd_yyyy_mm_dd_hh_mm(bytes.as_ptr())? };

    let (seconds, millis, offset_minutes) = parse_seconds_and_millis_simd(bytes)?;

    Ok(DateTimeComponents {
        year: timestamp.year_hi as i32 * 100 + timestamp.year_lo as i32,
        month: timestamp.month as u8,
        day: timestamp.day as u8,
        hour: timestamp.hour as u8,
        minute: timestamp.minute as u8,
        second: seconds as u8,
        millisecond: millis,
        offset_minute: offset_minutes,
    })
}

#[inline(always)]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
fn parse_seconds_and_millis_simd(bytes: &[u8]) -> ParseResult<(u32, u32, i32)> {
    if let Some((seconds, millis, offset_sign)) = try_parse_seconds_and_millis_simd(bytes) {
        match offset_sign {
            b'Z' => return Ok((seconds, millis, 0)),
            b'+' | b'-' => {
                let mut index = 24;
                let offset_minutes = parse_offset_minutes(bytes, &mut index)? as i32;
                let offset_minutes = if offset_sign == b'-' {
                    -offset_minutes
                } else {
                    offset_minutes
                };
                return Ok((seconds, millis, offset_minutes));
            }
            digit @ b'0'..=b'9' => {
                let mut i = 24 - 1;
                let offset_minutes = skip_nanos_and_parse_offset_minutes_slow_path(bytes, &mut i)?;
                return Ok((seconds, millis, offset_minutes));
            }
            _ => return Err(ParseError::InvalidChar(23)),
        }
    }

    let mut index = 16;
    let (second, nano, offset_minutes) = parse_seconds_and_nanos_and_offset_minutes_slow_path(bytes, &mut index)?;
    Ok((second, nano / 1_000_000, offset_minutes))
}

#[inline(always)]
#[cfg(all(target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
fn try_parse_seconds_and_millis_simd(input: &[u8]) -> Option<(u32, u32, u8)> {
    use std::arch::x86_64::*;
    if input.len() >= 24 {
        let buf = unsafe { std::ptr::read_unaligned(input.as_ptr().add(16) as *const u64) };

        unsafe {
            let min = _mm_sub_epi8(
                _mm_set_epi64x(0, i64::from_le_bytes(*b":00.000+")),
                _mm_set1_epi64x(0x0101_0101_0101_0101),
            );
            let max = _mm_add_epi8(
                _mm_set_epi64x(0, i64::from_le_bytes(*b":99.999Z")),
                _mm_set1_epi64x(0x0101_0101_0101_0101),
            );
            let reg = _mm_set1_epi64x(buf as _);

            let gt = _mm_cmpgt_epi8(reg, min);
            let lt = _mm_cmplt_epi8(reg, max);

            let mask = _mm_movemask_epi8(_mm_and_si128(gt, lt));

            if mask != 0xFF {
                return None;
            }
        }

        let buf = buf.to_le_bytes();

        let second = (buf[1] - b'0') as u32 * 10 + (buf[2] - b'0') as u32;
        let milli = (buf[4] - b'0') as u32 * 100 + (buf[5] - b'0') as u32 * 10 + (buf[6] - b'0') as u32;

        Some((second, milli, buf[7]))
    } else {
        None
    }
}

pub fn parse_to_timestamp_millis(bytes: &[u8]) -> ParseResult<i64> {
    #[cfg(target_feature = "sse4.1")]
    {
        let ts = parse_simd(bytes)?;
        Ok(ts_to_epoch_millis(&ts))
    }
    #[cfg(not(target_feature = "sse4.1"))]
    {
        let ts = parse_scalar(bytes)?;
        Ok(ts_to_epoch_millis(&ts))
    }
}

#[cfg(test)]
#[cfg(all(not(miri), target_arch = "x86_64", target_feature = "sse2", target_feature = "ssse3"))]
pub mod simd_tests {
    use crate::error::ParseError;
    use crate::parse::{parse_simd, try_parse_seconds_and_millis_simd, DateTimeComponents};
    use crate::parse_to_epoch_millis_simd;

    #[test]
    fn test_valid() {
        assert!(parse_simd(b"1970-01-01T00:00Z").is_ok());
        assert!(parse_simd(b"1970-01-01T00:00:00Z").is_ok());
        assert!(parse_simd(b"1970-01-01T00:00:00.000Z").is_ok());

        assert!(parse_simd(b"1970-01-01 00:00Z").is_ok());
        assert!(parse_simd(b"1970-01-01 00:00:00Z").is_ok());
        assert!(parse_simd(b"1970-01-01 00:00:00.000Z").is_ok());
    }

    #[test]
    fn test_invalid_len() {
        assert_eq!(Err(ParseError::InvalidLen(0)), parse_simd(b""));
        assert_eq!(Err(ParseError::InvalidLen(1)), parse_simd(b"X"));
        assert_eq!(Err(ParseError::InvalidLen(4)), parse_simd(b"2020"));
    }

    #[test]
    fn test_invalid_char() {
        assert_eq!(Err(ParseError::InvalidChar(0)), parse_simd(b"X020-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(1)), parse_simd(b"2X20-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(2)), parse_simd(b"20X0-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse_simd(b"2020-09-10X12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse_simd(b"2020-09-10X12:00/"));
        assert_eq!(Err(ParseError::InvalidChar(15)), parse_simd(b"2020-09-10T12:0X/"));
    }

    #[test]
    fn test_parse_simd() {
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 100),
            parse_simd(b"2345-12-24T17:30:15.1Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 120),
            parse_simd(b"2345-12-24T17:30:15.12Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.123Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.1234Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.12345Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.123456Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.123457Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.12345678Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_simd(b"2345-12-24T17:30:15.123456789Z").unwrap()
        );
        assert_eq!(
            DateTimeComponents::new_with_offset_minute(2345, 12, 24, 17, 30, 15, 123, -60),
            parse_simd(b"2345-12-24T17:30:15.123456789-01:00").unwrap()
        );
    }

    #[test]
    fn test_parse_with_offset_simd() {
        assert_eq!(
            DateTimeComponents::new_with_offset_minute(2020, 9, 19, 11, 40, 20, 123, 2 * 60),
            parse_simd(b"2020-09-19T11:40:20.123+02:00").unwrap()
        );
    }

    #[test]
    fn test_parse_with_zero_offset_simd() {
        assert_eq!(
            DateTimeComponents::new_with_offset_minute(2020, 9, 19, 11, 40, 20, 123, 0),
            parse_simd(b"2020-09-19T11:40:20.123-00:00").unwrap()
        );
    }

    #[test]
    fn test_parse_with_negative_offset_simd() {
        assert_eq!(
            DateTimeComponents::new_with_offset_minute(2020, 9, 19, 11, 40, 20, 123, -2 * 60),
            parse_simd(b"2020-09-19T11:40:20.123-02:00").unwrap()
        );
    }

    #[test]
    fn test_parse_millis_simd() {
        let input = "2020-09-18T23:30:15Z";
        let expected = chrono::DateTime::parse_from_rfc3339(input).unwrap().timestamp_millis();
        let actual = parse_to_epoch_millis_simd(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_parse_millis_simd_masked() {
        let input = "2020-09-18T23:30:15Z--::ZZ";
        let input = unsafe { input.get_unchecked(0..20) };
        let expected = chrono::DateTime::parse_from_rfc3339(input).unwrap().timestamp_millis();
        let actual = parse_to_epoch_millis_simd(input).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_try_parse_seconds_and_millis_simd() {
        let input = b"2020-09-08T13:42:29+00:00";
        // fast path should require milliseconds
        assert!(try_parse_seconds_and_millis_simd(input).is_none());

        let input = b"2020-09-08T13:42:29.123Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'Z')));

        let input = b"2020-09-08T13:42:29.123+01:00";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'+')));

        let input = b"2020-09-08T13:42:29.123-01:00";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'-')));

        let input = b"2020-09-08T13:42:29.123456Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));

        let input = b"2020-09-08T13:42:29.123456+01:00";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));

        let input = b"2020-09-08T13:42:29.1234567Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));
        let input = b"2020-09-08T13:42:29.1234567-01:00";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));

        let input = b"2020-09-08T13:42:29.12345678Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));
        let input = b"2020-09-08T13:42:29.123456789Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));
        let input = b"2020-09-08T13:42:29.123456789Z";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));
        let input = b"2020-09-08T13:42:29.123456789-02:00";
        assert_eq!(try_parse_seconds_and_millis_simd(input), Some((29, 123, b'4')));
    }

    #[test]
    fn test_parse_leap_seconds_simd() {
        assert_eq!(
            DateTimeComponents::new(2023, 1, 3, 9, 30, 60, 123),
            parse_simd(b"2023-01-03T09:30:60.123Z").unwrap()
        );
    }
}

#[cfg(test)]
mod scalar_tests {
    use crate::datetime::DateTimeComponents;
    use crate::{parse_scalar, parse_to_epoch_millis_scalar};

    #[test]
    fn test_parse_scalar() {
        assert_eq!(
            DateTimeComponents::new(2345, 12, 24, 17, 30, 15, 123),
            parse_scalar(b"2345-12-24T17:30:15.123Z").unwrap()
        );
    }

    #[test]
    fn test_parse_leap_seconds_scalar() {
        assert_eq!(
            DateTimeComponents::new(2023, 1, 3, 9, 30, 60, 123),
            parse_scalar(b"2023-01-03T09:30:60.123Z").unwrap()
        );
    }

    #[test]
    fn test_parse_millis_scalar() {
        let input = "2020-09-18T23:30:15Z";
        let expected = chrono::DateTime::parse_from_rfc3339(input).unwrap().timestamp_millis();
        let actual = parse_to_epoch_millis_scalar(input).unwrap();
        assert_eq!(expected, actual);
    }
}
