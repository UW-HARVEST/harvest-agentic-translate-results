pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}
pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    use impcheck::trusted_utils::{self, SIG_SIZE_BYTES};
    use impcheck::trusted_parser::TrustedParser;
    use impcheck::confirm::confirm_result;
    use std::fs::File;

    let mut formula_input: Option<&str> = None;
    let mut result_sig: Option<&str> = None;
    let mut resultint_str: Option<&str> = None;

    for i in 0..argc as usize {
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-formula-input=", &mut formula_input);
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-result-sig=", &mut result_sig);
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-result=", &mut resultint_str);
    }

    let result_str = resultint_str.unwrap_or("");
    let result: i32 = result_str.parse().unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils::trusted_utils_log_err("Result code missing or invalid");
        return error();
    }

    let sig_str = result_sig.unwrap_or("");
    if sig_str.len() != 2 * SIG_SIZE_BYTES {
        trusted_utils::trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    // Parse formula to get its signature
    let source = File::create("/dev/null").unwrap();
    let fi = formula_input.unwrap_or("");
    let mut parser = TrustedParser::tp_init(fi, source);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = parser.tp_parse(&mut sig_formula);
    if !ok {
        trusted_utils::trusted_utils_log_err("Problem during parsing");
        return error();
    }

    // Convert reported signature from hex string to raw data
    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    let ok = trusted_utils::trusted_utils_str_to_sig(sig_str, &mut sig_res_reported);
    if !ok {
        trusted_utils::trusted_utils_log_err("Invalid signature string");
        return error();
    }

    // Re-compute result signature
    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    let formula_sig = sig_formula.unwrap();
    confirm_result(&formula_sig, result as u8, &mut sig_res_computed);

    // Check reported signature against computed signature
    if !trusted_utils::trusted_utils_equal_signatures(&sig_res_computed, &sig_res_reported) {
        trusted_utils::trusted_utils_log_err("Signature does not match!");
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
