/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub const ZONE_OFFSET: i16 = 240; // UTC+4:00

// pub struct Config {
//     pub time: TimeConfig,
// }

// pub struct TimeConfig {
//     pub zone_offset: i64,
// }

// i know i know, but its gonna be here until vfs
// pub fn get_config() -> Config {
// ! ts cause panic after reboot (sometimeis gpf sometimes pagefault)
// let config = tomling::parse(include_str!("../../res/config.toml")).unwrap_or_default();

// let empty_table = tomling::Value::Table(tomling::Table::default());

// let time_config = config
//     .get("time")
//     .unwrap_or(&empty_table)
//     .as_table()
//     .unwrap();

// let zone = time_config
//     .get("zone_offset")
//     .unwrap_or(&tomling::Value::Integer(0))
//     .as_i64()
//     .unwrap()
//     .clamp(-720, 840); // UTC-12:00-UTC+14:00

// Config {
//     time: TimeConfig { zone_offset: zone },
// }
// }
