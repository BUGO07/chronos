/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{format, string::String};
use x86_64::instructions::port::Port;

pub struct RtcTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u16,
    pub timezone_offset_minutes: i16,
}

impl RtcTime {
    pub fn current() -> Self {
        read_rtc()
    }
    pub fn datetime_pretty(&self) -> String {
        format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
    pub fn timezone_pretty(&self) -> String {
        format!(
            "UTC{}{}:{:02}",
            if self.timezone_offset_minutes > 0 {
                "+"
            } else {
                "-"
            },
            self.timezone_offset_minutes.abs() / 60,
            self.timezone_offset_minutes.abs() % 60
        )
    }
    pub fn with_timezone_offset(mut self, offset_minutes: i16) -> Self {
        self.timezone_offset_minutes = offset_minutes;
        self
    }
    pub fn adjusted_for_timezone(self) -> Self {
        let total_minutes =
            (self.hour as i32) * 60 + (self.minute as i32) + (self.timezone_offset_minutes as i32);

        let hour = ((total_minutes / 60) % 24 + 24) % 24;
        let minute = ((total_minutes % 60) + 60) % 60;
        let mut day = self.day;
        let month = self.month;
        let year = self.year;

        if total_minutes < 0 {
            if hour > self.hour as i32 {
                day -= 1;
            }
        } else if hour < self.hour as i32 {
            day += 1;
        }

        RtcTime {
            second: self.second,
            minute: minute as u8,
            hour: hour as u8,
            day,
            month,
            year,
            timezone_offset_minutes: self.timezone_offset_minutes,
        }
    }
}

pub fn read_rtc() -> RtcTime {
    unsafe {
        let mut address_port = Port::new(0x70);
        let mut data_port = Port::new(0x71);

        while {
            address_port.write(0x0A);
            data_port.read() & 0x80 != 0
        } {}

        let mut second = read_cmos_register(&mut address_port, &mut data_port, 0x00);
        let mut minute = read_cmos_register(&mut address_port, &mut data_port, 0x02);
        let mut hour = read_cmos_register(&mut address_port, &mut data_port, 0x04);
        let mut day = read_cmos_register(&mut address_port, &mut data_port, 0x07);
        let mut month = read_cmos_register(&mut address_port, &mut data_port, 0x08);
        let mut year = read_cmos_register(&mut address_port, &mut data_port, 0x09);
        let mut century = read_cmos_register(&mut address_port, &mut data_port, 0x32);

        address_port.write(0x0B);
        let status_b = data_port.read();

        if (status_b & 0x04) == 0 {
            second = bcd_to_binary(second);
            minute = bcd_to_binary(minute);
            hour = bcd_to_binary(hour & 0x7F) | (hour & 0x80);
            day = bcd_to_binary(day);
            month = bcd_to_binary(month);
            year = bcd_to_binary(year);
            century = bcd_to_binary(century);
        }

        let full_year = if century == 0 {
            2000 + year as u16
        } else {
            (century as u16) * 100 + (year as u16)
        };

        RtcTime {
            second,
            minute,
            hour,
            day,
            month,
            year: full_year,
            timezone_offset_minutes: 0,
        }
    }
}

fn read_cmos_register(address_port: &mut Port<u8>, data_port: &mut Port<u8>, reg: u8) -> u8 {
    unsafe {
        address_port.write(reg);
    }
    unsafe { data_port.read() }
}

fn bcd_to_binary(value: u8) -> u8 {
    (value & 0x0F) + ((value / 16) * 10)
}
