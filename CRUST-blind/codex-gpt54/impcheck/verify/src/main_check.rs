pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let _ = argc;
    let mut fifo_directives = "";
    let mut fifo_feedback = "";
    let mut check_model = false;
    let mut lenient = false;
    for arg in argv.iter().skip(1) {
        if let Some(rest) = arg.strip_prefix("-fifo-directives=") {
            fifo_directives = rest;
        }
        if let Some(rest) = arg.strip_prefix("-fifo-feedback=") {
            fifo_feedback = rest;
        }
        if arg.starts_with("-check-model") {
            check_model = true;
        }
        if arg.starts_with("-lenient") {
            lenient = true;
        }
    }
    let mut checker = crate::trusted_checker::TrustedChecker::tc_init(fifo_directives, fifo_feedback);
    let result = crate::trusted_checker::TrustedChecker::tc_run(check_model, lenient);
    checker.tc_end();
    result
}
