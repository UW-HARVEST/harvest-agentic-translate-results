use crate::{common, io};
use std::fs::File;

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

fn fatal(msg: &str, file: &str, line: i32, func: &str) -> ! {
    common::error(msg, file, line, func);
    std::process::exit(1);
}

pub fn trim(str: &mut String) {
    *str = str.trim().to_string();
}

pub fn get_config_type(key: &str) -> option_type_t {
    match key {
        "file" => option_type_t::FILENAME,
        "step_by_step_reduction" => option_type_t::STEP_REDUCTION,
        "reduction_order" => option_type_t::REDUCTION_ORDER,
        _ => fatal(
            &format!("ERROR: Invalid key '{key}' at config file."),
            file!(),
            line!() as i32,
            "get_config_type",
        ),
    }
}

pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    let Some(eq_pos) = line.find('=') else {
        fatal(
            &format!("Malformed config file at line: {line} . Expected = sign.\n"),
            file!(),
            line!() as i32,
            "parse_config",
        );
    };

    key.clear();
    key.push_str(&line[..eq_pos]);
    trim(key);

    value.clear();
    value.push_str(&line[eq_pos + 1..]);
    trim(value);
}

pub fn get_config_from_file() -> Options {
    let config_contents = std::fs::read_to_string(CONFIG_PATH).unwrap_or_else(|_| {
        fatal(
            &format!("ERROR: Could not open file {CONFIG_PATH}\n"),
            file!(),
            line!() as i32,
            "get_config_from_file",
        )
    });

    let mut options = Options {
        file: io::get_file(CONFIG_PATH, "r").unwrap_or_else(|_| {
            fatal(
                &format!("ERROR: Could not open file {CONFIG_PATH}\n"),
                file!(),
                line!() as i32,
                "get_config_from_file",
            )
        }),
        step_by_step_reduction: false,
        reduction_order: reduction_order_t::APPLICATIVE,
    };

    let mut selected_file: Option<File> = None;

    for raw_line in config_contents.lines() {
        let mut key = String::new();
        let mut value = String::new();
        parse_config(raw_line, &mut key, &mut value);
        match get_config_type(&key) {
            option_type_t::FILENAME => {
                selected_file = Some(io::get_file(&value, "r").unwrap_or_else(|_| {
                    fatal(
                        &format!("ERROR: Could not open file {value}\n"),
                        file!(),
                        line!() as i32,
                        "get_config_from_file",
                    )
                }));
            }
            option_type_t::STEP_REDUCTION => {
                options.step_by_step_reduction = value == "true";
            }
            option_type_t::REDUCTION_ORDER => {
                options.reduction_order = match value.as_str() {
                    "applicative" => reduction_order_t::APPLICATIVE,
                    "normal" => reduction_order_t::NORMAL,
                    _ => fatal(
                        "ERROR: reduction order in cfg file should be 'normal' or 'applicative'.",
                        file!(),
                        line!() as i32,
                        "get_config_from_file",
                    ),
                };
            }
            option_type_t::CONFIG_ERROR => {
                fatal(
                    &format!("Unrecognized key: {key}"),
                    file!(),
                    line!() as i32,
                    "get_config_from_file",
                );
            }
        }
    }

    options.file = selected_file.unwrap_or_else(|| {
        fatal(
            "ERROR: File cannot be null in cfg file.\n",
            file!(),
            line!() as i32,
            "get_config_from_file",
        )
    });
    options
}
