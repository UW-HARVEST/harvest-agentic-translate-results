extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn __assert_fail(
        __assertion: *const ::core::ffi::c_char,
        __file: *const ::core::ffi::c_char,
        __line: ::core::ffi::c_uint,
        __function: *const ::core::ffi::c_char,
    ) -> !;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const REPLACEMENT_INC: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn w_utf8_drop(
    mut string: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    '_c2rust_label: {
        if !string.is_null() {
        } else {
            __assert_fail(
                b"string != NULL\0" as *const u8 as *const ::core::ffi::c_char,
                b"/tmp/harvest-translate-JpYcWR/driver/c_src/src/lib.c\0" as *const u8
                    as *const ::core::ffi::c_char,
                40 as ::core::ffi::c_uint,
                b"const char *w_utf8_drop(const char *)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
    while *string != 0 {
        if *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0x80 as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
        {
            string = string.offset(1);
        } else if *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xe0 as ::core::ffi::c_int
            == 0xc0 as ::core::ffi::c_int
            && *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                >= 0xc2 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
            && *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
        {
            string = string.offset(2 as ::core::ffi::c_int as isize);
        } else if *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf0 as ::core::ffi::c_int
            == 0xe0 as ::core::ffi::c_int
            && *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *string.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && (*string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xe0 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    >= 0xa0 as ::core::ffi::c_int)
            && (*string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xed as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || (*string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int)
                    < 0xa0 as ::core::ffi::c_int)
            && (*string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xef as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    <= 0xbf as ::core::ffi::c_int)
        {
            string = string.offset(3 as ::core::ffi::c_int as isize);
        } else if *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf8 as ::core::ffi::c_int
            == 0xf0 as ::core::ffi::c_int
            && *string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                as ::core::ffi::c_int
                <= 0xf4 as ::core::ffi::c_int
            && *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *string.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *string.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && (*string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xf0 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    >= 0x90 as ::core::ffi::c_int)
            && (*string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xf4 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    <= 0x8f as ::core::ffi::c_int)
        {
            string = string.offset(4 as ::core::ffi::c_int as isize);
        } else {
            return string;
        }
    }
    return string;
}
#[no_mangle]
pub unsafe extern "C" fn w_utf8_filter(
    mut string: *const ::core::ffi::c_char,
    mut replacement: bool,
) -> *mut ::core::ffi::c_char {
    '_c2rust_label: {
        if !string.is_null() {
        } else {
            __assert_fail(
                b"string != NULL\0" as *const u8 as *const ::core::ffi::c_char,
                b"/tmp/harvest-translate-JpYcWR/driver/c_src/src/lib.c\0" as *const u8
                    as *const ::core::ffi::c_char,
                60 as ::core::ffi::c_uint,
                b"char *w_utf8_filter(const char *, _Bool)\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    };
    let mut valid: *const ::core::ffi::c_char = w_utf8_drop(string);
    if *valid as ::core::ffi::c_int == '\0' as i32 {
        let mut copy: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        copy = strdup(string);
        return copy;
    }
    let mut size: size_t = strlen(string).wrapping_add(1 as size_t);
    let mut copy_0: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut i: size_t = valid.offset_from(string) as ::core::ffi::c_long as size_t;
    let mut repl: size_t = 0 as size_t;
    copy_0 = malloc(size) as *mut ::core::ffi::c_char;
    if copy_0.is_null() {
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    memcpy(
        copy_0 as *mut ::core::ffi::c_void,
        string as *const ::core::ffi::c_void,
        i,
    );
    while *valid != 0 {
        if *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0x80 as ::core::ffi::c_int
            == 0 as ::core::ffi::c_int
        {
            let fresh0 = valid;
            valid = valid.offset(1);
            let fresh1 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh1 as isize) = *fresh0;
        } else if *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xe0 as ::core::ffi::c_int
            == 0xc0 as ::core::ffi::c_int
            && *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                >= 0xc2 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
            && *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
        {
            let fresh2 = valid;
            valid = valid.offset(1);
            let fresh3 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh3 as isize) = *fresh2;
            let fresh4 = valid;
            valid = valid.offset(1);
            let fresh5 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh5 as isize) = *fresh4;
        } else if *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf0 as ::core::ffi::c_int
            == 0xe0 as ::core::ffi::c_int
            && *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *valid.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && (*valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xe0 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    >= 0xa0 as ::core::ffi::c_int)
            && (*valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xed as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || (*valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int)
                    < 0xa0 as ::core::ffi::c_int)
            && (*valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xef as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    <= 0xbf as ::core::ffi::c_int)
        {
            let fresh6 = valid;
            valid = valid.offset(1);
            let fresh7 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh7 as isize) = *fresh6;
            let fresh8 = valid;
            valid = valid.offset(1);
            let fresh9 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh9 as isize) = *fresh8;
            let fresh10 = valid;
            valid = valid.offset(1);
            let fresh11 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh11 as isize) = *fresh10;
        } else if *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            & 0xf8 as ::core::ffi::c_int
            == 0xf0 as ::core::ffi::c_int
            && *valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                as ::core::ffi::c_int
                <= 0xf4 as ::core::ffi::c_int
            && *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *valid.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && *valid.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                & 0xc0 as ::core::ffi::c_int
                == 0x80 as ::core::ffi::c_int
            && (*valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xf0 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    >= 0x90 as ::core::ffi::c_int)
            && (*valid.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != 0xf4 as ::core::ffi::c_int as ::core::ffi::c_char as ::core::ffi::c_int
                || *valid.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uchar
                    as ::core::ffi::c_int
                    <= 0x8f as ::core::ffi::c_int)
        {
            let fresh12 = valid;
            valid = valid.offset(1);
            let fresh13 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh13 as isize) = *fresh12;
            let fresh14 = valid;
            valid = valid.offset(1);
            let fresh15 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh15 as isize) = *fresh14;
            let fresh16 = valid;
            valid = valid.offset(1);
            let fresh17 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh17 as isize) = *fresh16;
            let fresh18 = valid;
            valid = valid.offset(1);
            let fresh19 = i;
            i = i.wrapping_add(1);
            *copy_0.offset(fresh19 as isize) = *fresh18;
        } else {
            if replacement {
                if repl < 3 as size_t {
                    size = (size as ::core::ffi::c_ulong)
                        .wrapping_add(REPLACEMENT_INC as ::core::ffi::c_ulong)
                        as size_t as size_t;
                    copy_0 = realloc(copy_0 as *mut ::core::ffi::c_void, size)
                        as *mut ::core::ffi::c_char;
                    if copy_0.is_null() {
                        return ::core::ptr::null_mut::<::core::ffi::c_char>();
                    }
                    repl = (repl as ::core::ffi::c_ulong)
                        .wrapping_add(REPLACEMENT_INC as ::core::ffi::c_ulong)
                        as size_t as size_t;
                }
                let fresh20 = i;
                i = i.wrapping_add(1);
                *copy_0.offset(fresh20 as isize) =
                    0xef as ::core::ffi::c_int as ::core::ffi::c_char;
                let fresh21 = i;
                i = i.wrapping_add(1);
                *copy_0.offset(fresh21 as isize) =
                    0xbf as ::core::ffi::c_int as ::core::ffi::c_char;
                let fresh22 = i;
                i = i.wrapping_add(1);
                *copy_0.offset(fresh22 as isize) =
                    0xbd as ::core::ffi::c_int as ::core::ffi::c_char;
                repl = (repl as ::core::ffi::c_ulong).wrapping_sub(3 as ::core::ffi::c_ulong)
                    as size_t as size_t;
            }
            valid = valid.offset(1);
        }
    }
    *copy_0.offset(i as isize) = '\0' as i32 as ::core::ffi::c_char;
    return copy_0;
}
