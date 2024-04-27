use core::fmt::Write;

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

        let mut years: Vec<String<4>, 11> = Vec::new();
        for year in (current_datetime.year - MENU_RANGE as u16)
            ..=(current_datetime.year + MENU_RANGE as u16)
        {
            let mut year_str = String::new();
            let _ = write!(year_str, "{year}");
            let _ = years.push(year_str);
        }

        let years = years.iter().map(|s| s.as_str()).collect::<Vec<_, 11>>();

        let mut year_menu = MenuManager::new(years.as_slice(), display_height);
        year_menu.select_item(MENU_RANGE);
        let year_choice = year_menu.choose(display).await;

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
        let month_choice = month_menu.choose(display).await;
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
}
