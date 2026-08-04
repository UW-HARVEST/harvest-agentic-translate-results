pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}
pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let _ = argc;
    let mut formula_input = "";
    let mut result_sig = "";
    let mut result_str = "";
    for arg in &argv {
        if let Some(rest) = arg.strip_prefix("-formula-input=") {
            formula_input = rest;
        }
        if let Some(rest) = arg.strip_prefix("-result-sig=") {
            result_sig = rest;
        }
        if let Some(rest) = arg.strip_prefix("-result=") {
            result_str = rest;
        }
    }

    let result: i32 = result_str.parse().unwrap_or_default();
    if result != 10 && result != 20 {
        crate::trusted_utils::trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    if result_sig.len() != crate::trusted_utils::SIG_SIZE_BYTES * 2 {
        crate::trusted_utils::trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    let sink = std::fs::File::create("/dev/null")
        .unwrap_or_else(|_| crate::trusted_utils::trusted_utils_exit_eof());
    let mut parser = crate::trusted_parser::TrustedParser::tp_init(formula_input, sink);
    let mut sig_formula = None;
    if !parser.tp_parse(&mut sig_formula) {
        crate::trusted_utils::trusted_utils_log_err("Problem during parsing");
        return error();
    }
    let sig_formula = sig_formula.unwrap_or_default();

    let mut reported = [0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
    if !crate::trusted_utils::trusted_utils_str_to_sig(result_sig, &mut reported) {
        crate::trusted_utils::trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut computed = [0_u8; crate::trusted_utils::SIG_SIZE_BYTES];
    crate::confirm::confirm_result(&sig_formula, result as u8, &mut computed);
    if !crate::trusted_utils::trusted_utils_equal_signatures(&computed, &reported) {
        crate::trusted_utils::trusted_utils_log_err("Signature does not match!");
        return error();
    }

    if result == 10 {
        println!("s VERIFIED SATISFIABLE");
    }
    if result == 20 {
        println!("s VERIFIED UNSATISFIABLE");
    }
    0
}
