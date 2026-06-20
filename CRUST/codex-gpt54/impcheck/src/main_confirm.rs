use std::fs::File;

use crate::confirm::confirm_result;
use crate::trusted_parser::TrustedParser;
use crate::trusted_utils::{
    trusted_utils_equal_signatures, trusted_utils_log_err, trusted_utils_str_to_sig,
    trusted_utils_try_match_arg, SIG_SIZE_BYTES,
};

pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}

pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input = None;
    let mut result_sig = None;
    let mut result_str = None;
    for arg in &argv {
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-result-sig=", &mut result_sig);
        trusted_utils_try_match_arg(arg, "-result=", &mut result_str);
    }

    let result = result_str.unwrap_or("").parse::<i32>().unwrap_or_default();
    if result != 10 && result != 20 {
        trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    if result_sig.unwrap_or("").len() != 2 * SIG_SIZE_BYTES {
        trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    let source = File::create("/dev/null").unwrap();
    let mut parser = TrustedParser::tp_init(formula_input.unwrap_or(""), source);
    let mut sig_formula = None;
    if !parser.tp_parse(&mut sig_formula) {
        trusted_utils_log_err("Problem during parsing");
        return error();
    }

    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    if !trusted_utils_str_to_sig(result_sig.unwrap_or(""), &mut sig_res_reported) {
        trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    confirm_result(sig_formula.as_ref().unwrap(), result as u8, &mut sig_res_computed);
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
