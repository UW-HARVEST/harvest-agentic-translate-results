use std::fs::File;

pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = None;
    let mut fifo_parsed_formula: Option<&str> = None;
    for arg in argv.iter() {
        if let Some(rest) = arg.strip_prefix("-formula-input=") {
            formula_input = Some(rest);
        }
        if let Some(rest) = arg.strip_prefix("-fifo-parsed-formula=") {
            fifo_parsed_formula = Some(rest);
        }
    }
    let path_in = formula_input.unwrap_or("");
    let path_out = fifo_parsed_formula.unwrap_or("");
    let source = match File::create(path_out) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let mut tp = crate::trusted_parser::TrustedParser::tp_init(path_in, source);
    let mut sig: Option<Vec<u8>> = None;
    if !tp.tp_parse(&mut sig) {
        return 1;
    }
    0
}
