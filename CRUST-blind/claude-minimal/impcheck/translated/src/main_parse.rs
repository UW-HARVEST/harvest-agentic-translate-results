use std::fs::OpenOptions;

use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut fifo_parsed_formula: Option<&str> = Some("");
    for i in 0..(argc as usize) {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(
            arg,
            "-fifo-parsed-formula=",
            &mut fifo_parsed_formula,
        );
    }
    let source = match OpenOptions::new()
        .write(true)
        .create(true)
        .open(fifo_parsed_formula.unwrap_or(""))
    {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let mut parser = TrustedParser::tp_init(formula_input.unwrap_or(""), source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = parser.tp_parse(&mut sig);
    if !ok {
        std::process::abort();
    }
    0
}
