//   MMMMdddddhhhhhmmmmmmssssss
// MMMMdddddhhhhhmmmmmmssssss00
// 3210765432107654321076543210

use crate::datetime::DateTimeComponents;
use crate::format::*;
use crate::{EpochDays, ParseError, ParseResult};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

const OFFSET_BITS: u32 = 12;
const MILLI_BITS: u32 = 10;
const SECOND_BITS: u32 = 6;
const MINUTE_BITS: u32 = 6;
const HOUR_BITS: u32 = 5;
const DAY_BITS: u32 = 5;
const MONTH_BITS: u32 = 4;
const YEAR_BITS: u32 = 64 - (MONTH_BITS + DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS);

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

#[allow(clippy::assertions_on_constants)]
const _: () = {
    assert!(MIN_YEAR_INTERNAL < MIN_YEAR || MAX_YEAR_INTERNAL > MAX_YEAR);
    assert!(MIN_OFFSET_MINUTES_INTERNAL < MIN_OFFSET_MINUTES || MAX_OFFSET_MINUTES_INTERNAL > MAX_OFFSET_MINUTES);
};

#[derive(PartialEq, Clone, Copy, Ord, PartialOrd, Eq)]
#[repr(transparent)]
pub struct PackedTimestamp {
    value: u64,
}

impl PackedTimestamp {
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn new_utc(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, milli: u32) -> Self {
        Self::new(year, month, day, hour, minute, second, milli, 0)
    }

    #[inline]
    pub fn new_ymd_utc(year: i32, month: u32, day: u32) -> Self {
        Self::new(year, month, day, 0, 0, 0, 0, 0)
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, milli: u32, offset_minutes: i32) -> Self {
        let value = ((((((((year as u64) << MONTH_BITS | month as u64) << DAY_BITS | day as u64) << HOUR_BITS | hour as u64)
            << MINUTE_BITS
            | minute as u64)
            << SECOND_BITS
            | second as u64)
            << MILLI_BITS
            | milli as u64)
            << OFFSET_BITS)
            | (offset_minutes & ((1 << OFFSET_BITS) - 1)) as u64;
        Self { value }
    }

    #[inline]
    pub fn from_value(value: u64) -> Self {
        Self { value }
    }

    #[inline]
    pub fn value(&self) -> u64 {
        self.value
    }

    #[inline]
    pub fn from_timestamp_millis(ts: i64) -> Self {
        let components = DateTimeComponents::from_timestamp_millis(ts);

        Self::new_utc(
            components.year,
            components.month as _,
            components.day as _,
            components.hour as _,
            components.minute as _,
            components.second as _,
            components.millisecond,
        )
    }

    #[inline]
    pub fn to_timestamp_millis(&self) -> i64 {
        let date_part = EpochDays::from_ymd(self.year() as i32, self.month() as i32, self.day() as i32).to_timestamp_millis();

        let h = self.hour() as i64;
        let m = self.minute() as i64;
        let s = self.second() as i64;
        let o = self.offset_minutes() as i64;
        let seconds = h * 60 * 60 + m * 60 + s - o * 60;
        let millis = self.millisecond() as i64;

        let time_part = seconds * 1000 + millis;

        date_part + time_part
    }

    pub fn from_rfc3339_bytes(input: &[u8]) -> ParseResult<Self> {
        #[cfg(all(not(miri), target_feature = "sse4.1"))]
        {
            let ts = crate::parse::parse_simd(input)?;
            Ok(ts.to_packed())
        }
        #[cfg(not(all(not(miri), target_feature = "sse4.1")))]
        {
            let ts = crate::parse::parse_scalar(input)?;
            Ok(ts.to_packed())
        }
    }

    pub fn from_rfc3339_str(input: &str) -> ParseResult<Self> {
        Self::from_rfc3339_bytes(input.as_bytes())
    }

    #[inline]
    pub fn year(&self) -> u32 {
        (self.value >> (MONTH_BITS + DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) as u32
    }

    #[inline]
    pub fn month(&self) -> u32 {
        ((self.value >> (DAY_BITS + HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MONTH_BITS) - 1))
            as u32
    }

    #[inline]
    pub fn day(&self) -> u32 {
        ((self.value >> (HOUR_BITS + MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << DAY_BITS) - 1)) as u32
    }

    #[inline]
    pub fn hour(&self) -> u32 {
        ((self.value >> (MINUTE_BITS + SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << HOUR_BITS) - 1)) as u32
    }

    #[inline]
    pub fn minute(&self) -> u32 {
        ((self.value >> (SECOND_BITS + MILLI_BITS + OFFSET_BITS)) & ((1 << MINUTE_BITS) - 1)) as u32
    }

    #[inline]
    pub fn second(&self) -> u32 {
        ((self.value >> (MILLI_BITS + OFFSET_BITS)) & ((1 << SECOND_BITS) - 1)) as u32
    }

    #[inline]
    pub fn millisecond(&self) -> u32 {
        ((self.value >> (OFFSET_BITS)) & ((1 << MILLI_BITS) - 1)) as u32
    }

    #[inline]
    pub fn offset_minutes(&self) -> i32 {
        let bits = (self.value & ((1 << OFFSET_BITS) - 1)) as i32;
        // offset is the only field that can be negative and needs sign extension
        bits << (32 - OFFSET_BITS) >> (32 - OFFSET_BITS)
    }

    #[inline]
    pub fn write_rfc3339_bytes<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        let buffer = self.to_rfc3339_bytes();
        writer.write_all(&buffer)
    }

    #[inline]
    pub fn write_rfc3339_str<W: std::fmt::Write>(&self, mut writer: W) -> std::fmt::Result {
        let buffer = self.to_rfc3339_bytes();
        #[cfg(not(debug_assertions))]
        {
            writer.write_str(unsafe { std::str::from_utf8_unchecked(&buffer) })
        }
        #[cfg(debug_assertions)]
        {
            writer.write_str(std::str::from_utf8(&buffer).expect("utf8 string"))
        }
    }

    #[inline]
    pub fn to_rfc3339_bytes(&self) -> [u8; 24] {
        format_to_rfc3339_utc_bytes(
            self.year(),
            self.month(),
            self.day(),
            self.hour(),
            self.minute(),
            self.second(),
            self.millisecond(),
        )
    }

    #[inline]
    pub fn to_rfc3339_string(&self) -> String {
        let buffer = self.to_rfc3339_bytes();
        #[cfg(not(debug_assertions))]
        {
            unsafe { std::str::from_utf8_unchecked(&buffer).to_string() }
        }
        #[cfg(debug_assertions)]
        {
            std::str::from_utf8(&buffer).expect("utf8 string").to_string()
        }
    }
}

impl Display for PackedTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.write_rfc3339_str(f)
    }
}

impl Debug for PackedTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        #[cfg(not(debug_assertions))]
        {
            self.write_rfc3339_str(f)
        }
        #[cfg(debug_assertions)]
        {
            f.write_fmt(format_args!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                self.year(),
                self.month(),
                self.day(),
                self.hour(),
                self.minute(),
                self.second(),
                self.millisecond()
            ))
        }
    }
}

impl From<EpochDays> for PackedTimestamp {
    fn from(epoch_days: EpochDays) -> Self {
        let (year, month, day) = epoch_days.to_ymd();
        PackedTimestamp::new_ymd_utc(year, month as _, day as _)
    }
}

impl TryFrom<&str> for PackedTimestamp {
    type Error = ParseError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        PackedTimestamp::from_rfc3339_str(s)
    }
}

impl FromStr for PackedTimestamp {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PackedTimestamp::from_rfc3339_str(s)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::{PackedTimestamp, ParseError};

    #[test]
    fn test_format() {
        assert_eq!(
            PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 0).to_rfc3339_string(),
            "2022-08-21T17:30:15.000Z".to_owned()
        );
        assert_eq!(
            PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 100).to_rfc3339_string(),
            "2022-08-21T17:30:15.100Z".to_owned()
        );
        assert_eq!(
            PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 123).to_rfc3339_string(),
            "2022-08-21T17:30:15.123Z".to_owned()
        );
        assert_eq!(
            PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250).to_rfc3339_string(),
            "2022-08-21T17:30:15.250Z".to_owned()
        );
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            "2022-08-21T17:30:15.250Z".parse(),
            Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250))
        );
        assert_eq!(
            "2022-08-21T17:30:15.25Z".parse(),
            Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 250))
        );
        assert_eq!(
            "2022-08-21 17:30:15.1Z".parse(),
            Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 100))
        );
        assert_eq!(
            "2022-08-21 17:30:15Z".parse(),
            Ok(PackedTimestamp::new_utc(2022, 8, 21, 17, 30, 15, 0))
        );
        assert_eq!(
            "2022-08-21T17:30:15.250+02:00".parse(),
            Ok(PackedTimestamp::new(2022, 8, 21, 17, 30, 15, 250, 120))
        );
        assert_eq!(
            "2022-08-21T17:30:15.250-02:00".parse(),
            Ok(PackedTimestamp::new(2022, 8, 21, 17, 30, 15, 250, -120))
        );
    }

    #[test]
    fn test_offset_minutes() {
        assert_eq!(
            120,
            PackedTimestamp::new(2022, 8, 21, 17, 30, 15, 250, 120).offset_minutes()
        );
        assert_eq!(
            -120,
            PackedTimestamp::new(2022, 8, 21, 17, 30, 15, 250, -120).offset_minutes()
        );
    }

    #[test]
    fn test_parse_error() {
        assert_eq!(
            PackedTimestamp::try_from("2022-08-21 FOO"),
            Err(ParseError::InvalidLen(14))
        );
        assert_eq!(
            PackedTimestamp::try_from("2022-08-21 00:00"),
            Err(ParseError::InvalidLen(16))
        );
        assert_eq!(
            PackedTimestamp::try_from("2022-08-21 XX:YY::ZZZ"),
            Err(ParseError::InvalidChar(11))
        );
    }

    #[test]
    fn test_packed() {
        let ts = PackedTimestamp::new_utc(2020, 9, 10, 17, 30, 15, 123);
        assert_eq!(2020, ts.year());
        assert_eq!(9, ts.month());
        assert_eq!(10, ts.day());
        assert_eq!(17, ts.hour());
        assert_eq!(30, ts.minute());
        assert_eq!(15, ts.second());
        assert_eq!(123, ts.millisecond());
    }

    #[test]
    fn test_from_timestamp_millis() {
        assert_eq!(
            PackedTimestamp::from_timestamp_millis(0),
            PackedTimestamp::new_utc(1970, 1, 1, 0, 0, 0, 0)
        );

        assert_eq!(
            PackedTimestamp::from_timestamp_millis(1000),
            PackedTimestamp::new_utc(1970, 1, 1, 0, 0, 1, 0)
        );

        assert_eq!(
            PackedTimestamp::from_timestamp_millis(24 * 60 * 60 * 1000),
            PackedTimestamp::new_utc(1970, 1, 2, 0, 0, 0, 0)
        );

        assert_eq!(
            PackedTimestamp::from_timestamp_millis(-1),
            PackedTimestamp::new_utc(1969, 12, 31, 23, 59, 59, 999)
        );

        assert_eq!(
            PackedTimestamp::from_timestamp_millis(-1000),
            PackedTimestamp::new_utc(1969, 12, 31, 23, 59, 59, 0)
        );

        assert_eq!(
            PackedTimestamp::from_timestamp_millis(-24 * 60 * 60 * 1000),
            PackedTimestamp::new_utc(1969, 12, 31, 0, 0, 0, 0)
        );
    }

    #[test]
    fn test_to_timestamp_millis() {
        assert_eq!(
            PackedTimestamp::new_utc(1970, 1, 1, 0, 0, 0, 0).to_timestamp_millis(),
            0
        );
        assert_eq!(
            PackedTimestamp::new_utc(2023, 7, 3, 22, 55, 30, 123).to_timestamp_millis(),
            1688424930123
        );

        assert_eq!(
            PackedTimestamp::new(2023, 7, 3, 22, 55, 30, 123, 120).to_timestamp_millis(),
            1688417730123
        );
        assert_eq!(
            PackedTimestamp::new(2023, 7, 3, 22, 55, 30, 123, -120).to_timestamp_millis(),
            1688432130123
        );
    }
}
