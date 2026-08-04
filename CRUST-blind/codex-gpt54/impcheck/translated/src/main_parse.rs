pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let _ = argc;
    let mut formula_input = "";
    let mut fifo_parsed_formula = "";
    for arg in &argv {
        if let Some(rest) = arg.strip_prefix("-formula-input=") {
            formula_input = rest;
        }
        if let Some(rest) = arg.strip_prefix("-fifo-parsed-formula=") {
            fifo_parsed_formula = rest;
        }
    }
    let out = std::fs::File::create(fifo_parsed_formula)
        .unwrap_or_else(|_| crate::trusted_utils::trusted_utils_exit_eof());
    let mut parser = crate::trusted_parser::TrustedParser::tp_init(formula_input, out);
    let mut sig = None;
    if parser.tp_parse(&mut sig) { 0 } else { 1 }
}
