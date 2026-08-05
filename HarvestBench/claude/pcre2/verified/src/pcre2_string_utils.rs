use crate::pcre2_internal::*;
use core::ffi::c_char;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_8(mut str1: PCRE2_SPTR, mut str2: PCRE2_SPTR) -> i32 {
    while *str1 != 0 || *str2 != 0 {
        let c1 = *str1;
        let c2 = *str2;
        str1 = str1.add(1);
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcmp_c8_8(mut str1: PCRE2_SPTR, mut str2: *const c_char) -> i32 {
    while *str1 != 0 || *str2 != 0 {
        let c1 = *str1;
        let c2 = *str2 as u8;
        str1 = str1.add(1);
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_8(
    mut str1: PCRE2_SPTR,
    mut str2: PCRE2_SPTR,
    mut len: usize,
) -> i32 {
    while len > 0 {
        let c1 = *str1;
        let c2 = *str2;
        str1 = str1.add(1);
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
        len -= 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strncmp_c8_8(
    mut str1: PCRE2_SPTR,
    mut str2: *const c_char,
    mut len: usize,
) -> i32 {
    while len > 0 {
        let c1 = *str1;
        let c2 = *str2 as u8;
        str1 = str1.add(1);
        str2 = str2.add(1);
        if c1 != c2 {
            return (((c1 > c2) as i32) << 1) - 1;
        }
        len -= 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strlen_8(mut str: PCRE2_SPTR) -> PCRE2_SIZE {
    let mut c: PCRE2_SIZE = 0;
    while *str != 0 {
        str = str.add(1);
        c += 1;
    }
    c
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_strcpy_c8_8(str1: *mut PCRE2_UCHAR, mut str2: *const c_char) -> PCRE2_SIZE {
    let mut t = str1;
    while *str2 != 0 {
        *t = *str2 as u8;
        t = t.add(1);
        str2 = str2.add(1);
    }
    *t = 0;
    t.offset_from(str1) as PCRE2_SIZE
}
