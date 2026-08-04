use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::trusted_utils_try_match_arg;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut fifo_parsed_formula: Option<&str> = Some("");
    for i in 0..argc as usize {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }
    let out = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(fifo_parsed_formula.unwrap_or(""))
        .expect("main_parse: failed to open output");
    let mut tp = TrustedParser::tp_init(formula_input.unwrap_or(""), out);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        return 1;
    }
    0
}
