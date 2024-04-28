use core::{fmt::Write, num, ops::RangeInclusive};

use embassy_rp::i2c::{I2c, Instance, Mode};
use heapless::{String, Vec};
use sh1106::{interface::DisplayInterface, mode::GraphicsMode};

use crate::menu::MenuManager;

const MENU_RANGE: usize = 5;
const RTC_ADDR: u8 = 0x68;

pub struct DateTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

pub struct Rtc<'d, T: Instance, M: Mode> {
    i2c: I2c<'d, T, M>,
}

impl<'d, T: Instance, M: Mode> Rtc<'d, T, M> {
    pub fn new(i2c: I2c<'d, T, M>) -> Self {
        Rtc { i2c }
    }

    pub async fn set_interactive<DI>(&mut self, display: &mut GraphicsMode<DI>, display_height: u32)
    where
        DI: DisplayInterface,
    {
        let current_datetime = self.get_datetime();

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
    }

    pub fn set_datetime(&mut self, time: &DateTime) {
        // Set the time
        todo!();
    }

    pub fn get_datetime(&self) -> DateTime {
        let year = 2024;
        let month = 4;
        let day = 27;
        let hours = 12;
        let minutes = 30;
        let seconds = 45;

        DateTime {
            year,
            month,
            day,
            hours,
            minutes,
            seconds,
        }
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
