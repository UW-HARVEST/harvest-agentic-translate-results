use std::ffi::{c_char, c_int, c_long};

static mut SUM: c_int = 0;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn puts(string: *const c_char) -> c_int;
    fn strtol(nptr: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long;
}

#[no_mangle]
pub unsafe extern "C" fn static_sum(update: c_int) -> c_int {
    unsafe {
        SUM = SUM.wrapping_add(update);
        SUM
    }
}

#[export_name = "main"]
pub unsafe extern "C" fn c_main(argc: c_int, argv: *mut *mut c_char) -> c_int {
    if argc != 2 {
        unsafe {
            puts(
                b"Error: should only be a single (integer) argument!\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return 1;
    }

    let argument = unsafe { argv.wrapping_add(1).read() };
    let mut end = std::ptr::null_mut();
    let stride = unsafe { strtol(argument, &mut end, 10) } as c_int;
    if end == argument {
        unsafe {
            puts(
                b"Error: first argument must be an integer!\0"
                    .as_ptr()
                    .cast(),
            );
        }
        return 1;
    }

    for i in 0_i32..10 {
        let value = unsafe { static_sum(i.wrapping_mul(stride)) };
        unsafe {
            printf(b"%d\n\0".as_ptr().cast(), value);
        }
    }

    0
}
