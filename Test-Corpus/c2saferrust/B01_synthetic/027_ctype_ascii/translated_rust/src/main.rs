#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]


use std::io::{self, Read};

#[allow(unused_imports)]
use ::driver;
extern "C" {
    fn __ctype_b_loc() -> *mut *const ::core::ffi::c_ushort;
    fn tolower(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn toupper(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn setlocale(
        __category: ::core::ffi::c_int,
        __locale: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn getchar() -> ::core::ffi::c_int;
}
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const _ISalnum: C2RustUnnamed = 8;
pub const _ISpunct: C2RustUnnamed = 4;
pub const _IScntrl: C2RustUnnamed = 2;
pub const _ISblank: C2RustUnnamed = 1;
pub const _ISgraph: C2RustUnnamed = 32768;
pub const _ISprint: C2RustUnnamed = 16384;
pub const _ISspace: C2RustUnnamed = 8192;
pub const _ISxdigit: C2RustUnnamed = 4096;
pub const _ISdigit: C2RustUnnamed = 2048;
pub const _ISalpha: C2RustUnnamed = 1024;
pub const _ISlower: C2RustUnnamed = 512;
pub const _ISupper: C2RustUnnamed = 256;
pub const __LC_ALL: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const LC_ALL: ::core::ffi::c_int = __LC_ALL;
#[no_mangle]
pub fn driver(c: char) {
    let is_alphanumeric = c.is_alphanumeric();
    let is_alphabetic = c.is_alphabetic();
    let is_lowercase = c.is_lowercase();
    let is_uppercase = c.is_uppercase();
    let is_digit = c.is_ascii_digit();
    let is_hexadecimal = c.is_ascii_hexdigit();
    let is_control = c.is_control();
    let is_graphical = !c.is_control() && !c.is_whitespace();
    let is_space = c.is_whitespace();
    let is_blank = matches!(c, ' ' | '\t');
    let is_printing = !c.is_control();
    let is_punctuation = c.is_ascii_punctuation();

    println!("alphanumeric: {}", if is_alphanumeric { 1 } else { 0 });
    println!("alphabetic: {}", if is_alphabetic { 1 } else { 0 });
    println!("lowercase: {}", if is_lowercase { 1 } else { 0 });
    println!("uppercase: {}", if is_uppercase { 1 } else { 0 });
    println!("digit: {}", if is_digit { 1 } else { 0 });
    println!("hexadecimal: {}", if is_hexadecimal { 1 } else { 0 });
    println!("control: {}", if is_control { 1 } else { 0 });
    println!("graphical: {}", if is_graphical { 1 } else { 0 });
    println!("space: {}", if is_space { 1 } else { 0 });
    println!("blank: {}", if is_blank { 1 } else { 0 });
    println!("printing: {}", if is_printing { 1 } else { 0 });
    println!("punctuation: {}", if is_punctuation { 1 } else { 0 });
    println!("to lower: {}", c.to_ascii_lowercase());
    println!("to upper: {}", c.to_ascii_uppercase());
}

fn main_0() -> i32 {
    let mut buf = [0u8; 1];
    let c = match io::stdin().read(&mut buf) {
        Ok(1) => buf[0] as char,
        _ => '\0',
    };
    driver(c);
    0
}

pub fn main() {
    std::process::exit(main_0());
}

