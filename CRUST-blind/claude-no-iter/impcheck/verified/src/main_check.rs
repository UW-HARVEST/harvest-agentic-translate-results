use crate::trusted_checker::TrustedChecker;
use crate::trusted_utils;

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = Some("");
    let mut fifo_feedback: Option<&str> = Some("");
    let mut check_model = false;
    let mut lenient = false;
    for i in 1..(argc as usize) {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        trusted_utils::trusted_utils_try_match_arg(
            arg,
            "-fifo-directives=",
            &mut fifo_directives,
        );
        trusted_utils::trusted_utils_try_match_arg(
            arg,
            "-fifo-feedback=",
            &mut fifo_feedback,
        );
        trusted_utils::trusted_utils_try_match_flag(arg, "-check-model", &mut check_model);
        trusted_utils::trusted_utils_try_match_flag(arg, "-lenient", &mut lenient);
    }

    let fd = fifo_directives.unwrap_or("");
    let ff = fifo_feedback.unwrap_or("");
    let mut tc = TrustedChecker::tc_init(fd, ff);
    let res = TrustedChecker::tc_run(check_model, lenient);
    tc.tc_end();
    use std::io::Write;
    let _ = std::io::stdout().flush();
    res
}
