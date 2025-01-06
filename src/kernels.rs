use crate::{EpochDays, MILLIS_PER_DAY};

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
    let truncated = epoch_days.date_trunc_year();
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
pub fn date_part_year_timestamp_millis(ts: i64) -> i32 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    epoch_days.extract_year()
}

#[inline]
pub fn date_part_month_timestamp_millis(ts: i64) -> i32 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    epoch_days.extract_month()
}

#[inline]
fn timestamp_to_epoch_days_and_remainder(ts: i64) -> (EpochDays, i64) {
    let (days, millis) = (ts.div_euclid(MILLIS_PER_DAY), ts.rem_euclid(MILLIS_PER_DAY));
    (EpochDays::new(days as i32), millis)
}

#[inline]
fn timestamp_to_epoch_days_and_remainder_float(ts: f64) -> (EpochDays, f64) {
    let days = (ts * (1.0 / MILLIS_PER_DAY as f64)).floor();
    let millis = ts - days * (MILLIS_PER_DAY as f64);
    (EpochDays::new(unsafe { days.to_int_unchecked() }), millis)
}

#[inline]
pub fn date_add_month_timestamp_millis(ts: i64, months: i32) -> i64 {
    let (epoch_days, millis) = timestamp_to_epoch_days_and_remainder(ts);
    let new_epoch_days = epoch_days.add_months(months);
    new_epoch_days.to_timestamp_millis() + millis
}

#[inline]
pub fn date_add_month_timestamp_millis_float(ts: f64, months: i32) -> f64 {
    let (epoch_days, millis) = timestamp_to_epoch_days_and_remainder_float(ts);
    let new_epoch_days = epoch_days.add_months(months);
    new_epoch_days.to_timestamp_millis_float() + millis
}

#[inline]
fn timestamp_to_year_month_millis_of_month(ts: i64) -> (i32, i32, i64) {
    let (ed, millis) = timestamp_to_epoch_days_and_remainder(ts);
    let (year, month, day) = ed.to_ymd();
    let millis_of_month = (day as i64)*MILLIS_PER_DAY + millis;
    (year, month, millis_of_month)
}

#[inline]
fn timestamp_to_year_month_millis_of_month_float(ts: f64) -> (i32, i32, f64) {
    let (ed, millis) = timestamp_to_epoch_days_and_remainder_float(ts);
    let (year, month, day) = ed.to_ymd();
    let millis_of_month = (day as f64)*(MILLIS_PER_DAY as f64) + millis;
    (year, month, millis_of_month)
}

#[inline]
pub fn date_diff_month_timestamp_millis(t0: i64, t1: i64) -> i32 {
    let (y0, m0, ms0) = timestamp_to_year_month_millis_of_month(t0);
    let (y1, m1, ms1) = timestamp_to_year_month_millis_of_month(t1);
    (y1*12 + m1) - (y0*12 + m0) - ((ms1 < ms0) as i32)
}

#[inline]
pub fn date_diff_month_timestamp_millis_float(t0: f64, t1: f64) -> i32 {
    let (y0, m0, ms0) = timestamp_to_year_month_millis_of_month_float(t0);
    let (y1, m1, ms1) = timestamp_to_year_month_millis_of_month_float(t1);
    (y1*12 + m1) - (y0*12 + m0) - ((ms1 < ms0) as i32)
}

#[inline]
pub fn date_diff_year_timestamp_millis(t0: i64, t1: i64) -> i32 {
    let (y0, m0, ms0) = timestamp_to_year_month_millis_of_month(t0);
    let (y1, m1, ms1) = timestamp_to_year_month_millis_of_month(t1);
    y1 - y0 - (((m1, ms1) < (m0, ms0)) as i32)
}

#[inline]
pub fn date_diff_year_timestamp_millis_float(t0: f64, t1: f64) -> i32 {
    let (y0, m0, ms0) = timestamp_to_year_month_millis_of_month_float(t0);
    let (y1, m1, ms1) = timestamp_to_year_month_millis_of_month_float(t1);
    y1 - y0 - (((m1, ms1) < (m0, ms0)) as i32)
}

#[inline]
pub fn days_in_month_timestamp_millis(ts: i64) -> i32 {
    let epoch_days = EpochDays::from_timestamp_millis(ts);
    epoch_days.days_in_month()
}

#[cfg(test)]
mod tests {
    use crate::epoch_days::EpochDays;
    use crate::{date_add_month_timestamp_millis, date_diff_month_timestamp_millis, date_diff_year_timestamp_millis, date_trunc_month_timestamp_millis, date_trunc_quarter_timestamp_millis, date_trunc_year_timestamp_millis};
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
            date_trunc_month_timestamp_millis(1640995200_000)
        );
        assert_eq!(
            1656633600_000,
            date_trunc_month_timestamp_millis(1658765238_000)
        );
    }

    #[test]
    fn test_date_add_months() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(epoch_day.add_months(1), EpochDays::from_ymd(2022, 8, 31));
        assert_eq!(epoch_day.add_months(2), EpochDays::from_ymd(2022, 9, 30));
        assert_eq!(epoch_day.add_months(3), EpochDays::from_ymd(2022, 10, 31));
        assert_eq!(epoch_day.add_months(4), EpochDays::from_ymd(2022, 11, 30));
        assert_eq!(epoch_day.add_months(5), EpochDays::from_ymd(2022, 12, 31));
    }

    #[test]
    fn test_date_add_months_year_boundary() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(epoch_day.add_months(6), EpochDays::from_ymd(2023, 1, 31));
        assert_eq!(epoch_day.add_months(7), EpochDays::from_ymd(2023, 2, 28));
        assert_eq!(epoch_day.add_months(8), EpochDays::from_ymd(2023, 3, 31));
    }

    #[test]
    fn test_date_add_months_leap_year() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(epoch_day.add_months(19), EpochDays::from_ymd(2024, 2, 29));

        let epoch_day = EpochDays::from_ymd(2022, 2, 28);
        assert_eq!(epoch_day.add_months(24), EpochDays::from_ymd(2024, 2, 28));

        let epoch_day = EpochDays::from_ymd(2024, 2, 29);
        assert_eq!(epoch_day.add_months(12), EpochDays::from_ymd(2025, 2, 28));
    }

    #[test]
    fn test_date_add_months_negative() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(epoch_day.add_months(-1), EpochDays::from_ymd(2022, 6, 30));
        assert_eq!(epoch_day.add_months(-2), EpochDays::from_ymd(2022, 5, 31));
        assert_eq!(epoch_day.add_months(-3), EpochDays::from_ymd(2022, 4, 30));
        assert_eq!(epoch_day.add_months(-4), EpochDays::from_ymd(2022, 3, 31));
        assert_eq!(epoch_day.add_months(-5), EpochDays::from_ymd(2022, 2, 28));
        assert_eq!(epoch_day.add_months(-6), EpochDays::from_ymd(2022, 1, 31));
        assert_eq!(epoch_day.add_months(-7), EpochDays::from_ymd(2021, 12, 31));
    }

    #[test]
    fn test_date_add_months_negative_year_boundary() {
        let epoch_day = EpochDays::from_ymd(2022, 7, 31);
        assert_eq!(epoch_day.add_months(-7), EpochDays::from_ymd(2021, 12, 31));
    }

    #[test]
    fn test_date_add_months_timestamp_millis() {
        assert_eq!(
            date_add_month_timestamp_millis(1661102969_000, 1),
            1663781369000
        );
        assert_eq!(
            date_add_month_timestamp_millis(1661102969_000, 12),
            1692638969000
        );
    }

    #[test]
    fn test_date_diff_months() {
        assert_eq!(date_diff_month_timestamp_millis(EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis(), EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis()), 0);
        assert_eq!(date_diff_month_timestamp_millis(EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis(), EpochDays::from_ymd(2023, 11, 1).to_timestamp_millis()), 1);
        assert_eq!(date_diff_month_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2023, 11, 14).to_timestamp_millis()), 0);
        assert_eq!(date_diff_month_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2023, 11, 15).to_timestamp_millis()), 1);
        assert_eq!(date_diff_month_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2023, 11, 16).to_timestamp_millis()), 1);
    }

    #[test]
    fn test_date_diff_years() {
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis(), EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis()), 0);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2023, 10, 1).to_timestamp_millis(), EpochDays::from_ymd(2023, 11, 1).to_timestamp_millis()), 0);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2024, 10, 14).to_timestamp_millis()), 0);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2024, 10, 15).to_timestamp_millis()), 1);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2023, 10, 15).to_timestamp_millis(), EpochDays::from_ymd(2024, 10, 16).to_timestamp_millis()), 1);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2024, 2, 29).to_timestamp_millis(), EpochDays::from_ymd(2025, 2, 28).to_timestamp_millis()), 0);
        assert_eq!(date_diff_year_timestamp_millis(EpochDays::from_ymd(2024, 2, 29).to_timestamp_millis(), EpochDays::from_ymd(2025, 3, 1).to_timestamp_millis()), 1);
    }

    #[test]
    #[cfg_attr(any(miri, not(feature = "expensive_tests")), ignore)]
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
    #[cfg_attr(any(miri, not(feature = "expensive_tests")), ignore)]
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
