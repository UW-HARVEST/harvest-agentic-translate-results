mod config;
mod mdcore;

pub use mdcore::{G_OP, G_OP_NAME, helper_call, helper_ptr, op_add, op_mul, op_sub, use_generated};
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn atoi(value: *const c_char) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn memcpy(destination: *mut c_void, source: *const c_void, count: usize) -> *mut c_void;
    fn printf(format: *const c_char, ...) -> c_int;
    static mut stderr: *mut c_void;
}

unsafe fn argv_entry(argv: *mut *mut c_char, index: usize) -> *mut c_char {
    let mut value = std::ptr::null_mut();
    let source = argv
        .cast::<u8>()
        .wrapping_add(index.wrapping_mul(std::mem::size_of::<*mut c_char>()));

    // SAFETY: For valid argv this copies exactly one pointer-sized entry.
    // For invalid addresses, libc produces the same process-level fault as
    // the corresponding C pointer read instead of a Rust UB-check abort.
    unsafe {
        memcpy(
            (&mut value as *mut *mut c_char).cast(),
            source.cast(),
            std::mem::size_of::<*mut c_char>(),
        );
    }
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc < 3 {
        // SAFETY: This deliberately follows the C implementation exactly.
        // As in C, the caller must provide argv[0] even when argc is below 3.
        unsafe {
            fprintf(stderr, c"usage: %s A B\n".as_ptr(), argv_entry(argv, 0));
        }
        return 2;
    }

    // SAFETY: This deliberately has the same caller contract as C main:
    // argv[1] and argv[2] must point to NUL-terminated byte strings.
    let (a, b) = unsafe { (atoi(argv_entry(argv, 1)), atoi(argv_entry(argv, 2))) };

    let result_call = config::OP.apply(a, b);
    let accumulator = config::run_unrolled(config::OP, config::REPEAT);

    let x1 = mdcore::helper_call(a, b);
    let x2 = mdcore::helper_ptr(a, b);
    let x3 = mdcore::use_generated(config::REPEAT);

    // SAFETY: The exported globals are initialized to matching static values,
    // exactly like their C counterparts.
    let (global_result, operation_name) = unsafe { ((mdcore::G_OP)(a, b), mdcore::G_OP_NAME) };

    // SAFETY: The formats and operation name are NUL-terminated C strings and
    // all `%d` arguments have C `int` representation.
    unsafe {
        printf(
            c"op=%s call=%d acc=%d g.call=%d\n".as_ptr(),
            operation_name,
            result_call,
            accumulator,
            global_result,
        );
    }

    let summary = result_call
        .wrapping_add(accumulator)
        .wrapping_add(x1)
        .wrapping_add(x2)
        .wrapping_add(x3)
        .wrapping_add(global_result);

    // SAFETY: The format is NUL-terminated and `summary` is a C `int`.
    unsafe {
        printf(c"summary=%d\n".as_ptr(), summary);
    }

    0
}
