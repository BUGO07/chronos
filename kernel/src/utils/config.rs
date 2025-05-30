/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
};

pub const CONFIG_PATH: &str = include_str!("../../res/kernel.cfg");

pub struct Config {
    pub timezone_offset: PropertyValue,
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl PropertyValue {
    pub fn to_str(&self) -> &str {
        match self {
            PropertyValue::String(s) => s,
            _ => "",
        }
    }
    pub fn to_unsliced_str(self) -> String {
        match self {
            PropertyValue::String(s) => s,
            _ => "".to_string(),
        }
    }
    pub fn to_int(self) -> i64 {
        match self {
            PropertyValue::Integer(i) => i,
            _ => 0,
        }
    }
    pub fn to_bool(self) -> bool {
        match self {
            PropertyValue::Boolean(b) => b,
            _ => false,
        }
    }
}

pub fn get_config() -> Config {
    let props = parse_config(CONFIG_PATH);

    let timezone_offset = props
        .get("timezone_offset")
        .cloned()
        .unwrap_or(PropertyValue::Integer(0));

    Config { timezone_offset }
}

fn parse_config(config: &str) -> BTreeMap<String, PropertyValue> {
    config
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }

            let mut parts = line.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let value = parts
                .next()?
                .trim()
                .split('#')
                .next()
                .unwrap_or("")
                .trim_end();

            let value = if let Ok(int_val) = value.parse::<i64>() {
                PropertyValue::Integer(int_val)
            } else if let Ok(bool_val) = value.parse::<bool>() {
                PropertyValue::Boolean(bool_val)
            } else {
                PropertyValue::String(value.to_string())
            };

            Some((key, value))
        })
        .collect()
}
