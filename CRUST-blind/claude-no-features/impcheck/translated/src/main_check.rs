use crate::trusted_checker::TrustedChecker;
use crate::trusted_utils::{trusted_utils_try_match_arg, trusted_utils_try_match_flag};

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = None;
    let mut fifo_feedback: Option<&str> = None;
    let mut check_model = false;
    let mut lenient = false;

    for i in 1..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils_try_match_arg(arg, "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(arg, "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(arg, "-check-model", &mut check_model);
        trusted_utils_try_match_flag(arg, "-lenient", &mut lenient);
    }

    let directives = fifo_directives.unwrap_or("");
    let feedback = fifo_feedback.unwrap_or("");

    let mut checker = TrustedChecker::tc_init(directives, feedback);
    let res = checker.run(check_model, lenient);
    checker.tc_end();
    res
}
