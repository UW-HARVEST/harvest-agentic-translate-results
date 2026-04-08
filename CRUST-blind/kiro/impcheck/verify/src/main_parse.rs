use crate::trusted_utils::*;
use crate::trusted_parser::TrustedParser;
use std::fs::File;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut fifo_parsed_formula: Option<&str> = Some("");

    for i in 0..argc as usize {
        trusted_utils_try_match_arg(&argv[i], "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(&argv[i], "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }

    let source = File::create(fifo_parsed_formula.unwrap_or("")).expect("Failed to open output");
    let mut parser = TrustedParser::tp_init(formula_input.unwrap_or(""), source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = parser.tp_parse(&mut sig);
    if !ok {
        std::process::abort();
    }
    0
}
