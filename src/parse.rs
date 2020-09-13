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
struct Timestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u32,
    offset_second: u32
}



fn parse_scalar(bytes: &[u8], timestamp: &mut Timestamp) -> ParseResult<()> {
    let mut index = 0;

    let year = parse_num4(bytes, &mut index)?;
    expect(bytes, &mut index, '-' as u8)?;
    let month = parse_num2(bytes, &mut index)?;
    expect(bytes, &mut index, '-' as u8)?;
    let day = parse_num2(bytes, &mut index)?;
    expect2(bytes, &mut index, 'T' as u8, ' ' as u8)?;
    let hour = parse_num2(bytes, &mut index)?;
    expect(bytes, &mut index, ':' as u8)?;
    let minute = parse_num2(bytes, &mut index)?;
    expect(bytes, &mut index, ':' as u8)?;

    let mut second = 0;
    let mut nano = 0;
    if index < bytes.len() {
        let ch = bytes[index];
        if ch == '.' as u8 {
            index += 1;
            nano = parse_nano(bytes, &mut index)?;

        } else if ch == ':' as u8 {
            index += 1;
            second = parse_num2(bytes, &mut index)?;
            if index < bytes.len() && bytes[index] == '.' as u8 {
                index += 1;
                nano = parse_nano(bytes, &mut index)?;
            }
        }
    }

    timestamp.year = year as u16;
    timestamp.month = month as u8;
    timestamp.day = day as u8;
    timestamp.hour = hour as u8;
    timestamp.minute = minute as u8;
    timestamp.second = second as u8;
    timestamp.millisecond = nano / 1000_000;

    Ok(())
}

fn parse_num2(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    Ok(d1*10 + d2)
}

fn parse_num4(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let d1 = digit(bytes, &mut i)?;
    let d2 = digit(bytes, &mut i)?;
    let d3 = digit(bytes, &mut i)?;
    let d4 = digit(bytes, &mut i)?;
    Ok(d1*1000 + d2 * 100 + d3*10 + d4)
}

fn parse_nano(bytes: &[u8], mut i: &mut usize) -> ParseResult<u32> {
    let mut r = digit(bytes, &mut i)?;
    let mut j = 1;

    while *i < bytes.len() && j < 9 {
        let ch = bytes[*i];
        if ch >= '0' as u8 && ch <= '9' as u8 {
            r = r*10 + (ch - ('0' as u8)) as u32;
            j += 1;
            *i += 1;
        } else {
            break;
        }
    }

    Ok(r)
}

fn expect(bytes: &[u8], i: &mut usize, expected: u8) -> ParseResult<()> {
    let ch = bytes[*i];
    if ch == expected {
        *i += 1;
        Ok(())
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}

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
    if ch >= '0' as u8 && ch <= '9' as u8 {
        *i += 1;
        Ok((ch - ('0' as u8)) as u32)
    } else {
        Err(ParseError::InvalidChar(*i))
    }
}



fn parse_seconds_and_millis(bytes: &[u8], index: &mut usize) -> ParseResult<(u32, u32)> {
    unimplemented!()
}



fn parse(input: &str) -> ParseResult<SimdTimestamp> {
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


    Ok(timestamp)
}

#[cfg(test)]
pub mod tests {
    use crate::parse::{parse, SimdTimestamp};
    use crate::error::ParseError;

    #[test]
    fn test_valid() {
        assert!(parse("1970-01-01T00:00Z").is_ok());
        assert!(parse("1970-01-01T00:00:00Z").is_ok());
        assert!(parse("1970-01-01T00:00:00.000Z").is_ok());

        assert!(parse("1970-01-01 00:00Z").is_ok());
        assert!(parse("1970-01-01 00:00:00Z").is_ok());
        assert!(parse("1970-01-01 00:00:00.000Z").is_ok());
    }

    #[test]
    fn test_invalid_len() {
        assert_eq!(Err(ParseError::InvalidLen(0)), parse(""));
        assert_eq!(Err(ParseError::InvalidLen(1)), parse("X"));
        assert_eq!(Err(ParseError::InvalidLen(4)), parse("2020"));
    }

    #[test]
    fn test_invalid_char() {
        assert_eq!(Err(ParseError::InvalidChar(0)), parse("X020-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(1)), parse("2X20-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(2)), parse("20X0-09-10T12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse("2020-09-10X12:00:00Z"));
        assert_eq!(Err(ParseError::InvalidChar(10)), parse("2020-09-10X12:00/"));
        assert_eq!(Err(ParseError::InvalidChar(15)), parse("2020-09-10T12:0X/"));
    }

    #[test]
    fn test_parse() {
        assert_eq!(SimdTimestamp::new(2345, 12, 24, 17, 30), parse("2345-12-24T17:30:15.010Z").unwrap());
    }
}