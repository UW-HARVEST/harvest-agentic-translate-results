#![allow(clashing_extern_declarations)]
#![allow(unused_assignments)]

// Library translation of the C source. Public extern "C" wrappers expose the
// same symbol names that the C build produces, so external callers (including
// libloading-based integration tests) can invoke them through the FFI boundary
// in exactly the same way as the C shared library.

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

const CHAR_MAX: i8 = i8::MAX;

fn print_hex_char_line_impl(char_hex: i8) {
    // C: printf("%02x\n", charHex);
    // In C, signed char is promoted to int via vararg promotion (sign-extended),
    // then %x interprets it as unsigned int.
    let promoted = char_hex as i32;
    let as_unsigned = promoted as u32;
    // Forward through libc::printf so output goes through the same FILE*
    // stdout used by the C .so when both are loaded into one process.
    unsafe {
        let fmt = b"%02x\n\0".as_ptr() as *const c_char;
        printf_u32(fmt, as_unsigned);
    }
}

fn print_line_impl(line: &CStr) {
    unsafe {
        let fmt = b"%s\n\0".as_ptr() as *const c_char;
        printf_ptr(fmt, line.as_ptr());
    }
}

// The standard `printf` is variadic. Rust stable doesn't permit declaring a
// variadic extern, but we can declare two non-variadic shims with different
// link names and let the linker resolve them all to libc's `printf`. To
// avoid the `clashing_extern_declarations` warning, declare each in its own
// module with distinct local names but the same `link_name`.
mod libc_shims {
    use std::os::raw::{c_char, c_int};
    extern "C" {
        #[link_name = "printf"]
        pub fn printf_u32(fmt: *const c_char, arg: u32) -> c_int;
    }
    extern "C" {
        #[link_name = "printf"]
        pub fn printf_ptr(fmt: *const c_char, arg: *const c_char) -> c_int;
    }
    extern "C" {
        #[link_name = "scanf"]
        pub fn scanf_int(fmt: *const c_char, arg: *mut c_int) -> c_int;
    }
}
use libc_shims::{printf_ptr, printf_u32, scanf_int};

/// Equivalent of C `printLine(const char *line)`.
#[no_mangle]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        let s = CStr::from_ptr(line);
        print_line_impl(s);
    }
}

/// Equivalent of C `printHexCharLine(char charHex)`.
#[no_mangle]
pub unsafe extern "C" fn printHexCharLine(char_hex: c_char) {
    // c_char is i8 on this platform; treat it as a signed char (i8) like C.
    print_hex_char_line_impl(char_hex as i8);
}

/// Equivalent of C `bad()`.
#[no_mangle]
pub unsafe extern "C" fn bad() {
    let data: i8 = CHAR_MAX;
    if data > 0 {
        // C: char result = data * 2;
        let result = (data as i32).wrapping_mul(2) as i8;
        print_hex_char_line_impl(result);
    }
}

unsafe fn good_g2b() {
    let data: i8 = 2;
    if data > 0 {
        let result = (data as i32).wrapping_mul(2) as i8;
        print_hex_char_line_impl(result);
    }
}

unsafe fn good_b2g() {
    let mut data: i8;
    data = b' ' as i8;
    data = CHAR_MAX;
    if data > 0 {
        if data < (CHAR_MAX / 2) {
            let result = (data as i32).wrapping_mul(2) as i8;
            print_hex_char_line_impl(result);
        } else {
            let s = b"data value is too large to perform arithmetic safely.\0";
            let cstr = CStr::from_bytes_with_nul(s).unwrap();
            print_line_impl(cstr);
        }
    }
    let _ = data;
}

/// Equivalent of C `good()`.
#[no_mangle]
pub unsafe extern "C" fn good() {
    good_g2b();
    good_b2g();
}

/// Equivalent of C `main()`.
///
/// We expose this as a free function with the symbol name `main`. When the
/// crate is built as a `cdylib`, this becomes a regular dynamic export
/// (matching the C .so). When the crate is compiled as part of a Rust test
/// binary, the test harness needs to define its own `main`, so we hide our
/// export under `cfg(not(test))`.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main() -> c_int {
    rust_main_impl()
}

#[doc(hidden)]
pub unsafe fn rust_main_impl() -> c_int {
    let mut x: c_int = 0;
    let fmt = b"%d\0".as_ptr() as *const c_char;
    scanf_int(fmt, &mut x as *mut c_int);

    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
