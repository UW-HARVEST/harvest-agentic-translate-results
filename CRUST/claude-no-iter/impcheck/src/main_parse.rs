// Note: This file is not included via `lib.rs`.
#![allow(dead_code)]

use std::fs::OpenOptions;

use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = None;
    let mut fifo_parsed_formula: Option<&str> = None;
    for i in 0..argc as usize {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }
    let formula_input = formula_input.unwrap_or("");
    let fifo_parsed_formula = fifo_parsed_formula.unwrap_or("");
    let source = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(fifo_parsed_formula)
        .unwrap_or_else(|_| {
            crate::trusted_utils::trusted_utils_exit_eof();
            unreachable!()
        });
    let mut tp = TrustedParser::tp_init(formula_input, source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        std::process::abort();
    }
    0
}
