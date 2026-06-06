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

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = None;
    let mut result_sig_str: Option<&str> = None;
    let mut resultint_str: Option<&str> = None;

    for i in 0..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-result-sig=", &mut result_sig_str);
        trusted_utils_try_match_arg(arg, "-result=", &mut resultint_str);
    }

    let result: i32 = resultint_str.and_then(|s| s.parse().ok()).unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils_log_err("Result code missing or invalid");
        return error();
    }

    let result_sig = result_sig_str.unwrap_or("");
    if result_sig.len() != 2 * SIG_SIZE_BYTES {
        trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    // Parse formula to get its signature
    let source = match File::create("/dev/null") {
        Ok(f) => f,
        Err(_) => {
            trusted_utils_log_err("cannot open /dev/null");
            return error();
        }
    };
    let mut tp = TrustedParser::tp_init(formula_input.unwrap_or(""), source);
    let mut sig_formula_opt: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig_formula_opt);
    if !ok {
        trusted_utils_log_err("Problem during parsing");
        return error();
    }
    let sig_formula = sig_formula_opt.unwrap_or_else(|| vec![0u8; SIG_SIZE_BYTES]);

    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    if !trusted_utils_str_to_sig(result_sig, &mut sig_res_reported) {
        trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    confirm_result(&sig_formula, result as u8, &mut sig_res_computed);

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
