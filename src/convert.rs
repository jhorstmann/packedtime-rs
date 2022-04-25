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

/// branchless calculation of whether the given year is a leap year
#[inline]
fn is_leap_year(year: i32) -> bool {
    ((year % 4) == 0) & (((year % 100) != 0) | ((year % 400) == 0))
}

/// Convert a date to the number of days since the unix epoch 1970-01-01.
/// See https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java#L1634
#[inline]
pub(crate) fn to_epoch_day(year: i32, month: i32, day: i32) -> i32 {
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

    total - DAYS_0000_TO_1970
}

/// Convert the number of days since the unix epoch into a (year, month, day) tuple.
/// See https://github.com/ThreeTen/threetenbp/blob/master/src/main/java/org/threeten/bp/LocalDate.java#L281
#[inline]
pub(crate) fn from_epoch_day(epoch_days: i32) -> (i32, i32, i32) {
    let mut zero_day = epoch_days + DAYS_0000_TO_1970;
    // find the march-based year
    zero_day -= 60;  // adjust to 0000-03-01 so leap day is at end of four year cycle
    let mut adjust = 0;
    if zero_day < 0 && SUPPORT_NEGATIVE_YEAR {
        // adjust negative years to positive for calculation
        let adjust_cycles = (zero_day + 1) / DAYS_PER_CYCLE - 1;
        adjust = adjust_cycles * 400;
        zero_day += -adjust_cycles * DAYS_PER_CYCLE;
    }
    let mut year_est = (400 * zero_day + 591) / DAYS_PER_CYCLE;
    let mut doy_est = zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);
    if doy_est < 0 {
        // fix estimate
        year_est -= 1;
        doy_est = zero_day - (365 * year_est + year_est / 4 - year_est / 100 + year_est / 400);
    }
    year_est += adjust;  // reset any negative year
    let march_doy0 = doy_est;

    // convert march-based values back to january-based
    let march_month0 = (march_doy0 * 5 + 2) / 153;
    let month = (march_month0 + 2) % 12 + 1;
    let dom = march_doy0 - (march_month0 * 306 + 5) / 10 + 1;
    year_est += march_month0 / 10;

    (year_est, month, dom)
}

#[inline]
pub fn timestamp_millis_to_epoch_days(ts: i64) -> i32 {
    // todo: find a way to get this vectorizable using integer operations or verify it is exact for all timestamps
    timestamp_float_to_epoch_days(ts as f64)
}

#[inline]
pub fn timestamp_float_to_epoch_days(ts: f64) -> i32 {
    unsafe { (ts * (1.0 / MILLIS_PER_DAY as f64)).to_int_unchecked() }
}

#[inline]
pub fn date_trunc_month_epoch_days(epoch_days: i32) -> i32 {
    let (y, m, d) = from_epoch_day(epoch_days);
    to_epoch_day(y, m, 0)
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
    let epoch_days = timestamp_millis_to_epoch_days(ts);
    let truncated = date_trunc_month_epoch_days(epoch_days) as i64;
    truncated * MILLIS_PER_DAY
}

#[inline]
pub fn date_trunc_month_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = timestamp_float_to_epoch_days(ts);
    let truncated = date_trunc_month_epoch_days(epoch_days) as f64;
    truncated * (MILLIS_PER_DAY as f64)
}

#[inline]
pub fn date_trunc_year_epoch_days(epoch_days: i32) -> i32 {
    let (y, m, d) = from_epoch_day(epoch_days);
    to_epoch_day(y, 0, 0)
}

#[inline]
pub fn date_trunc_year_timestamp_millis(ts: i64) -> i64 {
    let epoch_days = timestamp_millis_to_epoch_days(ts);
    let truncated = date_trunc_year_epoch_days(epoch_days) as i64;
    truncated * MILLIS_PER_DAY
}

#[inline]
pub fn date_trunc_year_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = timestamp_float_to_epoch_days(ts);
    let truncated = date_trunc_year_epoch_days(epoch_days) as f64;
    truncated * (MILLIS_PER_DAY as f64)
}

#[inline]
pub fn date_trunc_quarter_epoch_days(epoch_days: i32) -> i32 {
    let (y, m, d) = from_epoch_day(epoch_days);
    to_epoch_day(y, m / 4, 0)
}

#[inline]
pub fn date_trunc_quarter_timestamp_millis(ts: i64) -> i64 {
    let epoch_days = timestamp_millis_to_epoch_days(ts);
    let truncated = date_trunc_quarter_epoch_days(epoch_days) as i64;
    truncated * MILLIS_PER_DAY
}

#[inline]
pub fn date_trunc_quarter_timestamp_millis_float(ts: f64) -> f64 {
    let epoch_days = timestamp_float_to_epoch_days(ts);
    let truncated = date_trunc_quarter_epoch_days(epoch_days) as f64;
    truncated * (MILLIS_PER_DAY as f64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_to_epoch_day() {
        assert_eq!(0, super::to_epoch_day(1970, 1, 1));
        assert_eq!(18998, super::to_epoch_day(2022, 1, 6));
    }
}