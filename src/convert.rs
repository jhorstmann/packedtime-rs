/// Conversions from/to number of days since the unix epoch.
/// Ported from https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java
/// Original code has the following license:
/*
 * Copyright (c) 2007-present, Stephen Colebourne & Michael Nascimento Santos
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 *  * Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 *  * Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 *  * Neither the name of JSR-310 nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 * A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR
 * CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 * EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 * PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
 * LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 * NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 * SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

const SECONDS_PER_DAY: i32 = 86400;
const DAYS_PER_CYCLE: i32 = 146097;
const DAYS_0000_TO_1970: i32 = (DAYS_PER_CYCLE * 5) - (30 * 365 + 7);

const MILLIS_PER_DAY: i64 = 24 * 60 * 60 * 1000;

const SUPPORT_NEGATIVE_YEAR: bool = false;

// stored as i32 instead of smaller types in order to access via vectorized gather instructions
static DAYS_PER_MONTH: [[i32; 12]; 2] = [
    [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
    [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31],
];

/// branchless calculation of whether the given year is a leap year
#[inline]
fn is_leap_year(year: i32) -> bool {
    ((year % 4) == 0) & ((year % 100) != 0) | ((year % 400) == 0)
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct EpochDays(i32);

impl EpochDays {
    #[inline]
    pub fn new(epoch_days: i32) -> Self {
        Self(epoch_days)
    }

    #[inline]
    pub fn days(&self) -> i32 {
        self.0
    }

    /// Convert a date to the number of days since the unix epoch 1970-01-01.
    /// See https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java#L1634
    #[inline]
    pub fn from_ymd(year: i32, month: i32, day: i32) -> Self {
        let y = year;
        let m = month;
        let mut total = 365 * y;

        total += if y < 0 && SUPPORT_NEGATIVE_YEAR {
            -(y / -4 - y / -100 + y / -400)
        } else {
            (y + 3) / 4 - (y + 99) / 100 + (y + 399) / 400
        };

        total += ((367 * m - 362) / 12);
        total += day - 1;
        total -= if m > 2 {
            if !is_leap_year(year) {
                2
            } else {
                1
            }
        } else {
            0
        };

        Self(total - DAYS_0000_TO_1970)
    }

    /// Convert the number of days since the unix epoch into a (year, month, day) tuple.
    /// See https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java#L281
    /// The resulting month and day values are 1-based.
    #[inline]
    pub fn to_ymd(&self) -> (i32, i32, i32) {
        let epoch_days = self.0;
        let mut zero_day = epoch_days + DAYS_0000_TO_1970;
        // find the march-based year
        zero_day -= 60; // adjust to 0000-03-01 so leap day is at end of four year cycle
        let mut adjust = 0;
        if zero_day < 0 && SUPPORT_NEGATIVE_YEAR {
            // adjust negative years to positive for calculation
            let adjust_cycles = (zero_day + 1) / DAYS_PER_CYCLE - 1;
            adjust = adjust_cycles * 400;
            zero_day += -adjust_cycles * DAYS_PER_CYCLE;
        }
        let mut year_est = (400 * zero_day + 591) / DAYS_PER_CYCLE;
        let mut doy_est =
            zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);
        if doy_est < 0 {
            // fix estimate
            year_est -= 1;
            doy_est = zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);
        }
        year_est += adjust; // reset any negative year
        let march_doy0 = doy_est;

        // convert march-based values back to january-based
        let march_month0 = (march_doy0 * 5 + 2) / 153;
        let month = (march_month0 + 2) % 12 + 1;
        let dom = march_doy0 - (march_month0 * 306 + 5) / 10 + 1;
        year_est += march_month0 / 10;

        (year_est, month, dom)
    }

    #[inline]
    pub fn from_timestamp_millis(ts: i64) -> Self {
        // todo: find a way to get this vectorizable using integer operations or verify it is exact for all timestamps
        Self::from_timestamp_millis_float(ts as f64)
    }

    #[inline]
    pub fn from_timestamp_millis_float(ts: f64) -> Self {
        let epoch_days = (ts * (1.0 / MILLIS_PER_DAY as f64)).floor();
        Self(unsafe { epoch_days.to_int_unchecked() })
    }

    #[inline]
    pub fn to_timestamp_millis(&self) -> i64 {
        (self.0 as i64) * MILLIS_PER_DAY
    }

    #[inline]
    pub fn to_timestamp_millis_float(&self) -> f64 {
        (self.0 as f64) * (MILLIS_PER_DAY as f64)
    }

    /// Adds the given number of `months` to `epoch_days`.
    /// If the day would be out of range for the resulting month and the `CLAMP_DAYS` flag is set
    /// then the date will be clamped to the end of the month.
    ///
    /// For example: 2022-01-31 + 1month => 2022-02-28
    ///
    /// Setting `CLAMP_DAYS` to false might improve performance if the date is guaranteed to fall on a valid day.
    #[inline]
    pub fn add_months<const CLAMP_DAYS: bool>(&self, months: i32) -> Self {
        let (mut y, mut m, mut d) = self.to_ymd();
        let mut m0 = m - 1;
        m0 += months;
        y += m0.div_euclid(12);
        m0 = m0.rem_euclid(12);
        if CLAMP_DAYS {
            d = d.min(DAYS_PER_MONTH[is_leap_year(y) as usize][m0 as usize] as _);
        }
        m = m0 + 1;
        Self::from_ymd(y, m, d)
    }

    #[inline]
    pub fn date_trunc_month(&self) -> Self {
        let (y, m, d) = self.to_ymd();
        Self::from_ymd(y, m, 1)
    }

    #[inline]
    pub fn date_trunc_year(&self) -> Self {
        let (y, m, d) = self.to_ymd();
        Self::from_ymd(y, 1, 1)
    }

    #[inline]
    pub fn date_trunc_quarter(&self) -> Self {
        let (y, m, d) = self.to_ymd();
        Self::from_ymd(y, (m - 1) / 3 * 3 + 1, 1)
    }
}

#[inline]
fn truncate_millis(ts: i64, truncate: i64) -> i64 {
    ts / truncate * truncate
}

#[inline]
fn truncate_millis_float(ts: f64, truncate: i64) -> f64 {
    let truncate = truncate as f64;
    (ts / truncate).floor() * truncate
}

#[inline]
pub fn date_trunc_day_timestamp_millis(ts: i64) -> i64 {
    truncate_millis(ts, MILLIS_PER_DAY)
}

#[inline]
pub fn date_trunc_day_timestamp_millis_float(ts: f64) -> f64 {
    truncate_millis_float(ts, MILLIS_PER_DAY)
}

#[inline]
pub fn date_trunc_week_timestamp_millis(ts: i64) -> i64 {
    // unix epoch starts on a thursday
    let offset = 4 * MILLIS_PER_DAY;
    truncate_millis(ts - offset, 7 * MILLIS_PER_DAY) + offset
}

#[inline]
pub fn date_trunc_week_timestamp_millis_float(ts: f64) -> f64 {
    let offset = (4 * MILLIS_PER_DAY) as f64;
    truncate_millis_float(ts - offset, 7 * MILLIS_PER_DAY) + offset
}

#[inline]
pub fn date_trunc_month_timestamp_millis(ts: i64) -> i64 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    let truncated = epoch_days.date_trunc_month();
    truncated.to_timestamp_millis()
}

#[inline]
pub fn date_trunc_month_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = EpochDays::from_timestamp_millis_float(ts);
    let truncated = epoch_days.date_trunc_month();
    truncated.to_timestamp_millis_float()
}

#[inline]
pub fn date_trunc_year_timestamp_millis(ts: i64) -> i64 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    let truncated = epoch_days.date_trunc_year();
    truncated.to_timestamp_millis()
}

#[inline]
pub fn date_trunc_year_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = EpochDays::from_timestamp_millis_float(ts);
    let truncated = epoch_days.date_trunc_month();
    truncated.to_timestamp_millis_float()
}

#[inline]
pub fn date_trunc_quarter_timestamp_millis(ts: i64) -> i64 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    let truncated = epoch_days.date_trunc_quarter();
    truncated.to_timestamp_millis()
}

#[inline]
pub fn date_trunc_quarter_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = EpochDays::from_timestamp_millis_float(ts);
    let truncated = epoch_days.date_trunc_quarter();
    truncated.to_timestamp_millis_float()
}

#[inline]
pub fn date_add_month_timestamp_millis(ts: i64, months: i32) -> i64 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    let new_epoch_days = epoch_days.add_months::<true>(months);
    new_epoch_days.to_timestamp_millis()
}

#[inline]
pub fn date_add_month_timestamp_millis_unclamped(ts: i64, months: i32) -> i64 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    let new_epoch_days = epoch_days.add_months::<false>(months);
    new_epoch_days.to_timestamp_millis()
}

#[cfg(test)]
mod tests {
    use crate::convert::EpochDays;
    use crate::{
        date_trunc_month_timestamp_millis, date_trunc_quarter_timestamp_millis,
        date_trunc_year_timestamp_millis,
    };
    use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime};
    use std::ops::Add;

    fn timestamp_to_naive_date_time(ts: i64) -> NaiveDateTime {
        NaiveDateTime::from_timestamp(ts / 1000, 0).add(chrono::Duration::milliseconds(ts % 1000))
    }

    fn date_trunc_year_chrono(ts: i64) -> i64 {
        let ndt = timestamp_to_naive_date_time(ts);
        let truncated = NaiveDateTime::new(
            NaiveDate::from_ymd(ndt.year(), 1, 1),
            NaiveTime::from_hms(0, 0, 0),
        );
        truncated.timestamp_millis()
    }

    fn date_trunc_month_chrono(ts: i64) -> i64 {
        let ndt = timestamp_to_naive_date_time(ts);
        let truncated = NaiveDateTime::new(
            NaiveDate::from_ymd(ndt.year(), ndt.month(), 1),
            NaiveTime::from_hms(0, 0, 0),
        );
        truncated.timestamp_millis()
    }

    #[test]
    fn test_to_epoch_day() {
        assert_eq!(0, EpochDays::from_ymd(1970, 1, 1).0);
        assert_eq!(1, EpochDays::from_ymd(1970, 1, 2).0);
        assert_eq!(365, EpochDays::from_ymd(1971, 1, 1).0);
        assert_eq!(365 * 2, EpochDays::from_ymd(1972, 1, 1).0);
        assert_eq!(365 * 2 + 366, EpochDays::from_ymd(1973, 1, 1).0);

        assert_eq!(18998, EpochDays::from_ymd(2022, 1, 6).0);
        assert_eq!(19198, EpochDays::from_ymd(2022, 7, 25).0);
    }

    #[test]
    fn test_date_trunc_year_epoch_days() {
        assert_eq!(18993, EpochDays::new(19198).date_trunc_year().days());
    }

    #[test]
    fn test_date_trunc_year_millis() {
        assert_eq!(
            1640995200_000,
            date_trunc_year_timestamp_millis(1640995200_000)
        );
        assert_eq!(
            1640995200_000,
            date_trunc_year_timestamp_millis(1658765238_000)
        );
    }

    #[test]
    fn test_date_trunc_quarter_millis() {
        assert_eq!(
            1640995200_000,
            date_trunc_quarter_timestamp_millis(1640995200_000)
        );
        assert_eq!(
            1656633600_000,
            date_trunc_quarter_timestamp_millis(1658766592_000)
        );
    }

    #[test]
    fn test_date_trunc_month_millis() {
        assert_eq!(
            1640995200_000,
            super::date_trunc_month_timestamp_millis(1640995200_000)
        );
        assert_eq!(
            1656633600_000,
            super::date_trunc_month_timestamp_millis(1658765238_000)
        );
    }

    #[test]
    fn test_date_add_months() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(
            epoch_day.add_months::<true>(1),
            EpochDays::from_ymd(2022, 8, 31)
        );
        assert_eq!(
            epoch_day.add_months::<true>(2),
            EpochDays::from_ymd(2022, 9, 30)
        );
        assert_eq!(
            epoch_day.add_months::<true>(3),
            EpochDays::from_ymd(2022, 10, 31)
        );
        assert_eq!(
            epoch_day.add_months::<true>(4),
            EpochDays::from_ymd(2022, 11, 30)
        );
        assert_eq!(
            epoch_day.add_months::<true>(5),
            EpochDays::from_ymd(2022, 12, 31)
        );
    }

    #[test]
    fn test_date_add_months_year_boundary() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(
            epoch_day.add_months::<true>(6),
            EpochDays::from_ymd(2023, 1, 31)
        );
        assert_eq!(
            epoch_day.add_months::<true>(7),
            EpochDays::from_ymd(2023, 2, 28)
        );
        assert_eq!(
            epoch_day.add_months::<true>(8),
            EpochDays::from_ymd(2023, 3, 31)
        );
    }

    #[test]
    fn test_date_add_months_leap_year() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(
            epoch_day.add_months::<true>(19),
            EpochDays::from_ymd(2024, 2, 29)
        );

        let epoch_day = EpochDays::from_ymd(2022, 2, 28);
        assert_eq!(
            epoch_day.add_months::<true>(24),
            EpochDays::from_ymd(2024, 2, 28)
        );

        let epoch_day = EpochDays::from_ymd(2024, 2, 29);
        assert_eq!(
            epoch_day.add_months::<true>(12),
            EpochDays::from_ymd(2025, 2, 28)
        );
    }

    #[test]
    fn test_date_add_months_negative() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(
            epoch_day.add_months::<true>(-1),
            EpochDays::from_ymd(2022, 6, 30)
        );
        assert_eq!(
            epoch_day.add_months::<true>(-2),
            EpochDays::from_ymd(2022, 5, 31)
        );
        assert_eq!(
            epoch_day.add_months::<true>(-3),
            EpochDays::from_ymd(2022, 4, 30)
        );
        assert_eq!(
            epoch_day.add_months::<true>(-4),
            EpochDays::from_ymd(2022, 3, 31)
        );
        assert_eq!(
            epoch_day.add_months::<true>(-5),
            EpochDays::from_ymd(2022, 2, 28)
        );
        assert_eq!(
            epoch_day.add_months::<true>(-6),
            EpochDays::from_ymd(2022, 1, 31)
        );
    }

    #[test]
    fn test_date_add_months_negative_year_boundary() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(
            epoch_day.add_months::<true>(-7),
            EpochDays::from_ymd(2021, 12, 31)
        );
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn test_date_trunc_year_exhaustive() {
        let start = chrono::NaiveDate::from_ymd(1700, 1, 1)
            .and_hms(0, 0, 0)
            .timestamp_millis();
        let end = chrono::NaiveDate::from_ymd(2500, 1, 1)
            .and_hms(0, 0, 0)
            .timestamp_millis();

        for ts in (start..end).step_by(60_000) {
            let trunc_chrono = date_trunc_year_chrono(ts);
            let trunc_packed = date_trunc_year_timestamp_millis(ts);
            assert_eq!(
                trunc_chrono, trunc_packed,
                "{} != {} for {}",
                trunc_chrono, trunc_packed, ts
            );

            let ts = ts + 59_999;
            let trunc_chrono = date_trunc_year_chrono(ts);
            let trunc_packed = date_trunc_year_timestamp_millis(ts);
            assert_eq!(
                trunc_chrono, trunc_packed,
                "{} != {} for {}",
                trunc_chrono, trunc_packed, ts
            );
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[cfg_attr(not(feature = "expensive_tests"), ignore)]
    fn test_date_trunc_month_exhaustive() {
        let start = chrono::NaiveDate::from_ymd(1700, 1, 1)
            .and_hms(0, 0, 0)
            .timestamp_millis();
        let end = chrono::NaiveDate::from_ymd(2500, 1, 1)
            .and_hms(0, 0, 0)
            .timestamp_millis();

        for ts in (start..end).step_by(60_000) {
            let trunc_chrono = date_trunc_month_chrono(ts);
            let trunc_packed = date_trunc_month_timestamp_millis(ts);
            assert_eq!(
                trunc_packed, trunc_chrono,
                "{} != {} for {}",
                trunc_packed, trunc_chrono, ts
            );

            let ts = ts + 59_999;
            let trunc_chrono = date_trunc_month_chrono(ts);
            let trunc_packed = date_trunc_month_timestamp_millis(ts);
            assert_eq!(
                trunc_chrono, trunc_packed,
                "{} != {} for {}",
                trunc_chrono, trunc_packed, ts
            );
        }
    }
}
