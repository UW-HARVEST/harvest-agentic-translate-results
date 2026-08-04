use crate::common;
use crate::io as my_io;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
    let trimmed = str.trim().to_string();
    str.clear();
    str.push_str(&trimmed);
}

pub fn get_config_type(key: &str) -> option_type_t {
    if key == "file" {
        option_type_t::FILENAME
    } else if key == "step_by_step_reduction" {
        option_type_t::STEP_REDUCTION
    } else if key == "reduction_order" {
        option_type_t::REDUCTION_ORDER
    } else {
        let msg = format!("ERROR: Invalid key '{}' at config file.", key);
        common::error(&msg, file!(), line!() as i32, "get_config_type");
        option_type_t::CONFIG_ERROR
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
    let raw_key = parts.next().unwrap_or("").to_string();
    let raw_value = parts.next().unwrap_or("").to_string();

    key.clear();
    key.push_str(&raw_key);
    trim(key);

    value.clear();
    value.push_str(&raw_value);
    trim(value);
}

pub fn get_config_from_file() -> Options {
    let config_file = match my_io::get_file(CONFIG_PATH, "r") {
        Ok(f) => f,
        Err(_) => {
            common::error(
                "ERROR: Could not open config file",
                file!(),
                line!() as i32,
                "get_config_from_file",
            );
            unreachable!()
        }
    };

    let mut step_by_step_reduction = false;
    let mut reduction_order = reduction_order_t::APPLICATIVE;
    let mut maybe_file: Option<File> = None;

    let reader = BufReader::new(config_file);
    for line in reader.lines() {
        let line = match line {
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
            option_type_t::FILENAME => {
                maybe_file = Some(match my_io::get_file(&value, "r") {
                    Ok(f) => f,
                    Err(_) => {
                        let msg =
                            format!("ERROR: Could not open file {}", value);
                        common::error(
                            &msg,
                            file!(),
                            line!() as i32,
                            "get_config_from_file",
                        );
                        unreachable!()
                    }
                });
            }
            option_type_t::STEP_REDUCTION => {
                step_by_step_reduction = value == "true";
            }
            option_type_t::REDUCTION_ORDER => {
                if value == "applicative" {
                    reduction_order = reduction_order_t::APPLICATIVE;
                } else if value == "normal" {
                    reduction_order = reduction_order_t::NORMAL;
                } else {
                    common::error(
                        "ERROR: reduction order in cfg file should be 'normal' or 'applicative'.",
                        file!(),
                        line!() as i32,
                        "get_config_from_file",
                    );
                }
            }
            option_type_t::CONFIG_ERROR => {
                let msg = format!("Unrecognized key: {}", key);
                common::error(
                    &msg,
                    file!(),
                    line!() as i32,
                    "get_config_from_file",
                );
            }
        }
    }

    let file = match maybe_file {
        Some(f) => f,
        None => {
            common::error(
                "ERROR: File cannot be null in cfg file.\n",
                file!(),
                line!() as i32,
                "get_config_from_file",
            );
            unreachable!()
        }
    };

    Options {
        file,
        step_by_step_reduction,
        reduction_order,
    }
}
