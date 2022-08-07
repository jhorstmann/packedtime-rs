use crate::MILLIS_PER_DAY;

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

#[cfg(test)]
mod tests {
    use crate::EpochDays;

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
}
