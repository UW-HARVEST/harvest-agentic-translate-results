pub fn main(_argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = None;
    let mut fifo_feedback: Option<&str> = None;
    let mut check_model = false;
    let mut lenient = false;
    for arg in argv.iter().skip(1) {
        if let Some(rest) = arg.strip_prefix("-fifo-directives=") {
            fifo_directives = Some(rest);
        }
        if let Some(rest) = arg.strip_prefix("-fifo-feedback=") {
            fifo_feedback = Some(rest);
        }
        if arg.starts_with("-check-model") {
            check_model = true;
        }
        if arg.starts_with("-lenient") {
            lenient = true;
        }
    }
    let fin = fifo_directives.unwrap_or("");
    let fout = fifo_feedback.unwrap_or("");
    let mut checker = crate::trusted_checker::TrustedChecker::tc_init(fin, fout);
    let res = crate::trusted_checker::TrustedChecker::tc_run(check_model, lenient);
    checker.tc_end();
    res
}
