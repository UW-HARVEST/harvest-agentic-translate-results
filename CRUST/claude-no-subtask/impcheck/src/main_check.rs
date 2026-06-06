use crate::trusted_checker::{run_checker_loop, TrustedChecker};
use crate::trusted_utils::{trusted_utils_try_match_arg, trusted_utils_try_match_flag};

pub fn main(argc: i32, argv: Vec<String>) -> i32 {
    let mut fifo_directives: Option<&str> = Some("");
    let mut fifo_feedback: Option<&str> = Some("");
    let mut check_model = false;
    let mut lenient = false;

    for i in 1..argc as usize {
        if i >= argv.len() {
            break;
        }
        trusted_utils_try_match_arg(&argv[i], "-fifo-directives=", &mut fifo_directives);
        trusted_utils_try_match_arg(&argv[i], "-fifo-feedback=", &mut fifo_feedback);
        trusted_utils_try_match_flag(&argv[i], "-check-model", &mut check_model);
        trusted_utils_try_match_flag(&argv[i], "-lenient", &mut lenient);
    }

    let dir = fifo_directives.unwrap_or("");
    let fb = fifo_feedback.unwrap_or("");
    let mut tc = TrustedChecker::tc_init(dir, fb);
    // tc_run on the type takes ownership of input/output via internal state.
    // We instead drive the loop using the run_checker_loop helper.
    let res = {
        // Use unsafe cast to access the file fields by the same struct layout.
        // SAFETY: TrustedChecker is created here and only accessed by us.
        let tc_ptr = &mut tc as *mut TrustedChecker;
        unsafe {
            // The struct exposes input/output as crate-private fields; access
            // them here via direct path.
            let tc_ref = &mut *tc_ptr;
            run_checker_loop(
                tc_get_input(tc_ref),
                tc_get_output(tc_ref),
                check_model,
                lenient,
            )
        }
    };
    tc.tc_end();
    res
}

// Helper accessors that use private field access via the same crate.
fn tc_get_input(tc: &mut TrustedChecker) -> &mut std::fs::File {
    // The fields are private to the module; access through a stable
    // helper added there.
    crate::trusted_checker::tc_input_mut(tc)
}

fn tc_get_output(tc: &mut TrustedChecker) -> &mut std::fs::File {
    crate::trusted_checker::tc_output_mut(tc)
}
