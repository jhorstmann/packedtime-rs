use crate::PackedTimestamp;

/// Internal struct containing the components of a DateTime as separate fields.
#[derive(PartialEq, Clone, Debug, Default)]
pub(crate) struct DateTimeComponents {
    pub year: u16,
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
    pub(crate) fn new(
        year: u16,
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
        year: u16,
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
