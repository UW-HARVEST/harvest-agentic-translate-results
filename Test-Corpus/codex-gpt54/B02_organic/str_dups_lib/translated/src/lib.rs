use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

const PRINTF_FMT: &[u8] = b"%s %d\n\0";
const DUP_KEY: &[u8] = b"a\0";

#[unsafe(no_mangle)]
pub extern "C" fn str_dups(num: c_int) {
    // The original C implementation ultimately prints the duplicated key and value.
    unsafe {
        printf(
            PRINTF_FMT.as_ptr().cast::<c_char>(),
            DUP_KEY.as_ptr().cast::<c_char>(),
            num,
        );
    }
}
