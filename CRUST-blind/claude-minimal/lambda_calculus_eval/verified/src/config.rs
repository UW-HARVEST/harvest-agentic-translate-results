use std::fs::File;
use std::io::{BufRead, BufReader};
use crate::{common, io};

pub const CONFIG_PATH: &str = "config";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
            "Malformed config file at line: {} . Expected = sign.",
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
    let config_file = match io::get_file(CONFIG_PATH, "r") {
        Ok(f) => f,
        Err(_) => {
            common::error(
                "Could not open config file",
                file!(),
                line!() as i32,
                "get_config_from_file",
            );
            unreachable!();
        }
    };

    let mut reduction_order = reduction_order_t::APPLICATIVE;
    let mut step_by_step_reduction = false;
    let mut file: Option<File> = None;

    let reader = BufReader::new(config_file);
    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);
        let cfg = get_config_type(&key);

        match cfg {
            option_type_t::FILENAME => match io::get_file(&value, "r") {
                Ok(f) => file = Some(f),
                Err(_) => {
                    let msg = format!("Could not open file: {}", value);
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
                common::error(&msg, file!(), line!() as i32, "get_config_from_file");
            }
        }
    }

    let file = match file {
        Some(f) => f,
        None => {
            common::error(
                "ERROR: File cannot be null in cfg file.",
                file!(),
                line!() as i32,
                "get_config_from_file",
            );
            unreachable!();
        }
    };

    Options {
        file,
        step_by_step_reduction,
        reduction_order,
    }
}
