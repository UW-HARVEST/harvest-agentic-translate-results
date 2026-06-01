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
    let mut formula_input: Option<&str> = Some("");
    let mut result_sig: Option<&str> = Some("");
    let mut resultint_str: Option<&str> = Some("");
    for i in 0..argc as usize {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils_try_match_arg(arg, "-result-sig=", &mut result_sig);
        trusted_utils_try_match_arg(arg, "-result=", &mut resultint_str);
    }

    let result: i32 = resultint_str.unwrap_or("").parse().unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    let rs = result_sig.unwrap_or("");
    if rs.len() != 2 * SIG_SIZE_BYTES {
        trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    // Parse formula to get signature
    let null_file = std::fs::File::create("/dev/null").expect("open /dev/null");
    let mut tp = TrustedParser::tp_init(formula_input.unwrap_or(""), null_file);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig_formula);
    if !ok {
        trusted_utils_log_err("Problem during parsing");
        return error();
    }

    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils_str_to_sig(rs, &mut sig_res_reported);
    if !ok {
        trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    let f_sig = sig_formula.unwrap_or_else(|| vec![0u8; SIG_SIZE_BYTES]);
    confirm_result(&f_sig, result as u8, &mut sig_res_computed);

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
