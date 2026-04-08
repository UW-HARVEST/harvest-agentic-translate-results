use crate::trusted_utils::*;
use crate::trusted_parser::TrustedParser;
use crate::confirm::confirm_result;
use std::fs::File;

pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut result_sig: Option<&str> = Some("");
    let mut resultint_str: Option<&str> = Some("");

    for i in 0..argc as usize {
        trusted_utils_try_match_arg(&argv[i], "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(&argv[i], "-result-sig=", &mut result_sig);
        trusted_utils_try_match_arg(&argv[i], "-result=", &mut resultint_str);
    }

    let result_str = resultint_str.unwrap_or("");
    let result: i32 = result_str.parse().unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils_log_err("Result code missing or invalid");
        return error();
    }

    let sig_str = result_sig.unwrap_or("");
    if sig_str.len() != 2 * SIG_SIZE_BYTES {
        trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    let source = File::create("/dev/null").expect("Failed to open /dev/null");
    let mut parser = TrustedParser::tp_init(formula_input.unwrap_or(""), source);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = parser.tp_parse(&mut sig_formula);
    if !ok {
        trusted_utils_log_err("Problem during parsing");
        return error();
    }

    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils_str_to_sig(sig_str, &mut sig_res_reported);
    if !ok {
        trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    if let Some(ref sf) = sig_formula {
        confirm_result(sf, result as u8, &mut sig_res_computed);
    }

    if !trusted_utils_equal_signatures(&sig_res_computed, &sig_res_reported) {
        trusted_utils_log_err("Signature does not match!");
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
