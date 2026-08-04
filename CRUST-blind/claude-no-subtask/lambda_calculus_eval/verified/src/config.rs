use std::fs::File;
use std::io::{BufRead, BufReader};

use crate::common;
use crate::io as io_mod;

pub const CONFIG_PATH: &str = "config";

#[derive(Debug)]
pub enum reduction_order_t {
    APPLICATIVE,
    NORMAL,
}

#[derive(Debug, PartialEq, Eq)]
pub enum option_type_t {
    FILENAME,
    STEP_REDUCTION,
    REDUCTION_ORDER,
    CONFIG_ERROR,
}

pub struct Options {
    pub file: File,
    pub step_by_step_reduction: bool,
    pub reduction_order: reduction_order_t,
}

pub fn trim(str: &mut String) {
    let trimmed: String = str.trim().to_string();
    *str = trimmed;
}

pub fn get_config_type(key: &str) -> option_type_t {
    match key {
        "file" => option_type_t::FILENAME,
        "step_by_step_reduction" => option_type_t::STEP_REDUCTION,
        "reduction_order" => option_type_t::REDUCTION_ORDER,
        _ => {
            let msg = format!("ERROR: Invalid key '{}' at config file.", key);
            common::error(&msg, file!(), line!() as i32, "get_config_type");
            option_type_t::CONFIG_ERROR
        }
    }
}

pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    if !line.contains('=') {
        let msg = format!(
            "Malformed config file at line: {} . Expected = sign.\n",
            line
        );
        common::error(&msg, file!(), line!() as i32, "parse_config");
    }
    let mut parts = line.splitn(2, '=');
    let k = parts.next().unwrap_or("").to_string();
    let v = parts.next().unwrap_or("").to_string();
    *key = k;
    *value = v;
    trim(key);
    trim(value);
}

pub fn get_config_from_file() -> Options {
    let config_file = match io_mod::get_file(CONFIG_PATH, "r") {
        Ok(f) => f,
        Err(_) => {
            let msg = format!("ERROR: Could not open file {}\n", CONFIG_PATH);
            common::error(&msg, file!(), line!() as i32, "get_config_from_file");
            unreachable!();
        }
    };

    let reader = BufReader::new(config_file);

    let mut reduction_order = reduction_order_t::APPLICATIVE;
    let mut step_by_step_reduction = false;
    let mut file: Option<File> = None;

    for line_result in reader.lines() {
        let line = match line_result {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);
        let cfg = get_config_type(&key);

        match cfg {
            option_type_t::FILENAME => match io_mod::get_file(&value, "r") {
                Ok(f) => file = Some(f),
                Err(_) => {
                    let msg = format!("ERROR: Could not open file {}\n", value);
                    common::error(&msg, file!(), line!() as i32, "get_config_from_file");
                }
            },
            option_type_t::STEP_REDUCTION => {
                step_by_step_reduction = value == "true";
            }
            option_type_t::REDUCTION_ORDER => {
                if value == "applicative" {
                    reduction_order = reduction_order_t::APPLICATIVE;
                } else if value == "normal" {
                    reduction_order = reduction_order_t::NORMAL;
                } else {
                    let msg =
                        "ERROR: reduction order in cfg file should be 'normal' or 'applicative'.";
                    common::error(msg, file!(), line!() as i32, "get_config_from_file");
                }
            }
            option_type_t::CONFIG_ERROR => {
                let msg = format!("Unrecognized key: {}", key);
                common::error(&msg, file!(), line!() as i32, "get_config_from_file");
            }
        }
    }

    let f = match file {
        Some(f) => f,
        None => {
            let msg = "ERROR: File cannot be null in cfg file.\n";
            common::error(msg, file!(), line!() as i32, "get_config_from_file");
            unreachable!();
        }
    };

    Options {
        file: f,
        step_by_step_reduction,
        reduction_order,
    }
}
