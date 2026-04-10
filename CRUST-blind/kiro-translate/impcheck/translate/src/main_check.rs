pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    use impcheck::trusted_utils;
    use impcheck::trusted_checker::TrustedChecker;

    let mut fifo_directives: Option<&str> = None;
    let mut fifo_feedback: Option<&str> = None;
    let mut check_model = false;
    let mut lenient = false;

    for i in 1..argc as usize {
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-fifo-directives=", &mut fifo_directives);
        trusted_utils::trusted_utils_try_match_arg(&argv[i], "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils::trusted_utils_try_match_flag(&argv[i], "-check-model", &mut check_model);
        trusted_utils::trusted_utils_try_match_flag(&argv[i], "-lenient", &mut lenient);
    }

    let fd = fifo_directives.unwrap_or("");
    let ff = fifo_feedback.unwrap_or("");

    let mut tc = TrustedChecker::tc_init(fd, ff);
    let res = tc.run(check_model, lenient);
    res
}
