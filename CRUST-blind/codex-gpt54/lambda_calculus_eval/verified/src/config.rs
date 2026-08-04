use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use crate::{common, io as file_io};
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
    match key {
        "file" => option_type_t::FILENAME,
        "step_by_step_reduction" => option_type_t::STEP_REDUCTION,
        "reduction_order" => option_type_t::REDUCTION_ORDER,
        _ => {
            let error_msg = common::format("", format_args!("ERROR: Invalid key '{}' at config file.", key));
            common::error(&error_msg, file!(), line!() as i32, "get_config_type");
            option_type_t::CONFIG_ERROR
        }
    }
}
pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    let Some((raw_key, raw_value)) = line.split_once('=') else {
        let error_msg = common::format(
            "",
            format_args!("Malformed config file at line: {} . Expected = sign.\n", line),
        );
        common::error(&error_msg, file!(), line!() as i32, "parse_config");
        return;
    };
    *key = raw_key.to_string();
    *value = raw_value.to_string();
    trim(key);
    trim(value);
}
pub fn get_config_from_file() -> Options {
    let config_path = Path::new(CONFIG_PATH);
    let config_file = file_io::get_file(config_path.to_str().unwrap_or(CONFIG_PATH), "r")
        .unwrap_or_else(|_| {
            let error_msg = common::format("", format_args!("ERROR: Could not open file {}\n", CONFIG_PATH));
            common::error(&error_msg, file!(), line!() as i32, "get_config_from_file");
            unreachable!()
        });

    let mut reduction_order = reduction_order_t::APPLICATIVE;
    let mut step_by_step_reduction = false;
    let mut opened_file: Option<File> = None;

    for line in io::BufReader::new(config_file).lines() {
        let line = line.unwrap_or_else(|_| {
            common::error("ERROR: failed to read config file.", file!(), line!() as i32, "get_config_from_file");
            unreachable!()
        });
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);
        match get_config_type(&key) {
            option_type_t::FILENAME => {
                opened_file = Some(file_io::get_file(&value, "r").unwrap_or_else(|_| {
                    let error_msg = common::format("", format_args!("ERROR: Could not open file {}\n", value));
                    common::error(&error_msg, file!(), line!() as i32, "get_config_from_file");
                    unreachable!()
                }));
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
                let error_msg = common::format("", format_args!("Unrecognized key: {}", key));
                common::error(&error_msg, file!(), line!() as i32, "get_config_from_file");
            }
        }
    }

    let file = opened_file.unwrap_or_else(|| {
        common::error(
            "ERROR: File cannot be null in cfg file.\n",
            file!(),
            line!() as i32,
            "get_config_from_file",
        );
        unreachable!()
    });

    Options {
        file,
        step_by_step_reduction,
        reduction_order,
    }
}
