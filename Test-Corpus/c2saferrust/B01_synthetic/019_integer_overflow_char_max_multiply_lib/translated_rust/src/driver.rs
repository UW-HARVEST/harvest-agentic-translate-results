






extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const CHAR_MAX: ::core::ffi::c_int = __SCHAR_MAX__;
#[no_mangle]
pub fn printLine(line: &str) {
    println!("{}", line);
}

#[no_mangle]
pub fn printHexCharLine(char_hex: i8) {
    println!("{:02x}", char_hex as u8);
}

#[no_mangle]
pub fn bad() {
    let data: i8 = CHAR_MAX as i8;
    if data > 0 {
        let result = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn goodG2B() {
    let data: i8 = 2;
    if data > 0 {
        let result = data.wrapping_mul(2);
        printHexCharLine(result);
    }
}

fn goodB2G() {
    let data: i8 = CHAR_MAX as i8;
    if data > 0 {
        if (data as i32) < CHAR_MAX / 2 {
            let result: i8 = ((data as i32) * 2) as i8;
            printHexCharLine(result);
        } else {
            printLine("data value is too large to perform arithmetic safely.");
        }
    }
}

#[no_mangle]
pub fn good() {
    goodG2B();
    goodB2G();
}

#[no_mangle]
pub fn driver(use_good: bool) {
    if use_good {
        good();
    } else {
        bad();
    }
}

pub const __SCHAR_MAX__: ::core::ffi::c_int = 127 as ::core::ffi::c_int;
