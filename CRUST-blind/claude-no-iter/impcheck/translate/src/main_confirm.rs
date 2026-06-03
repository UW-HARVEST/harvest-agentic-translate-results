use crate::confirm::confirm_result;
use crate::trusted_parser::TrustedParser;
use crate::trusted_utils;
use std::fs::OpenOptions;

pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: Option<&str> = Some("");
    let mut result_sig: Option<&str> = Some("");
    let mut resultint_str: Option<&str> = Some("");
    for i in 0..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils::trusted_utils_try_match_arg(arg, "-formula-input=", &mut formula_input);
        trusted_utils::trusted_utils_try_match_arg(arg, "-result-sig=", &mut result_sig);
        trusted_utils::trusted_utils_try_match_arg(arg, "-result=", &mut resultint_str);
    }

    let result_str = resultint_str.unwrap_or("");
    let result: i32 = result_str.trim().parse().unwrap_or(0);
    if result != 10 && result != 20 {
        trusted_utils::trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    let rs = result_sig.unwrap_or("");
    if rs.len() != 2 * trusted_utils::SIG_SIZE_BYTES {
        trusted_utils::trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    let fi = formula_input.unwrap_or("");
    let source = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("/dev/null")
    {
        Ok(f) => f,
        Err(_) => {
            return error();
        }
    };
    let mut tp = TrustedParser::tp_init(fi, source);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig_formula);
    if !ok {
        trusted_utils::trusted_utils_log_err("Problem during parsing");
        return error();
    }
    let sig_formula = match sig_formula {
        Some(s) => s,
        None => return error(),
    };

    let mut sig_res_reported = [0u8; 16];
    let ok = trusted_utils::trusted_utils_str_to_sig(rs, &mut sig_res_reported);
    if !ok {
        trusted_utils::trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; 16];
    confirm_result(&sig_formula, result as u8, &mut sig_res_computed);

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
