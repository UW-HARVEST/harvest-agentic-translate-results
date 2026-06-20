use std::fs::File;

use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input = None;
    let mut fifo_parsed_formula = None;
    for arg in &argv {
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }

    let out = File::create(fifo_parsed_formula.unwrap_or("")).unwrap();
    let mut parser = TrustedParser::tp_init(formula_input.unwrap_or(""), out);
    let mut sig = None;
    if !parser.tp_parse(&mut sig) {
        std::process::abort();
    }
    0
}
