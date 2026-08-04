use crate::trusted_parser::TrustedParser;
use crate::trusted_utils;
use std::fs::OpenOptions;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut fifo_parsed_formula: Option<&str> = Some("");
    for i in 0..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils::trusted_utils_try_match_arg(
            arg,
            "-formula-input=",
            &mut formula_input,
        );
        trusted_utils::trusted_utils_try_match_arg(
            arg,
            "-fifo-parsed-formula=",
            &mut fifo_parsed_formula,
        );
    }

    let fi = formula_input.unwrap_or("");
    let fp = fifo_parsed_formula.unwrap_or("");
    let source = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(fp)
    {
        Ok(f) => f,
        Err(_) => {
            std::process::exit(1);
        }
    };
    let mut tp = TrustedParser::tp_init(fi, source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        std::process::exit(1);
    }
    0
}
