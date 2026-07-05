use std::ffi::c_char;

extern "C" {
    fn __ctype_b_loc() -> *mut *const ::core::ffi::c_ushort;
    fn tolower(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn toupper(__c: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn setlocale(
        __category: ::core::ffi::c_int,
        __locale: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub const _ISpunct: C2RustUnnamed = 4;
pub const _ISprint: C2RustUnnamed = 16384;
pub const _ISblank: C2RustUnnamed = 1;
pub const _ISspace: C2RustUnnamed = 8192;
pub const _ISgraph: C2RustUnnamed = 32768;
pub const _IScntrl: C2RustUnnamed = 2;
pub const _ISxdigit: C2RustUnnamed = 4096;
pub const _ISdigit: C2RustUnnamed = 2048;
pub const _ISupper: C2RustUnnamed = 256;
pub const _ISlower: C2RustUnnamed = 512;
pub const _ISalpha: C2RustUnnamed = 1024;
pub const _ISalnum: C2RustUnnamed = 8;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const __LC_ALL: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const LC_ALL: ::core::ffi::c_int = __LC_ALL;
#[no_mangle]
pub fn driver(c: c_char) {
    let ch = (c as u8) as char;

    println!("alphanumeric: {}", ch.is_alphanumeric() as i32);
    println!("alphabetic: {}", ch.is_alphabetic() as i32);
    println!("lowercase: {}", ch.is_lowercase() as i32);
    println!("uppercase: {}", ch.is_uppercase() as i32);
    println!("digit: {}", ch.is_ascii_digit() as i32);
    println!("hexadecimal: {}", ch.is_ascii_hexdigit() as i32);
    println!("control: {}", ch.is_control() as i32);
    println!("graphical: {}", (!ch.is_control() && !ch.is_whitespace()) as i32);
    println!("space: {}", ch.is_whitespace() as i32);
    println!("blank: {}", matches!(ch, ' ' | '\t') as i32);
    println!("printing: {}", (!ch.is_control()) as i32);
    println!(
        "punctuation: {}",
        (ch.is_ascii_punctuation()
            || (!ch.is_alphanumeric() && !ch.is_whitespace() && !ch.is_control())) as i32
    );

    let lower = ch.to_lowercase().next().unwrap_or(ch);
    let upper = ch.to_uppercase().next().unwrap_or(ch);

    println!("to lower: {}", lower);
    println!("to upper: {}", upper);
}

