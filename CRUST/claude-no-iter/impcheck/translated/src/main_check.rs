// Note: This file is not included via `lib.rs`.
#![allow(dead_code)]

use crate::trusted_checker::TrustedChecker;
use crate::trusted_utils::{trusted_utils_try_match_arg, trusted_utils_try_match_flag};

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = None;
    let mut fifo_feedback: Option<&str> = None;
    let mut check_model = false;
    let mut lenient = false;
    for i in 1..argc as usize {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(arg, "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(arg, "-check-model", &mut check_model);
        trusted_utils_try_match_flag(arg, "-lenient", &mut lenient);
    }
    let fifo_in = fifo_directives.unwrap_or("");
    let fifo_out = fifo_feedback.unwrap_or("");
    std::env::set_var("IMPCHECK_FIFO_IN", fifo_in);
    std::env::set_var("IMPCHECK_FIFO_OUT", fifo_out);
    TrustedChecker::tc_run(check_model, lenient)
}
