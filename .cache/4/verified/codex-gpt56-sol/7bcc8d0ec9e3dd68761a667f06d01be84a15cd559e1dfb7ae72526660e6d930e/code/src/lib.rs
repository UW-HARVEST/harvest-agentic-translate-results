mod config;
mod core;

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    static mut stderr: *mut c_void;

    fn atoi(value: *const c_char) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc < 3 {
        if argv.is_null() {
            unsafe {
                atoi(argv.cast());
            }
        }
        unsafe {
            fprintf(stderr, c"usage: %s A B\n".as_ptr(), *argv);
        }
        return 2;
    }

    let a = unsafe { atoi(*argv.add(1)) };
    let b = unsafe { atoi(*argv.add(2)) };
    let r_call = core::selected_call(a, b);
    let acc = core::configured_accumulator();
    let x1 = core::helper_call(a, b);
    let x2 = core::helper_ptr(a, b);
    let x3 = core::use_generated(config::REPEAT);
    let g = unsafe { (core::G_OP)(a, b) };
    let op_name = unsafe { core::G_OP_NAME };

    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            op_name,
            r_call,
            acc,
            g,
        );
    }
    let summary = r_call
        .wrapping_add(acc)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(g);
    unsafe {
        printf(c"summary=%d\n".as_ptr(), summary);
    }
    0
}
