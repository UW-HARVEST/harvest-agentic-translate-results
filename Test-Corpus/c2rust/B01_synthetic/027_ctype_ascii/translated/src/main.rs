#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
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
pub unsafe extern "C" fn driver(mut c: ::core::ffi::c_char) {
    setlocale(LC_ALL, b"C\0" as *const u8 as *const ::core::ffi::c_char);
    printf(
        b"alphanumeric: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISalnum as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"alphabetic: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISalpha as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"lowercase: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISlower as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"uppercase: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISupper as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"digit: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"hexadecimal: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISxdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"control: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _IScntrl as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"graphical: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISgraph as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"space: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISspace as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"blank: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISblank as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"printing: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISprint as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"punctuation: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        *(*__ctype_b_loc()).offset(c as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & _ISpunct as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int,
    );
    printf(
        b"to lower: %c\n\0" as *const u8 as *const ::core::ffi::c_char,
        tolower(c as ::core::ffi::c_int),
    );
    printf(
        b"to upper: %c\n\0" as *const u8 as *const ::core::ffi::c_char,
        toupper(c as ::core::ffi::c_int),
    );
}
unsafe fn main_0() -> ::core::ffi::c_int {
    let mut c: ::core::ffi::c_char = getchar() as ::core::ffi::c_char;
    driver(c);
    return 0;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
