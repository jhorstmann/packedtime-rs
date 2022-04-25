use crate::error::*;
use crate::convert::to_epoch_day;
use crate::PackedTimestamp;

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

const _: () = { assert!(std::mem::size_of::<SimdTimestamp>() == 16); };

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

#[derive(PartialEq, Clone, Debug, Default)]
struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u32,
    offset_minute: i32,
}

impl Timestamp {
    fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8, millisecond: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            offset_minute: 0,
        }
    }

    fn new_with_offset_seconds(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8, millisecond: u32, offset_second: i32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            offset_minute: offset_second,
        }
    }
}


#[inline(always)]
fn ts_to_epoch_millis(ts: &Timestamp) -> i64 {
    let epoch_day = to_epoch_day(ts.year as i32, ts.month as i32, ts.day as i32) as i64;

    let h = ts.hour as i64;
    let m = ts.minute as i64;
    let s = ts.second as i64;
    let offset_minute = ts.offset_minute as i64;
    let seconds = epoch_day * 24 * 60 * 60 + h * 60 * 60 + m * 60 + s as i64 - offset_minute * 60;

    return seconds * 1000 + ts.millisecond as i64;
}

pub fn parse_to_epoch_millis_scalar(input: &str) -> ParseResult<i64> {
    let ts = parse_scalar(input)?;
    Ok(ts_to_epoch_millis(&ts))
}

pub fn parse_to_packed_timestamp_scalar(input: &str) -> ParseResult<PackedTimestamp> {
    let ts = parse_scalar(input)?;
    Ok(PackedTimestamp::new(ts.year as i32, ts.month as u32, ts.day as u32, ts.hour as u32, ts.minute as u32, ts.second as u32, ts.millisecond as u32, ts.offset_minute))
}

fn parse_scalar(str: &str) -> ParseResult<Timestamp> {
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

    let offset = parse_utc_or_offset_minutes(bytes, &mut index)?;

    timestamp.year = year as u16;
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

#[inline(never)]
fn parse_seconds_and_nanos_and_offset_minutes_slow_path(bytes: &[u8], mut index: &mut usize) -> ParseResult<(u32, u32, i32)> {
    let (seconds, nanos) = parse_seconds_and_nanos(bytes, &mut index)?;
    let offset_minutes = parse_utc_or_offset_minutes(bytes, &mut index)?;
    Ok((seconds, nanos, offset_minutes))
}

#[inline(always)]
fn parse_utc_or_offset_minutes(bytes: &[u8], mut index: &mut usize) -> ParseResult<i32> {
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
        Ok(parse_offset_minutes(bytes, &mut index)? as i32)
    } else if first == b'-' {
        *index += 1;
        Ok(-(parse_offset_minutes(bytes, &mut index)? as i32))
    } else {
        Err(ParseError::InvalidChar(*index))
    }
}

#[inline(always)]
fn parse_offset_minutes(bytes: &[u8], mut index: &mut usize) -> ParseResult<u32> {
    let offset_hour = parse_num2(bytes, &mut index)?;
    expect(bytes, index, b':')?;
    let offset_minute = parse_num2(bytes, &mut index)?;

    Ok(offset_hour * 60 + offset_minute)
}

#[inline(always)]
fn parse_num2(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    Ok(d1 * 10 + d2)
}

#[inline(always)]
fn parse_num4(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    let d3 = digit(bytes, &mut i)?;
    let d4 = digit(bytes, &mut i)?;
    Ok(d1 * 1000 + d2 * 100 + d3 * 10 + d4)
}

const NANO_MULTIPLIER: [u32; 9] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, 100_000_000];

#[inline(always)]
fn parse_nano(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let mut r = digit(bytes, &mut i)?;
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

// only public for benchmarks
#[doc(hidden)]
#[inline]
pub fn parse_to_epoch_millis_simd(input: &str) -> ParseResult<i64> {
    let ts = parse_simd(input)?;
    Ok(ts_to_epoch_millis(&ts))
}

// only public for benchmarks
#[doc(hidden)]
#[inline]
pub fn parse_to_packed_timestamp_simd(input: &str) -> ParseResult<PackedTimestamp> {
    let ts = parse_simd(input)?;
    Ok(PackedTimestamp::new(ts.year as i32, ts.month as u32, ts.day as u32, ts.hour as u32, ts.minute as u32, ts.second as u32, ts.millisecond as u32, ts.offset_minute))
}

#[inline(always)]
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
    let nums = _mm_shuffle_epi8(nums, _mm_set_epi8(
        -1, -1, -1, -1, 15, 14, 12, 11, 9, 8, 6, 5, 3, 2, 1, 0,
    ));

    let hundreds = _mm_and_si128(nums, _mm_set1_epi16(0x00FF));
    let hundreds = _mm_mullo_epi16(hundreds, _mm_set1_epi16(10));

    let ones = _mm_srli_epi16(nums, 8);

    let res = _mm_add_epi16(ones, hundreds);

    let timestamp_ptr: *mut SimdTimestamp = &mut timestamp;
    _mm_storeu_si128(timestamp_ptr as *mut __m128i, res);

    Ok(timestamp)
}

#[inline(always)]
fn parse_simd(input: &str) -> ParseResult<Timestamp> {

    if input.len() < 16 {
        return Err(ParseError::InvalidLen(input.len()));
    }

    let bytes = input.as_bytes();

    let timestamp = unsafe { parse_simd_yyyy_mm_dd_hh_mm(bytes.as_ptr())? };

    let (second, milli, offset_minutes) = if let Some((second, nano, offset_sign)) = try_parse_seconds_and_millis_simd(bytes) {
        let offset = match offset_sign {
            b'Z' => 0,
            b'+' | b'-' => {
                let mut index = 24;
                let tmp = parse_offset_minutes(bytes, &mut index)? as i32;
                if offset_sign == b'-' {
                    -tmp
                } else {
                    tmp
                }
            }
            _ => return Err(ParseError::InvalidChar(23))
        };
        (second, nano, offset)
    } else {
        let mut index = 16;
        let (second, nano, offset_minutes) = parse_seconds_and_nanos_and_offset_minutes_slow_path(bytes, &mut index)?;
        (second, nano / 1_000_000, offset_minutes)
    };

    let mut result = Timestamp::default();
    result.year = timestamp.year_hi * 100 + timestamp.year_lo;
    result.month = timestamp.month as u8;
    result.day = timestamp.day as u8;
    result.hour = timestamp.hour as u8;
    result.minute = timestamp.minute as u8;
    result.second = second as u8;
    result.millisecond = milli;
    result.offset_minute = offset_minutes * 60;

    Ok(result)
}

#[inline(always)]
fn try_parse_seconds_and_millis_simd(input: &[u8]) -> Option<(u32, u32, u8)> {
    if input.len() >= 24 {
        let min: u64 = unsafe { std::ptr::read_unaligned(b":00.000+".as_ptr() as *const u64) };
        let max: u64 = unsafe { std::ptr::read_unaligned(b":99.999Z".as_ptr() as *const u64) };
        let buf = unsafe { std::ptr::read_unaligned(input.as_ptr().add(16) as *const u64) };

        if buf < min || buf > max {
            return None
        }

        let buf = unsafe { std::mem::transmute::<u64, [u8; 8]>(buf) };

        let second = (buf[1] - b'0') as u32 * 10 + (buf[2] - b'0') as u32;
        let milli = (buf[4] - b'0') as u32 * 100 + (buf[5] - b'0') as u32 * 10 + (buf[6] - b'0') as u32;

        Some((second, milli, buf[7]))
    } else {
        None
    }
}

#[cfg(test)]
pub mod tests {
    use crate::parse::{parse_simd, parse_scalar, Timestamp};
    use crate::error::ParseError;
    use crate::{parse_to_epoch_millis_scalar, parse_to_epoch_millis_simd};

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
    fn test_parse_simd() {
        assert_eq!(Timestamp::new(2345, 12, 24, 17, 30, 15, 123), parse_simd("2345-12-24T17:30:15.123Z").unwrap());
    }

    #[test]
    fn test_parse_with_offset_simd() {
        assert_eq!(Timestamp::new_with_offset_seconds(2020, 9, 19, 11, 40, 20, 123, 2 * 60 * 60),
                   parse_simd("2020-09-19T11:40:20.123+02:00").unwrap());
    }

    #[test]
    fn test_parse_millis_scalar() {
        let input = "2020-09-18T23:30:15Z";
        let expected = chrono::DateTime::parse_from_rfc3339(input).unwrap().timestamp_millis();
        let actual = parse_to_epoch_millis_scalar(input).unwrap();
        assert_eq!(expected, actual);
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
}