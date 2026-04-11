extern "C" {
    fn __ctype_b_loc() -> *mut *const libc::c_ushort;
    fn tolower(__c: libc::c_int) -> libc::c_int;
    fn toupper(__c: libc::c_int) -> libc::c_int;
    fn setlocale(
        __category: libc::c_int,
        __locale: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
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
pub type C2RustUnnamed = libc::c_uint;
pub const __LC_ALL: libc::c_int = 6 as libc::c_int;
pub const LC_ALL: libc::c_int = __LC_ALL;
#[no_mangle]
pub unsafe extern "C" fn driver(mut c: libc::c_char) {
    setlocale(LC_ALL, b"C\0" as *const u8 as *const libc::c_char);
    printf(
        b"alphanumeric: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISalnum as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"alphabetic: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISalpha as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"lowercase: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISlower as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"uppercase: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISupper as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"digit: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISdigit as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"hexadecimal: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISxdigit as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"control: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _IScntrl as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"graphical: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISgraph as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"space: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISspace as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"blank: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISblank as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"printing: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISprint as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"punctuation: %d\n\0" as *const u8 as *const libc::c_char,
        *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
            & _ISpunct as libc::c_int as libc::c_ushort as libc::c_int,
    );
    printf(
        b"to lower: %c\n\0" as *const u8 as *const libc::c_char,
        tolower(c as libc::c_int),
    );
    printf(
        b"to upper: %c\n\0" as *const u8 as *const libc::c_char,
        toupper(c as libc::c_int),
    );
}
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

