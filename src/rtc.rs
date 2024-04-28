use core::{fmt::Write, ops::RangeInclusive};

use ds323x::{DateTimeAccess, Ds323x, NaiveDate};
use embassy_rp::i2c::{I2c, Instance, Mode};
use heapless::{String, Vec};
use rtcc::{Datelike, Timelike};
use sh1106::{interface::DisplayInterface, mode::GraphicsMode};

use crate::menu::MenuManager;

const MENU_RANGE: usize = 5;

pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

pub struct Rtc<'d, T: Instance, M: Mode> {
    rtc: Ds323x<ds323x::interface::I2cInterface<I2c<'d, T, M>>, ds323x::ic::DS3231>,
}

impl<'d, T: Instance, M: Mode> Rtc<'d, T, M> {
    pub fn new(i2c: I2c<'d, T, M>) -> Self {
        let rtc = Ds323x::new_ds3231(i2c);

        Rtc { rtc }
    }

    pub async fn set_interactive<DI>(&mut self, display: &mut GraphicsMode<DI>, display_height: u32)
    where
        DI: DisplayInterface,
    {
        let Ok(current_datetime) = self.datetime() else {
            return;
        };

        // set the year
        let Some(year_choice) = self
            .choose_from_range(
                (current_datetime.year as usize - MENU_RANGE)
                    ..=(current_datetime.year as usize + MENU_RANGE),
                MENU_RANGE,
                display,
                display_height,
            )
            .await
        else {
            return;
        };

        let year = current_datetime.year as usize - MENU_RANGE + year_choice;

        // set the month
        let months = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];

        let mut month_menu = MenuManager::new(&months, display_height);
        month_menu.select_item(current_datetime.month as usize - 1);
        let Some(month_choice) = month_menu.choose(display).await else {
            return;
        };

        let month = month_choice + 1;

        // set the day
        let num_days = match month {
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let Some(day_choice) = self
            .choose_from_range(
                1..=num_days,
                current_datetime.day as usize - 1,
                display,
                display_height,
            )
            .await
        else {
            return;
        };

        let day = day_choice + 1;

        // set the hours
        let Some(hours) = self
            .choose_from_range(
                0..=23,
                current_datetime.hours as usize,
                display,
                display_height,
            )
            .await
        else {
            return;
        };

        // set the minutes
        let Some(minutes) = self
            .choose_from_range(
                0..=59,
                current_datetime.minutes as usize,
                display,
                display_height,
            )
            .await
        else {
            return;
        };

        // set the seconds
        let Some(seconds) = self
            .choose_from_range(
                0..=59,
                current_datetime.seconds as usize,
                display,
                display_height,
            )
            .await
        else {
            return;
        };

        let new_datetime = DateTime {
            year: year as u16,
            month: month as u8,
            day: day as u8,
            hours: hours as u8,
            minutes: minutes as u8,
            seconds: seconds as u8,
        };

        let _ = self.set_datetime(&new_datetime);
    }

    pub fn set_datetime(&mut self, datetime: &DateTime) -> Result<(), ()> {
        let datetime = NaiveDate::from_ymd_opt(
            datetime.year as i32,
            datetime.month as u32,
            datetime.day as u32,
        )
        .ok_or(())?
        .and_hms_opt(
            datetime.hours as u32,
            datetime.minutes as u32,
            datetime.seconds as u32,
        )
        .ok_or(())?;

        self.rtc.set_datetime(&datetime).map_err(|_| ())
    }

    pub fn datetime(&mut self) -> Result<DateTime, ()> {
        self.rtc
            .datetime()
            .map(|datetime| {
                let year = datetime.year() as u16;
                let month = datetime.month() as u8;
                let day = datetime.day() as u8;
                let hours = datetime.hour() as u8;
                let minutes = datetime.minute() as u8;
                let seconds = datetime.second() as u8;

                DateTime {
                    year,
                    month,
                    day,
                    hours,
                    minutes,
                    seconds,
                }
            })
            .map_err(|_| ())
    }

    async fn choose_from_range<DI>(
        &mut self,
        range: RangeInclusive<usize>,
        default: usize,
        display: &mut GraphicsMode<DI>,
        display_height: u32,
    ) -> Option<usize>
    where
        DI: DisplayInterface,
    {
        let mut items: Vec<String<4>, 64> = Vec::new();
        for item in range {
            let mut item_str = String::new();
            let _ = write!(item_str, "{item}");
            let _ = items.push(item_str);
        }

        let items = items.iter().map(|s| s.as_str()).collect::<Vec<_, 64>>();

        // TODO: use custom logic instead of MenuManager
        let mut menu = MenuManager::new(items.as_slice(), display_height);
        menu.select_item(default);
        menu.choose(display).await
    }
}
