use crate::io as cio;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub const CONFIG_PATH: &str = "config";

#[derive(Debug, PartialEq, Eq)]
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
            eprintln!("ERROR: Invalid key '{}' at config file.", key);
            std::process::exit(1);
        }
    }
}

pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    if !line.contains('=') {
        eprintln!(
            "Malformed config file at line: {} . Expected = sign.",
            line
        );
        std::process::exit(1);
    }
    let mut parts = line.splitn(2, '=');
    let k = parts.next().unwrap_or("").to_string();
    let v = parts.next().unwrap_or("").to_string();
    *key = k;
    trim(key);
    *value = v;
    trim(value);
}

pub fn get_config_from_file() -> Options {
    let config_file = cio::get_file(CONFIG_PATH, "r")
        .expect("ERROR: Could not open config file");
    let reader = BufReader::new(config_file);

    let mut options = Options {
        file: File::open("/dev/null").unwrap_or_else(|_| {
            eprintln!("ERROR: File cannot be null in cfg file.");
            std::process::exit(1);
        }),
        step_by_step_reduction: false,
        reduction_order: reduction_order_t::APPLICATIVE,
    };
    let mut file_set = false;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => continue,
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
                options.file = cio::get_file(&value, "r")
                    .expect("ERROR: Could not open file");
                file_set = true;
            }
            option_type_t::STEP_REDUCTION => {
                options.step_by_step_reduction = value == "true";
            }
            option_type_t::REDUCTION_ORDER => {
                if value == "applicative" {
                    options.reduction_order = reduction_order_t::APPLICATIVE;
                } else if value == "normal" {
                    options.reduction_order = reduction_order_t::NORMAL;
                } else {
                    eprintln!(
                        "ERROR: reduction order in cfg file should be 'normal' or 'applicative'."
                    );
                    std::process::exit(1);
                }
            }
            option_type_t::CONFIG_ERROR => {
                eprintln!("Unrecognized key: {}", key);
                std::process::exit(1);
            }
        }
    }

    if !file_set {
        eprintln!("ERROR: File cannot be null in cfg file.");
        std::process::exit(1);
    }
    options
}
