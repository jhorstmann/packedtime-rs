
//   MMMMdddddhhhhhmmmmmmssssss
// MMMMdddddhhhhhmmmmmmssssss00
// 3210765432107654321076543210

use crate::format::*;
use std::fmt::{Display, Debug, Formatter};
use std::str::from_utf8_unchecked;

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

const _: () = {
    assert!(MIN_YEAR_INTERNAL < MIN_YEAR || MAX_YEAR_INTERNAL > MAX_YEAR);
    assert!(MIN_OFFSET_MINUTES_INTERNAL < MIN_OFFSET_MINUTES || MAX_OFFSET_MINUTES_INTERNAL > MAX_OFFSET_MINUTES);
};


#[derive(PartialEq, Clone, Copy)]
pub struct Packedtime(u64);

impl Packedtime {
    #[inline]
    pub fn new_utc(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        milli: u32,
    ) -> Self {
        Self::new(year, month, day, hour, minute, second, milli, 0)
    }
    pub fn new(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        milli: u32,
        offset: u32,
    ) -> Self {
        let value = ((((((((year as u64) << MONTH_BITS | month as u64) << DAY_BITS | day as u64)
            << HOUR_BITS
            | hour as u64)
            << MINUTE_BITS
            | minute as u64)
            << SECOND_BITS
            | second as u64)
            << MILLI_BITS
            | milli as u64)
            << OFFSET_BITS)
            | offset as u64;
        Self(value)
    }

    #[inline]
    pub fn year(&self) -> u32 {
        (self.0 >> (MONTH_BITS + DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) as u32
    }

    #[inline]
    pub fn month(&self) -> u32 {
        ((self.0 >> (DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MONTH_BITS) - 1)) as u32
    }

    #[inline]
    pub fn day(&self) -> u32 {
        ((self.0 >> (HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << DAY_BITS) - 1)) as u32
    }

    #[inline]
    pub fn hour(&self) -> u32 {
        ((self.0 >> (MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << HOUR_BITS) - 1)) as u32
    }

    #[inline]
    pub fn minute(&self) -> u32 {
        ((self.0 >> (SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MINUTE_BITS) - 1)) as u32
    }

    #[inline]
    pub fn second(&self) -> u32 {
        ((self.0 >> (MILLI_BITS + OFFSET_BITS)) & ((1 << SECOND_BITS) - 1)) as u32
    }

    #[inline]
    pub fn millisecond(&self) -> u32 {
        ((self.0 >> (OFFSET_BITS)) & ((1 << MILLI_BITS) - 1)) as u32
    }

    #[inline]
    pub fn write_rfc3339_str(&self, buffer: &mut [u8]) {
        #[cfg(target_feature = "sse4.1")]
            unsafe {
            format_simd_mul_to_slice(buffer, self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond());
        }
        #[cfg(not(target_feature = "sse4.1"))]
            {
                format_scalar_to_slice(buffer, self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond());
            }
    }

    pub fn to_rfc3339_string(&self) -> String {
        let mut buffer = vec![0_u8; 24];

        self.write_rfc3339_str(&mut buffer);

        #[cfg(not(debug_assertions))]
            return unsafe { String::from_utf8_unchecked(buffer) };
        #[cfg(debug_assertions)]
            return String::from_utf8(buffer).expect("utf8 string");
    }
}

impl Display for Packedtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0_u8; 24];
        #[cfg(target_feature = "sse4.1")]
            unsafe {
                format_simd_mul_to_slice(&mut buffer, self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond());
            }
        #[cfg(not(target_feature = "sse4.1"))]
            {
                format_scalar_to_slice(&mut buffer, self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond());
            }
        f.write_str(unsafe { from_utf8_unchecked(&buffer) })
    }
}

impl Debug for Packedtime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", self.year(), self.month(), self.day(), self.hour(), self.minute(), self.second(), self.millisecond()))
    }
}

pub mod tests {
    use super::Packedtime;

    #[test]
    fn test_format() {
        assert_eq!("2020-12-24T17:30:15.010Z".to_owned(), Packedtime::new_utc(2020, 12, 24, 17, 30, 15, 10).to_rfc3339_string());
        assert_eq!("2020-09-10T17:30:15.123Z".to_owned(), Packedtime::new_utc(2020, 9, 10, 17, 30, 15, 123).to_rfc3339_string());
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
}