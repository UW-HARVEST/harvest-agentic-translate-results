use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
    fn scanf(format: *const c_char, ...) -> c_int;
}

const STRING_LINE_FORMAT: &[u8] = b"%s\n\0";
const HEX_CHAR_LINE_FORMAT: &[u8] = b"%02x\n\0";
const DECIMAL_INT_FORMAT: &[u8] = b"%d\0";
const TOO_LARGE_MESSAGE: &[u8] = b"data value is too large to perform arithmetic safely.\0";

#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(STRING_LINE_FORMAT.as_ptr().cast(), line);
    }
}

#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(char_hex: i8) {
    printf(HEX_CHAR_LINE_FORMAT.as_ptr().cast(), c_int::from(char_hex));
}

#[no_mangle]
pub unsafe extern "C" fn bad() {
    let data = i8::MAX;
    if data > 0 {
        let result = (c_int::from(data) * 2) as i8;
        printHexCharLine(result);
    }
}

unsafe fn good_g2b() {
    let data = 2_i8;
    if data > 0 {
        let result = (c_int::from(data) * 2) as i8;
        printHexCharLine(result);
    }
}

unsafe fn good_b2g() {
    let data = i8::MAX;
    if data > 0 {
        if data < (i8::MAX / 2) {
            let result = (c_int::from(data) * 2) as i8;
            printHexCharLine(result);
        } else {
            printLine(TOO_LARGE_MESSAGE.as_ptr().cast());
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn good() {
    good_g2b();
    good_b2g();
}

#[export_name = "main"]
pub unsafe extern "C" fn c_main() -> c_int {
    let mut x = 0;
    scanf(DECIMAL_INT_FORMAT.as_ptr().cast(), &mut x);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
