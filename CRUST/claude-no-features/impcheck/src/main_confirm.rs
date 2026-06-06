use crate::trusted_utils::{
    trusted_utils_equal_signatures, trusted_utils_log_err, trusted_utils_str_to_sig,
    SIG_SIZE_BYTES,
};

pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}

pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = None;
    let mut result_sig: Option<&str> = None;
    let mut resultint_str: Option<&str> = None;
    for arg in argv.iter() {
        if let Some(rest) = arg.strip_prefix("-formula-input=") {
            formula_input = Some(rest);
        }
        if let Some(rest) = arg.strip_prefix("-result-sig=") {
            result_sig = Some(rest);
        }
        if let Some(rest) = arg.strip_prefix("-result=") {
            resultint_str = Some(rest);
        }
    }

    let result: i32 = resultint_str.unwrap_or("0").parse().unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    let sig_str = result_sig.unwrap_or("");
    if sig_str.len() != 2 * SIG_SIZE_BYTES {
        trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    // Parse formula to get its signature
    let path_in = formula_input.unwrap_or("");
    let source = match std::fs::File::create("/dev/null") {
        Ok(f) => f,
        Err(_) => return error(),
    };
    let mut tp = crate::trusted_parser::TrustedParser::tp_init(path_in, source);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig_formula);
    if !ok {
        trusted_utils_log_err("Problem during parsing");
        return error();
    }
    let sig_formula = match sig_formula {
        Some(s) => s,
        None => return error(),
    };

    // Convert reported signature from hex string to raw data
    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    if !trusted_utils_str_to_sig(sig_str, &mut sig_res_reported) {
        trusted_utils_log_err("Invalid signature string");
        return error();
    }

    // Re-compute result signature
    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    crate::confirm::confirm_result(&sig_formula, result as u8, &mut sig_res_computed);

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
