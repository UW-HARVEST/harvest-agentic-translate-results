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
    // Trim ASCII whitespace from both ends, in-place.
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
        eprintln!("ERROR: Invalid key '{}' at config file.", key);
        std::process::exit(1);
    }
}

pub fn parse_config(line: &str, key: &mut String, value: &mut String) {
    let pos = match line.find('=') {
        Some(p) => p,
        None => {
            eprintln!(
                "Malformed config file at line: {} . Expected = sign.",
                line
            );
            std::process::exit(1);
        }
    };
    let mut k = line[..pos].to_string();
    let mut v = line[pos + 1..].to_string();
    trim(&mut k);
    trim(&mut v);
    key.clear();
    key.push_str(&k);
    value.clear();
    value.push_str(&v);
}

pub fn get_config_from_file() -> Options {
    let f = File::open(CONFIG_PATH).expect("Failed to open config file");
    let reader = BufReader::new(f);

    let mut file_path: Option<String> = None;
    let mut step_by_step = false;
    let mut order = reduction_order_t::APPLICATIVE;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);

        match get_config_type(&key) {
            option_type_t::FILENAME => {
                file_path = Some(value);
            }
            option_type_t::STEP_REDUCTION => {
                step_by_step = value == "true";
            }
            option_type_t::REDUCTION_ORDER => {
                if value == "applicative" {
                    order = reduction_order_t::APPLICATIVE;
                } else if value == "normal" {
                    order = reduction_order_t::NORMAL;
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

    let path = match file_path {
        Some(p) => p,
        None => {
            eprintln!("ERROR: File cannot be null in cfg file.");
            std::process::exit(1);
        }
    };
    let file = File::open(&path).expect("Failed to open file from config");
    Options {
        file,
        step_by_step_reduction: step_by_step,
        reduction_order: order,
    }
}
