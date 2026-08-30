use std::ffi::{c_char, c_int, c_void};

use crate::config::{OP, REPEAT, run_unrolled};
use crate::mdcore::{G_OP, G_OP_NAME, helper_call, helper_ptr, use_generated};

unsafe extern "C" {
    fn atoi(value: *const c_char) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;

    static mut stderr: *mut c_void;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc < 3 {
        unsafe {
            fprintf(stderr, c"usage: %s A B\n".as_ptr(), *argv);
        }
        return 2;
    }

    let (a, b) = unsafe { (atoi(*argv.add(1)), atoi(*argv.add(2))) };

    let result = OP.apply(a, b);
    let accumulator = run_unrolled(OP, REPEAT);

    let x1 = helper_call(a, b);
    let x2 = helper_ptr(a, b);
    let x3 = use_generated(REPEAT);
    let global_result = unsafe { G_OP(a, b) };

    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            G_OP_NAME,
            result,
            accumulator,
            global_result,
        );
        printf(
            c"summary=%d\n".as_ptr(),
            result
                .wrapping_add(accumulator)
                .wrapping_add(x1)
                .wrapping_add(x2)
                .wrapping_add(x3)
                .wrapping_add(global_result),
        );
    }
    0
}
