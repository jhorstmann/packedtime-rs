use crate::MILLIS_PER_DAY;

/// Conversions from/to number of days since the unix epoch.
/// Ported from <https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java>
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

#[inline]
fn days_per_month(year: i32, zero_based_month: i32) -> i32 {
    let is_leap = is_leap_year(year);
    let is_feb = zero_based_month == 1;
    let mut days = 30 + ((zero_based_month % 2) != (zero_based_month <= 6) as i32) as i32;
    days -= (2 - is_leap as i32  ) * (is_feb as i32);
    days
}

/// A date represented as the number of days since the unix epoch 1970-01-01.
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

        total -= 0_i32.wrapping_sub((m > 2) as i32) & (1 + (!is_leap_year(year) as i32));

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

        if !SUPPORT_NEGATIVE_YEAR {
            year_est &= i32::MAX;
        }

        let mut doy_est =
            zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);

        // fix estimate
        year_est -= (doy_est < 0) as i32;
        if !SUPPORT_NEGATIVE_YEAR {
            year_est &= i32::MAX;
        }

        doy_est = zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);

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
        // Converting to f64 is not necessarily faster but allows autovectorization if used in a loop
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
    /// If the day would be out of range for the resulting month
    /// then the date will be clamped to the end of the month.
    ///
    /// For example: 2022-01-31 + 1month => 2022-02-28
    #[inline]
    pub fn add_months(&self, months: i32) -> Self {
        let (mut y, mut m, mut d) = self.to_ymd();
        let mut m0 = m - 1;
        m0 += months;
        y += m0.div_euclid(12);
        m0 = m0.rem_euclid(12);
        d = d.min(days_per_month(y, m0));
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

    #[inline]
    pub fn extract_year(&self) -> i32 {
        self.to_ymd().0
    }

    #[inline]
    pub fn extract_month(&self) -> i32 {
        self.to_ymd().1
    }

    #[inline]
    pub fn extract_quarter(&self) -> i32 {
        (self.to_ymd().1 - 1) / 3 + 1
    }

    #[inline]
    pub fn extract_day_of_month(&self) -> i32 {
        self.to_ymd().2
    }
}

#[cfg(test)]
mod tests {
    use crate::epoch_days::{days_per_month, DAYS_PER_MONTH, is_leap_year};
    use crate::EpochDays;

    #[test]
    fn test_is_leap_year() {
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(1999));

        assert!(is_leap_year(2000));

        assert!(!is_leap_year(2001));
        assert!(!is_leap_year(2002));
        assert!(!is_leap_year(2003));

        assert!(is_leap_year(2004));
        assert!(is_leap_year(2020));
    }

    #[test]
    fn test_days_per_month() {
        for i in 0..12 {
            assert_eq!(days_per_month(2023, i as i32), DAYS_PER_MONTH[0][i], "non-leap: {i}");
        }
        for i in 0..12 {
            assert_eq!(days_per_month(2020, i as i32), DAYS_PER_MONTH[1][i], "leap: {i}");
        }
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
    fn test_extract_year() {
        assert_eq!(2022, EpochDays::from_ymd(2022, 1, 1).extract_year());
        assert_eq!(2022, EpochDays::from_ymd(2022, 8, 24).extract_year());
        assert_eq!(2022, EpochDays::from_ymd(2022, 12, 31).extract_year());
    }

    #[test]
    fn test_extract_month() {
        assert_eq!(1, EpochDays::from_ymd(2000, 1, 1).extract_month());
        assert_eq!(2, EpochDays::from_ymd(2000, 2, 1).extract_month());
        assert_eq!(2, EpochDays::from_ymd(2000, 2, 29).extract_month());
        assert_eq!(1, EpochDays::from_ymd(2022, 1, 1).extract_month());
        assert_eq!(8, EpochDays::from_ymd(2022, 8, 24).extract_month());
        assert_eq!(12, EpochDays::from_ymd(2022, 12, 31).extract_month());
    }

    #[test]
    fn test_extract_day() {
        assert_eq!(1, EpochDays::from_ymd(2000, 1, 1).extract_day_of_month());
        assert_eq!(1, EpochDays::from_ymd(2000, 2, 1).extract_day_of_month());
        assert_eq!(29, EpochDays::from_ymd(2000, 2, 29).extract_day_of_month());
        assert_eq!(1, EpochDays::from_ymd(2000, 3, 1).extract_day_of_month());
    }

    #[test]
    fn test_extract_quarter() {
        assert_eq!(1, EpochDays::from_ymd(2000, 1, 1).extract_quarter());
        assert_eq!(1, EpochDays::from_ymd(2000, 2, 1).extract_quarter());
        assert_eq!(1, EpochDays::from_ymd(2000, 3, 31).extract_quarter());
        assert_eq!(2, EpochDays::from_ymd(2000, 4, 1).extract_quarter());
        assert_eq!(3, EpochDays::from_ymd(2000, 7, 1).extract_quarter());
        assert_eq!(4, EpochDays::from_ymd(2000, 10, 1).extract_quarter());
        assert_eq!(4, EpochDays::from_ymd(2000, 12, 31).extract_quarter());
    }
}
