use std::fs::File;

const SIG_SIZE_BYTES: usize = 16;

pub fn error() -> i32 {
    println!("s NOT VERIFIED");
    1
}

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut formula_input: String = String::new();
    let mut result_sig: String = String::new();
    let mut resultint_str: String = String::new();
    let n = argc as usize;
    for i in 0..n {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        if arg.starts_with("-formula-input=") {
            formula_input = arg["-formula-input=".len()..].to_string();
        } else if arg.starts_with("-result-sig=") {
            result_sig = arg["-result-sig=".len()..].to_string();
        } else if arg.starts_with("-result=") {
            resultint_str = arg["-result=".len()..].to_string();
        }
    }

    let result: i32 = resultint_str.parse().unwrap_or(0);
    if result != 10 && result != 20 {
        crate::trusted_utils::trusted_utils_log_err("Result code missing or invalid");
        return error();
    }
    if result_sig.len() != 2 * SIG_SIZE_BYTES {
        crate::trusted_utils::trusted_utils_log_err("Result signature missing or malformed");
        return error();
    }

    // Parse formula to get its signature
    let dev_null = match File::create("/dev/null") {
        Ok(f) => f,
        Err(_) => return error(),
    };
    let mut tp = crate::trusted_parser::TrustedParser::tp_init(&formula_input, dev_null);
    let mut sig_formula: Option<Vec<u8>> = None;
    let ok = tp.tp_parse(&mut sig_formula);
    if !ok {
        crate::trusted_utils::trusted_utils_log_err("Problem during parsing");
        return error();
    }
    let sig_formula = match sig_formula {
        Some(s) => s,
        None => return error(),
    };

    let mut sig_res_reported = [0u8; SIG_SIZE_BYTES];
    if !crate::trusted_utils::trusted_utils_str_to_sig(&result_sig, &mut sig_res_reported) {
        crate::trusted_utils::trusted_utils_log_err("Invalid signature string");
        return error();
    }

    let mut sig_res_computed = [0u8; SIG_SIZE_BYTES];
    crate::confirm::confirm_result(&sig_formula, result as u8, &mut sig_res_computed);

    if !crate::trusted_utils::trusted_utils_equal_signatures(&sig_res_computed, &sig_res_reported) {
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
