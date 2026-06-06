use std::fs::File;

use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = None;
    let mut fifo_parsed_formula: Option<&str> = None;

    for i in 0..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }

    let formula = formula_input.unwrap_or("");
    let parsed = fifo_parsed_formula.unwrap_or("");

    let source = match File::create(parsed) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("main_parse: cannot open output");
            return 1;
        }
    };

    let mut tp = TrustedParser::tp_init(formula, source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        return 1;
    }
    0
}
