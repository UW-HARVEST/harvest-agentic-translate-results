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
    let eq_pos = match line.find('=') {
        Some(p) => p,
        None => {
            eprintln!("Malformed config file at line: {} . Expected = sign.", line);
            std::process::exit(1);
        }
    };
    let mut k = line[..eq_pos].to_string();
    let mut v = line[eq_pos + 1..].to_string();
    trim(&mut k);
    trim(&mut v);
    *key = k;
    *value = v;
}
pub fn get_config_from_file() -> Options {
    let path = Path::new(CONFIG_PATH);
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("ERROR: Could not open config file: {}", CONFIG_PATH);
            std::process::exit(1);
        }
    };
    let reader = io::BufReader::new(file);

    let mut reduction_order = reduction_order_t::APPLICATIVE;
    let mut step_by_step_reduction = false;
    let mut file_opt: Option<File> = None;

    for line_res in reader.lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.is_empty() {
            continue;
        }
        let mut key = String::new();
        let mut value = String::new();
        parse_config(&line, &mut key, &mut value);
        let cfg = get_config_type(&key);
        match cfg {
            option_type_t::FILENAME => {
                match File::open(&value) {
                    Ok(f) => file_opt = Some(f),
                    Err(_) => {
                        eprintln!("ERROR: Could not open file {}", value);
                        std::process::exit(1);
                    }
                }
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
                    eprintln!("ERROR: reduction order in cfg file should be 'normal' or 'applicative'.");
                    std::process::exit(1);
                }
            }
            option_type_t::CONFIG_ERROR => {
                eprintln!("Unrecognized key: {}", key);
                std::process::exit(1);
            }
        }
    }

    let f = match file_opt {
        Some(f) => f,
        None => {
            eprintln!("ERROR: File cannot be null in cfg file.");
            std::process::exit(1);
        }
    };

    Options {
        file: f,
        step_by_step_reduction,
        reduction_order,
    }
}
