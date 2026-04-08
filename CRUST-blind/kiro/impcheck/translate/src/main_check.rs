use crate::trusted_utils::*;
use crate::trusted_checker::TrustedChecker;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = Some("");
    let mut fifo_feedback: Option<&str> = Some("");
    let mut check_model = false;
    let mut lenient = false;

    for i in 1..argc as usize {
        trusted_utils_try_match_arg(&argv[i], "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(&argv[i], "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(&argv[i], "-check-model", &mut check_model);
        trusted_utils_try_match_flag(&argv[i], "-lenient", &mut lenient);
    }

    let mut tc = TrustedChecker::tc_init(
        fifo_directives.unwrap_or(""),
        fifo_feedback.unwrap_or(""),
    );
    let res = tc.run(check_model, lenient);
    res
}
