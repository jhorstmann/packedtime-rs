use crate::{EpochDays, MILLIS_PER_DAY, PackedTimestamp};

/// Internal struct containing the components of a DateTime as separate fields.
#[derive(PartialEq, Clone, Debug, Default)]
pub(crate) struct DateTimeComponents {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u32,
    pub offset_minute: i32,
}

impl DateTimeComponents {
    #[inline(always)]
    pub(crate) fn from_timestamp_millis(ts: i64) -> Self {
        let epoch_days = ts.div_euclid(MILLIS_PER_DAY) as i32;
        let milli_of_day = ts.rem_euclid(MILLIS_PER_DAY) as u32;
        let millisecond = milli_of_day % 1000;
        let second_of_day = milli_of_day / 1000;
        let second = second_of_day % 60;
        let minute_of_day = second_of_day / 60;
        let minute = minute_of_day % 60;
        let hour_of_day = minute_of_day / 60;

        let (year, month, day) = EpochDays::new(epoch_days).to_ymd();

        Self {
            year,
            month: month as _,
            day: day as _,
            hour: hour_of_day as _,
            minute: minute as _,
            second: second as _,
            millisecond,
            offset_minute: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn new(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u32,
    ) -> Self {
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

    #[inline(always)]
    pub(crate) fn new_with_offset_minute(
        year: i32,
        month: u8,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        millisecond: u32,
        offset_minute: i32,
    ) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
            offset_minute,
        }
    }

    #[inline(always)]
    pub(crate) fn to_packed(&self) -> PackedTimestamp {
        PackedTimestamp::new(
            self.year as _,
            self.month as _,
            self.day as _,
            self.hour as _,
            self.minute as _,
            self.second as _,
            self.millisecond as _,
            self.offset_minute,
        )
    }
}
