use core::time::Duration;

pub fn unix_to_date(datetime: Duration) -> (u32, u32, u32, u32, u32, u32) {
    let secs = datetime.as_secs() as u32;

    let mut remaining = secs;

    const SECS_PER_MIN: u32 = 60;
    const SECS_PER_HOUR: u32 = 60 * SECS_PER_MIN;
    const SECS_PER_DAY: u32 = 24 * SECS_PER_HOUR;

    let days = remaining / SECS_PER_DAY;
    remaining %= SECS_PER_DAY;

    let hour = remaining / SECS_PER_HOUR;
    remaining %= SECS_PER_HOUR;
    let minute = remaining / SECS_PER_MIN;
    let second = remaining % SECS_PER_MIN;

    let (year, month, day) = days_to_ymd(days);

    (year, month, day, hour, minute, second)
}

fn days_to_ymd(mut days: u32) -> (u32, u32, u32) {
    let mut year = 1970u32;

    loop {
        let is_leap = is_leap_year(year);
        let days_in_year = if is_leap { 366 } else { 365 };

        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let months = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 0;
    while days >= months[month] {
        days -= months[month];
        month += 1;
    }

    (year, (month + 1) as u32, (days + 1) as u32)
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
