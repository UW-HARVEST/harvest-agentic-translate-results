pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    use impcheck::trusted_utils;
    use impcheck::trusted_parser::TrustedParser;
    use std::fs::File;

    let mut formula_input: Option<&str> = None;
    let mut fifo_parsed_formula: Option<&str> = None;

    for i in 0..argc as usize {
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-formula-input=", &mut formula_input);
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-fifo-parsed-formula=", &mut fifo_parsed_formula);
    }

    let fi = formula_input.unwrap_or("");
    let fpf = fifo_parsed_formula.unwrap_or("");

    let source = File::create(fpf).unwrap_or_else(|_| {
        std::process::abort();
    });
    let mut parser = TrustedParser::tp_init(fi, source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = parser.tp_parse(&mut sig);
    if !ok {
        std::process::abort();
    }
    0
}
