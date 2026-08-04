use crate::trusted_checker::TrustedChecker;
use crate::trusted_utils::{trusted_utils_try_match_arg, trusted_utils_try_match_flag};

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = Some("");
    let mut fifo_feedback: Option<&str> = Some("");
    let mut check_model = false;
    let mut lenient = false;
    for i in 1..(argc as usize) {
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(arg, "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(arg, "-check-model", &mut check_model);
        trusted_utils_try_match_flag(arg, "-lenient", &mut lenient);
    }
    // tc_init not needed separately; tc_run will create its own checker.
    let _ = fifo_directives;
    let _ = fifo_feedback;
    let res = TrustedChecker::tc_run(check_model, lenient);
    res
}
