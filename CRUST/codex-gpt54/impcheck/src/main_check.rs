use crate::trusted_checker::TrustedChecker;
use crate::trusted_utils::{trusted_utils_try_match_arg, trusted_utils_try_match_flag};

pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives = None;
    let mut fifo_feedback = None;
    let mut check_model = false;
    let mut lenient = false;

    for arg in &argv {
        trusted_utils_try_match_arg(arg, "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(arg, "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(arg, "-check-model", &mut check_model);
        trusted_utils_try_match_flag(arg, "-lenient", &mut lenient);
    }

    let _checker = TrustedChecker::tc_init(
        fifo_directives.unwrap_or(""),
        fifo_feedback.unwrap_or(""),
    );
    TrustedChecker::tc_run(check_model, lenient)
}
