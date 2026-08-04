use std::fs::File;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: String = String::new();
    let mut fifo_parsed_formula: String = String::new();
    let n = argc as usize;
    for i in 0..n {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        if arg.starts_with("-formula-input=") {
            formula_input = arg["-formula-input=".len()..].to_string();
        } else if arg.starts_with("-fifo-parsed-formula=") {
            fifo_parsed_formula = arg["-fifo-parsed-formula=".len()..].to_string();
        }
    }

    let source = match File::create(&fifo_parsed_formula) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let mut tp = crate::trusted_parser::TrustedParser::tp_init(&formula_input, source);
    let mut sig: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig);
    if !ok {
        return 1;
    }
    0
}
