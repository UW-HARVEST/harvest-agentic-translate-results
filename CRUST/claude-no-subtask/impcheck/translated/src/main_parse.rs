use std::fs::File;

use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut fifo_parsed_formula: Option<&str> = Some("");
    for i in 0..argc as usize {
        if i >= argv.len() {
            break;
        }
        trusted_utils_try_match_arg(&argv[i], "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(
            &argv[i],
            "-fifo-parsed-formula=",
            &mut fifo_parsed_formula,
        );
    }

    let out_path = fifo_parsed_formula.unwrap_or("");
    let in_path = formula_input.unwrap_or("");
    let out = match File::create(out_path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("could not open output {}", out_path);
            return 1;
        }
    };

    let mut tp = TrustedParser::tp_init(in_path, out);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        return 1;
    }
    0
}
