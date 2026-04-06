use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
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
    *str = trimmed;
}
pub fn get_config_type(key: &str) -> option_type_t {
    match key {
        "file" => option_type_t::FILENAME,
        "step_by_step_reduction" => option_type_t::STEP_REDUCTION,
        "reduction_order" => option_type_t::REDUCTION_ORDER,
        _ => {
            crate::common::error(
                &format!("ERROR: Invalid key '{}' at config file.", key),
                file!(), line!() as i32, "get_config_type",
            );
            option_type_t::CONFIG_ERROR
        }
    }
}
pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    let eq_pos = line.find('=');
    if eq_pos.is_none() {
        crate::common::error(
            &format!("Malformed config file at line: {} . Expected = sign.", line),
            file!(), line!() as i32, "parse_config",
        );
    }
    let mut parts = line.splitn(2, '=');
    *key = parts.next().unwrap().trim().to_string();
    *value = parts.next().unwrap().trim().to_string();
}
pub fn get_config_from_file() -> Options {
    let config_file = File::open(CONFIG_PATH).unwrap_or_else(|_| {
        crate::common::error(
            &format!("ERROR: Could not open file {}", CONFIG_PATH),
            file!(), line!() as i32, "get_config_from_file",
        );
        unreachable!()
    });
    let reader = io::BufReader::new(config_file);
    let mut options_file: Option<File> = None;
    let mut step_by_step_reduction = false;
    let mut reduction_order = reduction_order_t::APPLICATIVE;

    for line in reader.lines() {
        let line = line.unwrap();
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);
        let cfg = get_config_type(&key);
        match cfg {
            option_type_t::FILENAME => {
                options_file = Some(File::open(&value).unwrap_or_else(|_| {
                    crate::common::error(
                        &format!("ERROR: Could not open file {}", value),
                        file!(), line!() as i32, "get_config_from_file",
                    );
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
                    crate::common::error(
                        "ERROR: reduction order in cfg file should be 'normal' or 'applicative'.",
                        file!(), line!() as i32, "get_config_from_file",
                    );
                }
            }
            option_type_t::CONFIG_ERROR => {
                crate::common::error(
                    &format!("Unrecognized key: {}", key),
                    file!(), line!() as i32, "get_config_from_file",
                );
            }
        }
    }
    if options_file.is_none() {
        crate::common::error(
            "ERROR: File cannot be null in cfg file.",
            file!(), line!() as i32, "get_config_from_file",
        );
    }
    Options {
        file: options_file.unwrap(),
        step_by_step_reduction,
        reduction_order,
    }
}
