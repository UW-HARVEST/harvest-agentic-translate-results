pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: String = String::new();
    let mut fifo_feedback: String = String::new();
    let mut check_model = false;
    let mut lenient = false;
    let n = argc as usize;
    for i in 1..n {
        if i >= argv.len() {
            break;
        }
        let arg = &argv[i];
        if arg.starts_with("-fifo-directives=") {
            fifo_directives = arg["-fifo-directives=".len()..].to_string();
        } else if arg.starts_with("-fifo-feedback=") {
            fifo_feedback = arg["-fifo-feedback=".len()..].to_string();
        } else if arg.starts_with("-check-model") {
            check_model = true;
        } else if arg.starts_with("-lenient") {
            lenient = true;
        }
    }
    let _ = (fifo_directives, fifo_feedback);
    crate::trusted_checker::TrustedChecker::tc_run(check_model, lenient)
}
