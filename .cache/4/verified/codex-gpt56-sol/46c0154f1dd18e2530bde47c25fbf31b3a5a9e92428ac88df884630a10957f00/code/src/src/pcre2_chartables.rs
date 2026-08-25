pub mod internal {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct __va_list_tag {
        pub gp_offset: ::core::ffi::c_uint,
        pub fp_offset: ::core::ffi::c_uint,
        pub overflow_arg_area: *mut ::core::ffi::c_void,
        pub reg_save_area: *mut ::core::ffi::c_void,
    }
}
pub mod types_h {
    pub type __uint8_t = u8;
    pub type __uint16_t = u16;
    pub type __int32_t = i32;
    pub type __uint32_t = u32;
    pub type __uint64_t = u64;
    pub type __off_t = ::core::ffi::c_long;
    pub type __off64_t = ::core::ffi::c_long;
    pub type __ssize_t = ::core::ffi::c_long;
}
pub mod stddef_h {
    pub type size_t = usize;
    pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
}
pub mod struct_FILE_h {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct _IO_FILE {
        pub _flags: ::core::ffi::c_int,
        pub _IO_read_ptr: *mut ::core::ffi::c_char,
        pub _IO_read_end: *mut ::core::ffi::c_char,
        pub _IO_read_base: *mut ::core::ffi::c_char,
        pub _IO_write_base: *mut ::core::ffi::c_char,
        pub _IO_write_ptr: *mut ::core::ffi::c_char,
        pub _IO_write_end: *mut ::core::ffi::c_char,
        pub _IO_buf_base: *mut ::core::ffi::c_char,
        pub _IO_buf_end: *mut ::core::ffi::c_char,
        pub _IO_save_base: *mut ::core::ffi::c_char,
        pub _IO_backup_base: *mut ::core::ffi::c_char,
        pub _IO_save_end: *mut ::core::ffi::c_char,
        pub _markers: *mut _IO_marker,
        pub _chain: *mut _IO_FILE,
        pub _fileno: ::core::ffi::c_int,
        pub _flags2: ::core::ffi::c_int,
        pub _old_offset: __off_t,
        pub _cur_column: ::core::ffi::c_ushort,
        pub _vtable_offset: ::core::ffi::c_schar,
        pub _shortbuf: [::core::ffi::c_char; 1],
        pub _lock: *mut ::core::ffi::c_void,
        pub _offset: __off64_t,
        pub _codecvt: *mut _IO_codecvt,
        pub _wide_data: *mut _IO_wide_data,
        pub _freeres_list: *mut _IO_FILE,
        pub _freeres_buf: *mut ::core::ffi::c_void,
        pub __pad5: size_t,
        pub _mode: ::core::ffi::c_int,
        pub _unused2: [::core::ffi::c_char; 20],
    }
    pub type _IO_lock_t = ();
    pub const _IO_EOF_SEEN: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
    pub const _IO_ERR_SEEN: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
    use super::stddef_h::size_t;
    use super::types_h::{__off64_t, __off_t};
        pub enum _IO_wide_data {}
        pub enum _IO_codecvt {}
        pub enum _IO_marker {}
}
pub mod FILE_h {
    pub type FILE = _IO_FILE;
    use super::struct_FILE_h::_IO_FILE;
}
pub mod stdlib_h {
    pub type __compar_fn_t = Option<
        unsafe extern "C" fn(
            *const ::core::ffi::c_void,
            *const ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >;
    #[inline]
    pub unsafe extern "C" fn atoi(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
        return strtol(
            __nptr,
            NULL as *mut *mut ::core::ffi::c_char,
            10 as ::core::ffi::c_int,
        ) as ::core::ffi::c_int;
    }
    #[inline]
    pub unsafe extern "C" fn atol(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_long {
        return strtol(
            __nptr,
            NULL as *mut *mut ::core::ffi::c_char,
            10 as ::core::ffi::c_int,
        );
    }
    #[inline]
    pub unsafe extern "C" fn atoll(
        mut __nptr: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_longlong {
        return strtoll(
            __nptr,
            NULL as *mut *mut ::core::ffi::c_char,
            10 as ::core::ffi::c_int,
        );
    }
    use super::stddef_h::NULL;
    extern "C" {
        pub fn strtod(
            __nptr: *const ::core::ffi::c_char,
            __endptr: *mut *mut ::core::ffi::c_char,
        ) -> ::core::ffi::c_double;
        pub fn strtol(
            __nptr: *const ::core::ffi::c_char,
            __endptr: *mut *mut ::core::ffi::c_char,
            __base: ::core::ffi::c_int,
        ) -> ::core::ffi::c_long;
        pub fn strtoll(
            __nptr: *const ::core::ffi::c_char,
            __endptr: *mut *mut ::core::ffi::c_char,
            __base: ::core::ffi::c_int,
        ) -> ::core::ffi::c_longlong;
    }
}
pub mod stdint_uintn_h {
    pub type uint8_t = __uint8_t;
    use super::types_h::__uint8_t;
}
pub mod ctype_h {
    #[inline]
    pub unsafe extern "C" fn tolower(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
        return if __c >= -(128 as ::core::ffi::c_int) && __c < 256 as ::core::ffi::c_int {
            *(*__ctype_tolower_loc()).offset(__c as isize) as ::core::ffi::c_int
        } else {
            __c
        };
    }
    #[inline]
    pub unsafe extern "C" fn toupper(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
        return if __c >= -(128 as ::core::ffi::c_int) && __c < 256 as ::core::ffi::c_int {
            *(*__ctype_toupper_loc()).offset(__c as isize) as ::core::ffi::c_int
        } else {
            __c
        };
    }
    use super::types_h::__int32_t;
    extern "C" {
        pub fn __ctype_tolower_loc() -> *mut *const __int32_t;
        pub fn __ctype_toupper_loc() -> *mut *const __int32_t;
    }
}
pub mod stdio_h {
    use super::internal::__va_list_tag;
    use super::stddef_h::size_t;
    use super::types_h::__ssize_t;
    use super::FILE_h::FILE;
    extern "C" {
        pub static mut stdin: *mut FILE;
        pub static mut stdout: *mut FILE;
        pub fn vfprintf(
            __s: *mut FILE,
            __format: *const ::core::ffi::c_char,
            __arg: *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int;
        pub fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
        pub fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
        pub fn __getdelim(
            __lineptr: *mut *mut ::core::ffi::c_char,
            __n: *mut size_t,
            __delimiter: ::core::ffi::c_int,
            __stream: *mut FILE,
        ) -> __ssize_t;
        pub fn __uflow(_: *mut FILE) -> ::core::ffi::c_int;
        pub fn __overflow(_: *mut FILE, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    }
}
pub mod bits_stdio_h {
    #[inline]
    pub unsafe extern "C" fn vprintf(
        mut __fmt: *const ::core::ffi::c_char,
        mut __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int {
        return vfprintf(stdout, __fmt, __arg);
    }
    #[inline]
    pub unsafe extern "C" fn getchar() -> ::core::ffi::c_int {
        return getc(stdin);
    }
    #[inline]
    pub unsafe extern "C" fn fgetc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
        return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __uflow(__fp)
        } else {
            let fresh2 = (*__fp)._IO_read_ptr;
            (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
            *(fresh2 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
        return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __uflow(__fp)
        } else {
            let fresh0 = (*__fp)._IO_read_ptr;
            (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
            *(fresh0 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
        return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __uflow(stdin)
        } else {
            let fresh1 = (*stdin)._IO_read_ptr;
            (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
            *(fresh1 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn putchar(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
        return putc(__c, stdout);
    }
    #[inline]
    pub unsafe extern "C" fn fputc_unlocked(
        mut __c: ::core::ffi::c_int,
        mut __stream: *mut FILE,
    ) -> ::core::ffi::c_int {
        return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __overflow(__stream, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
        } else {
            let fresh3 = (*__stream)._IO_write_ptr;
            (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
            *fresh3 = __c as ::core::ffi::c_char;
            *fresh3 as ::core::ffi::c_uchar as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn putc_unlocked(
        mut __c: ::core::ffi::c_int,
        mut __stream: *mut FILE,
    ) -> ::core::ffi::c_int {
        return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __overflow(__stream, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
        } else {
            let fresh4 = (*__stream)._IO_write_ptr;
            (*__stream)._IO_write_ptr = (*__stream)._IO_write_ptr.offset(1);
            *fresh4 = __c as ::core::ffi::c_char;
            *fresh4 as ::core::ffi::c_uchar as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn putchar_unlocked(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
        return if ((*stdout)._IO_write_ptr >= (*stdout)._IO_write_end) as ::core::ffi::c_int
            as ::core::ffi::c_long
            != 0
        {
            __overflow(stdout, __c as ::core::ffi::c_uchar as ::core::ffi::c_int)
        } else {
            let fresh5 = (*stdout)._IO_write_ptr;
            (*stdout)._IO_write_ptr = (*stdout)._IO_write_ptr.offset(1);
            *fresh5 = __c as ::core::ffi::c_char;
            *fresh5 as ::core::ffi::c_uchar as ::core::ffi::c_int
        };
    }
    #[inline]
    pub unsafe extern "C" fn getline(
        mut __lineptr: *mut *mut ::core::ffi::c_char,
        mut __n: *mut size_t,
        mut __stream: *mut FILE,
    ) -> __ssize_t {
        return __getdelim(__lineptr, __n, '\n' as i32, __stream);
    }
    #[inline]
    pub unsafe extern "C" fn feof_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
        return ((*__stream)._flags & _IO_EOF_SEEN != 0 as ::core::ffi::c_int)
            as ::core::ffi::c_int;
    }
    #[inline]
    pub unsafe extern "C" fn ferror_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
        return ((*__stream)._flags & _IO_ERR_SEEN != 0 as ::core::ffi::c_int)
            as ::core::ffi::c_int;
    }
    use super::internal::__va_list_tag;
    use super::stddef_h::size_t;
    use super::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
    use super::struct_FILE_h::{_IO_EOF_SEEN, _IO_ERR_SEEN};
    use super::types_h::__ssize_t;
    use super::FILE_h::FILE;
}
pub mod stdlib_float_h {
    #[inline]
    pub unsafe extern "C" fn atof(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_double {
        return strtod(__nptr, NULL as *mut *mut ::core::ffi::c_char);
    }
    use super::stddef_h::NULL;
    use super::stdlib_h::strtod;
}
pub mod byteswap_h {
    #[inline]
    pub unsafe extern "C" fn __bswap_16(mut __bsx: __uint16_t) -> __uint16_t {
        return (__bsx as ::core::ffi::c_int >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
            as __uint16_t;
    }
    #[inline]
    pub unsafe extern "C" fn __bswap_32(mut __bsx: __uint32_t) -> __uint32_t {
        return (__bsx & 0xff000000 as __uint32_t) >> 24 as ::core::ffi::c_int
            | (__bsx & 0xff0000 as __uint32_t) >> 8 as ::core::ffi::c_int
            | (__bsx & 0xff00 as __uint32_t) << 8 as ::core::ffi::c_int
            | (__bsx & 0xff as __uint32_t) << 24 as ::core::ffi::c_int;
    }
    #[inline]
    pub unsafe extern "C" fn __bswap_64(mut __bsx: __uint64_t) -> __uint64_t {
        return ((__bsx as ::core::ffi::c_ulonglong
            & 0xff00000000000000 as ::core::ffi::c_ulonglong)
            >> 56 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff000000000000 as ::core::ffi::c_ulonglong)
                >> 40 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff0000000000 as ::core::ffi::c_ulonglong)
                >> 24 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff00000000 as ::core::ffi::c_ulonglong)
                >> 8 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff000000 as ::core::ffi::c_ulonglong)
                << 8 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff0000 as ::core::ffi::c_ulonglong)
                << 24 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff00 as ::core::ffi::c_ulonglong)
                << 40 as ::core::ffi::c_int
            | (__bsx as ::core::ffi::c_ulonglong & 0xff as ::core::ffi::c_ulonglong)
                << 56 as ::core::ffi::c_int) as __uint64_t;
    }
    use super::types_h::{__uint16_t, __uint32_t, __uint64_t};
}
pub mod uintn_identity_h {
    #[inline]
    pub unsafe extern "C" fn __uint16_identity(mut __x: __uint16_t) -> __uint16_t {
        return __x;
    }
    #[inline]
    pub unsafe extern "C" fn __uint32_identity(mut __x: __uint32_t) -> __uint32_t {
        return __x;
    }
    #[inline]
    pub unsafe extern "C" fn __uint64_identity(mut __x: __uint64_t) -> __uint64_t {
        return __x;
    }
    use super::types_h::{__uint16_t, __uint32_t, __uint64_t};
}
pub mod stdlib_bsearch_h {
    #[inline]
    pub unsafe extern "C" fn bsearch(
        mut __key: *const ::core::ffi::c_void,
        mut __base: *const ::core::ffi::c_void,
        mut __nmemb: size_t,
        mut __size: size_t,
        mut __compar: __compar_fn_t,
    ) -> *mut ::core::ffi::c_void {
        let mut __l: size_t = 0;
        let mut __u: size_t = 0;
        let mut __idx: size_t = 0;
        let mut __p: *const ::core::ffi::c_void = ::core::ptr::null::<::core::ffi::c_void>();
        let mut __comparison: ::core::ffi::c_int = 0;
        __l = 0 as size_t;
        __u = __nmemb;
        while __l < __u {
            __idx = __l.wrapping_add(__u).wrapping_div(2 as size_t);
            __p = (__base as *const ::core::ffi::c_char).offset(__idx.wrapping_mul(__size) as isize)
                as *const ::core::ffi::c_void;
            __comparison = Some(__compar.expect("non-null function pointer"))
                .expect("non-null function pointer")(__key, __p);
            if __comparison < 0 as ::core::ffi::c_int {
                __u = __idx;
            } else if __comparison > 0 as ::core::ffi::c_int {
                __l = __idx.wrapping_add(1 as size_t);
            } else {
                return __p as *mut ::core::ffi::c_void;
            }
        }
        return NULL;
    }
    use super::stddef_h::{size_t, NULL};
    use super::stdlib_h::__compar_fn_t;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::stddef_h::{size_t, NULL};
pub use self::stdint_uintn_h::uint8_t;
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, strtod, strtol, strtoll};
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
#[unsafe(no_mangle)]
pub static mut _pcre2_default_tables_8: [uint8_t; 1088] = [
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    6 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    10 as ::core::ffi::c_int as uint8_t,
    11 as ::core::ffi::c_int as uint8_t,
    12 as ::core::ffi::c_int as uint8_t,
    13 as ::core::ffi::c_int as uint8_t,
    14 as ::core::ffi::c_int as uint8_t,
    15 as ::core::ffi::c_int as uint8_t,
    16 as ::core::ffi::c_int as uint8_t,
    17 as ::core::ffi::c_int as uint8_t,
    18 as ::core::ffi::c_int as uint8_t,
    19 as ::core::ffi::c_int as uint8_t,
    20 as ::core::ffi::c_int as uint8_t,
    21 as ::core::ffi::c_int as uint8_t,
    22 as ::core::ffi::c_int as uint8_t,
    23 as ::core::ffi::c_int as uint8_t,
    24 as ::core::ffi::c_int as uint8_t,
    25 as ::core::ffi::c_int as uint8_t,
    26 as ::core::ffi::c_int as uint8_t,
    27 as ::core::ffi::c_int as uint8_t,
    28 as ::core::ffi::c_int as uint8_t,
    29 as ::core::ffi::c_int as uint8_t,
    30 as ::core::ffi::c_int as uint8_t,
    31 as ::core::ffi::c_int as uint8_t,
    32 as ::core::ffi::c_int as uint8_t,
    33 as ::core::ffi::c_int as uint8_t,
    34 as ::core::ffi::c_int as uint8_t,
    35 as ::core::ffi::c_int as uint8_t,
    36 as ::core::ffi::c_int as uint8_t,
    37 as ::core::ffi::c_int as uint8_t,
    38 as ::core::ffi::c_int as uint8_t,
    39 as ::core::ffi::c_int as uint8_t,
    40 as ::core::ffi::c_int as uint8_t,
    41 as ::core::ffi::c_int as uint8_t,
    42 as ::core::ffi::c_int as uint8_t,
    43 as ::core::ffi::c_int as uint8_t,
    44 as ::core::ffi::c_int as uint8_t,
    45 as ::core::ffi::c_int as uint8_t,
    46 as ::core::ffi::c_int as uint8_t,
    47 as ::core::ffi::c_int as uint8_t,
    48 as ::core::ffi::c_int as uint8_t,
    49 as ::core::ffi::c_int as uint8_t,
    50 as ::core::ffi::c_int as uint8_t,
    51 as ::core::ffi::c_int as uint8_t,
    52 as ::core::ffi::c_int as uint8_t,
    53 as ::core::ffi::c_int as uint8_t,
    54 as ::core::ffi::c_int as uint8_t,
    55 as ::core::ffi::c_int as uint8_t,
    56 as ::core::ffi::c_int as uint8_t,
    57 as ::core::ffi::c_int as uint8_t,
    58 as ::core::ffi::c_int as uint8_t,
    59 as ::core::ffi::c_int as uint8_t,
    60 as ::core::ffi::c_int as uint8_t,
    61 as ::core::ffi::c_int as uint8_t,
    62 as ::core::ffi::c_int as uint8_t,
    63 as ::core::ffi::c_int as uint8_t,
    64 as ::core::ffi::c_int as uint8_t,
    97 as ::core::ffi::c_int as uint8_t,
    98 as ::core::ffi::c_int as uint8_t,
    99 as ::core::ffi::c_int as uint8_t,
    100 as ::core::ffi::c_int as uint8_t,
    101 as ::core::ffi::c_int as uint8_t,
    102 as ::core::ffi::c_int as uint8_t,
    103 as ::core::ffi::c_int as uint8_t,
    104 as ::core::ffi::c_int as uint8_t,
    105 as ::core::ffi::c_int as uint8_t,
    106 as ::core::ffi::c_int as uint8_t,
    107 as ::core::ffi::c_int as uint8_t,
    108 as ::core::ffi::c_int as uint8_t,
    109 as ::core::ffi::c_int as uint8_t,
    110 as ::core::ffi::c_int as uint8_t,
    111 as ::core::ffi::c_int as uint8_t,
    112 as ::core::ffi::c_int as uint8_t,
    113 as ::core::ffi::c_int as uint8_t,
    114 as ::core::ffi::c_int as uint8_t,
    115 as ::core::ffi::c_int as uint8_t,
    116 as ::core::ffi::c_int as uint8_t,
    117 as ::core::ffi::c_int as uint8_t,
    118 as ::core::ffi::c_int as uint8_t,
    119 as ::core::ffi::c_int as uint8_t,
    120 as ::core::ffi::c_int as uint8_t,
    121 as ::core::ffi::c_int as uint8_t,
    122 as ::core::ffi::c_int as uint8_t,
    91 as ::core::ffi::c_int as uint8_t,
    92 as ::core::ffi::c_int as uint8_t,
    93 as ::core::ffi::c_int as uint8_t,
    94 as ::core::ffi::c_int as uint8_t,
    95 as ::core::ffi::c_int as uint8_t,
    96 as ::core::ffi::c_int as uint8_t,
    97 as ::core::ffi::c_int as uint8_t,
    98 as ::core::ffi::c_int as uint8_t,
    99 as ::core::ffi::c_int as uint8_t,
    100 as ::core::ffi::c_int as uint8_t,
    101 as ::core::ffi::c_int as uint8_t,
    102 as ::core::ffi::c_int as uint8_t,
    103 as ::core::ffi::c_int as uint8_t,
    104 as ::core::ffi::c_int as uint8_t,
    105 as ::core::ffi::c_int as uint8_t,
    106 as ::core::ffi::c_int as uint8_t,
    107 as ::core::ffi::c_int as uint8_t,
    108 as ::core::ffi::c_int as uint8_t,
    109 as ::core::ffi::c_int as uint8_t,
    110 as ::core::ffi::c_int as uint8_t,
    111 as ::core::ffi::c_int as uint8_t,
    112 as ::core::ffi::c_int as uint8_t,
    113 as ::core::ffi::c_int as uint8_t,
    114 as ::core::ffi::c_int as uint8_t,
    115 as ::core::ffi::c_int as uint8_t,
    116 as ::core::ffi::c_int as uint8_t,
    117 as ::core::ffi::c_int as uint8_t,
    118 as ::core::ffi::c_int as uint8_t,
    119 as ::core::ffi::c_int as uint8_t,
    120 as ::core::ffi::c_int as uint8_t,
    121 as ::core::ffi::c_int as uint8_t,
    122 as ::core::ffi::c_int as uint8_t,
    123 as ::core::ffi::c_int as uint8_t,
    124 as ::core::ffi::c_int as uint8_t,
    125 as ::core::ffi::c_int as uint8_t,
    126 as ::core::ffi::c_int as uint8_t,
    127 as ::core::ffi::c_int as uint8_t,
    128 as ::core::ffi::c_int as uint8_t,
    129 as ::core::ffi::c_int as uint8_t,
    130 as ::core::ffi::c_int as uint8_t,
    131 as ::core::ffi::c_int as uint8_t,
    132 as ::core::ffi::c_int as uint8_t,
    133 as ::core::ffi::c_int as uint8_t,
    134 as ::core::ffi::c_int as uint8_t,
    135 as ::core::ffi::c_int as uint8_t,
    136 as ::core::ffi::c_int as uint8_t,
    137 as ::core::ffi::c_int as uint8_t,
    138 as ::core::ffi::c_int as uint8_t,
    139 as ::core::ffi::c_int as uint8_t,
    140 as ::core::ffi::c_int as uint8_t,
    141 as ::core::ffi::c_int as uint8_t,
    142 as ::core::ffi::c_int as uint8_t,
    143 as ::core::ffi::c_int as uint8_t,
    144 as ::core::ffi::c_int as uint8_t,
    145 as ::core::ffi::c_int as uint8_t,
    146 as ::core::ffi::c_int as uint8_t,
    147 as ::core::ffi::c_int as uint8_t,
    148 as ::core::ffi::c_int as uint8_t,
    149 as ::core::ffi::c_int as uint8_t,
    150 as ::core::ffi::c_int as uint8_t,
    151 as ::core::ffi::c_int as uint8_t,
    152 as ::core::ffi::c_int as uint8_t,
    153 as ::core::ffi::c_int as uint8_t,
    154 as ::core::ffi::c_int as uint8_t,
    155 as ::core::ffi::c_int as uint8_t,
    156 as ::core::ffi::c_int as uint8_t,
    157 as ::core::ffi::c_int as uint8_t,
    158 as ::core::ffi::c_int as uint8_t,
    159 as ::core::ffi::c_int as uint8_t,
    160 as ::core::ffi::c_int as uint8_t,
    161 as ::core::ffi::c_int as uint8_t,
    162 as ::core::ffi::c_int as uint8_t,
    163 as ::core::ffi::c_int as uint8_t,
    164 as ::core::ffi::c_int as uint8_t,
    165 as ::core::ffi::c_int as uint8_t,
    166 as ::core::ffi::c_int as uint8_t,
    167 as ::core::ffi::c_int as uint8_t,
    168 as ::core::ffi::c_int as uint8_t,
    169 as ::core::ffi::c_int as uint8_t,
    170 as ::core::ffi::c_int as uint8_t,
    171 as ::core::ffi::c_int as uint8_t,
    172 as ::core::ffi::c_int as uint8_t,
    173 as ::core::ffi::c_int as uint8_t,
    174 as ::core::ffi::c_int as uint8_t,
    175 as ::core::ffi::c_int as uint8_t,
    176 as ::core::ffi::c_int as uint8_t,
    177 as ::core::ffi::c_int as uint8_t,
    178 as ::core::ffi::c_int as uint8_t,
    179 as ::core::ffi::c_int as uint8_t,
    180 as ::core::ffi::c_int as uint8_t,
    181 as ::core::ffi::c_int as uint8_t,
    182 as ::core::ffi::c_int as uint8_t,
    183 as ::core::ffi::c_int as uint8_t,
    184 as ::core::ffi::c_int as uint8_t,
    185 as ::core::ffi::c_int as uint8_t,
    186 as ::core::ffi::c_int as uint8_t,
    187 as ::core::ffi::c_int as uint8_t,
    188 as ::core::ffi::c_int as uint8_t,
    189 as ::core::ffi::c_int as uint8_t,
    190 as ::core::ffi::c_int as uint8_t,
    191 as ::core::ffi::c_int as uint8_t,
    192 as ::core::ffi::c_int as uint8_t,
    193 as ::core::ffi::c_int as uint8_t,
    194 as ::core::ffi::c_int as uint8_t,
    195 as ::core::ffi::c_int as uint8_t,
    196 as ::core::ffi::c_int as uint8_t,
    197 as ::core::ffi::c_int as uint8_t,
    198 as ::core::ffi::c_int as uint8_t,
    199 as ::core::ffi::c_int as uint8_t,
    200 as ::core::ffi::c_int as uint8_t,
    201 as ::core::ffi::c_int as uint8_t,
    202 as ::core::ffi::c_int as uint8_t,
    203 as ::core::ffi::c_int as uint8_t,
    204 as ::core::ffi::c_int as uint8_t,
    205 as ::core::ffi::c_int as uint8_t,
    206 as ::core::ffi::c_int as uint8_t,
    207 as ::core::ffi::c_int as uint8_t,
    208 as ::core::ffi::c_int as uint8_t,
    209 as ::core::ffi::c_int as uint8_t,
    210 as ::core::ffi::c_int as uint8_t,
    211 as ::core::ffi::c_int as uint8_t,
    212 as ::core::ffi::c_int as uint8_t,
    213 as ::core::ffi::c_int as uint8_t,
    214 as ::core::ffi::c_int as uint8_t,
    215 as ::core::ffi::c_int as uint8_t,
    216 as ::core::ffi::c_int as uint8_t,
    217 as ::core::ffi::c_int as uint8_t,
    218 as ::core::ffi::c_int as uint8_t,
    219 as ::core::ffi::c_int as uint8_t,
    220 as ::core::ffi::c_int as uint8_t,
    221 as ::core::ffi::c_int as uint8_t,
    222 as ::core::ffi::c_int as uint8_t,
    223 as ::core::ffi::c_int as uint8_t,
    224 as ::core::ffi::c_int as uint8_t,
    225 as ::core::ffi::c_int as uint8_t,
    226 as ::core::ffi::c_int as uint8_t,
    227 as ::core::ffi::c_int as uint8_t,
    228 as ::core::ffi::c_int as uint8_t,
    229 as ::core::ffi::c_int as uint8_t,
    230 as ::core::ffi::c_int as uint8_t,
    231 as ::core::ffi::c_int as uint8_t,
    232 as ::core::ffi::c_int as uint8_t,
    233 as ::core::ffi::c_int as uint8_t,
    234 as ::core::ffi::c_int as uint8_t,
    235 as ::core::ffi::c_int as uint8_t,
    236 as ::core::ffi::c_int as uint8_t,
    237 as ::core::ffi::c_int as uint8_t,
    238 as ::core::ffi::c_int as uint8_t,
    239 as ::core::ffi::c_int as uint8_t,
    240 as ::core::ffi::c_int as uint8_t,
    241 as ::core::ffi::c_int as uint8_t,
    242 as ::core::ffi::c_int as uint8_t,
    243 as ::core::ffi::c_int as uint8_t,
    244 as ::core::ffi::c_int as uint8_t,
    245 as ::core::ffi::c_int as uint8_t,
    246 as ::core::ffi::c_int as uint8_t,
    247 as ::core::ffi::c_int as uint8_t,
    248 as ::core::ffi::c_int as uint8_t,
    249 as ::core::ffi::c_int as uint8_t,
    250 as ::core::ffi::c_int as uint8_t,
    251 as ::core::ffi::c_int as uint8_t,
    252 as ::core::ffi::c_int as uint8_t,
    253 as ::core::ffi::c_int as uint8_t,
    254 as ::core::ffi::c_int as uint8_t,
    255 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    1 as ::core::ffi::c_int as uint8_t,
    2 as ::core::ffi::c_int as uint8_t,
    3 as ::core::ffi::c_int as uint8_t,
    4 as ::core::ffi::c_int as uint8_t,
    5 as ::core::ffi::c_int as uint8_t,
    6 as ::core::ffi::c_int as uint8_t,
    7 as ::core::ffi::c_int as uint8_t,
    8 as ::core::ffi::c_int as uint8_t,
    9 as ::core::ffi::c_int as uint8_t,
    10 as ::core::ffi::c_int as uint8_t,
    11 as ::core::ffi::c_int as uint8_t,
    12 as ::core::ffi::c_int as uint8_t,
    13 as ::core::ffi::c_int as uint8_t,
    14 as ::core::ffi::c_int as uint8_t,
    15 as ::core::ffi::c_int as uint8_t,
    16 as ::core::ffi::c_int as uint8_t,
    17 as ::core::ffi::c_int as uint8_t,
    18 as ::core::ffi::c_int as uint8_t,
    19 as ::core::ffi::c_int as uint8_t,
    20 as ::core::ffi::c_int as uint8_t,
    21 as ::core::ffi::c_int as uint8_t,
    22 as ::core::ffi::c_int as uint8_t,
    23 as ::core::ffi::c_int as uint8_t,
    24 as ::core::ffi::c_int as uint8_t,
    25 as ::core::ffi::c_int as uint8_t,
    26 as ::core::ffi::c_int as uint8_t,
    27 as ::core::ffi::c_int as uint8_t,
    28 as ::core::ffi::c_int as uint8_t,
    29 as ::core::ffi::c_int as uint8_t,
    30 as ::core::ffi::c_int as uint8_t,
    31 as ::core::ffi::c_int as uint8_t,
    32 as ::core::ffi::c_int as uint8_t,
    33 as ::core::ffi::c_int as uint8_t,
    34 as ::core::ffi::c_int as uint8_t,
    35 as ::core::ffi::c_int as uint8_t,
    36 as ::core::ffi::c_int as uint8_t,
    37 as ::core::ffi::c_int as uint8_t,
    38 as ::core::ffi::c_int as uint8_t,
    39 as ::core::ffi::c_int as uint8_t,
    40 as ::core::ffi::c_int as uint8_t,
    41 as ::core::ffi::c_int as uint8_t,
    42 as ::core::ffi::c_int as uint8_t,
    43 as ::core::ffi::c_int as uint8_t,
    44 as ::core::ffi::c_int as uint8_t,
    45 as ::core::ffi::c_int as uint8_t,
    46 as ::core::ffi::c_int as uint8_t,
    47 as ::core::ffi::c_int as uint8_t,
    48 as ::core::ffi::c_int as uint8_t,
    49 as ::core::ffi::c_int as uint8_t,
    50 as ::core::ffi::c_int as uint8_t,
    51 as ::core::ffi::c_int as uint8_t,
    52 as ::core::ffi::c_int as uint8_t,
    53 as ::core::ffi::c_int as uint8_t,
    54 as ::core::ffi::c_int as uint8_t,
    55 as ::core::ffi::c_int as uint8_t,
    56 as ::core::ffi::c_int as uint8_t,
    57 as ::core::ffi::c_int as uint8_t,
    58 as ::core::ffi::c_int as uint8_t,
    59 as ::core::ffi::c_int as uint8_t,
    60 as ::core::ffi::c_int as uint8_t,
    61 as ::core::ffi::c_int as uint8_t,
    62 as ::core::ffi::c_int as uint8_t,
    63 as ::core::ffi::c_int as uint8_t,
    64 as ::core::ffi::c_int as uint8_t,
    97 as ::core::ffi::c_int as uint8_t,
    98 as ::core::ffi::c_int as uint8_t,
    99 as ::core::ffi::c_int as uint8_t,
    100 as ::core::ffi::c_int as uint8_t,
    101 as ::core::ffi::c_int as uint8_t,
    102 as ::core::ffi::c_int as uint8_t,
    103 as ::core::ffi::c_int as uint8_t,
    104 as ::core::ffi::c_int as uint8_t,
    105 as ::core::ffi::c_int as uint8_t,
    106 as ::core::ffi::c_int as uint8_t,
    107 as ::core::ffi::c_int as uint8_t,
    108 as ::core::ffi::c_int as uint8_t,
    109 as ::core::ffi::c_int as uint8_t,
    110 as ::core::ffi::c_int as uint8_t,
    111 as ::core::ffi::c_int as uint8_t,
    112 as ::core::ffi::c_int as uint8_t,
    113 as ::core::ffi::c_int as uint8_t,
    114 as ::core::ffi::c_int as uint8_t,
    115 as ::core::ffi::c_int as uint8_t,
    116 as ::core::ffi::c_int as uint8_t,
    117 as ::core::ffi::c_int as uint8_t,
    118 as ::core::ffi::c_int as uint8_t,
    119 as ::core::ffi::c_int as uint8_t,
    120 as ::core::ffi::c_int as uint8_t,
    121 as ::core::ffi::c_int as uint8_t,
    122 as ::core::ffi::c_int as uint8_t,
    91 as ::core::ffi::c_int as uint8_t,
    92 as ::core::ffi::c_int as uint8_t,
    93 as ::core::ffi::c_int as uint8_t,
    94 as ::core::ffi::c_int as uint8_t,
    95 as ::core::ffi::c_int as uint8_t,
    96 as ::core::ffi::c_int as uint8_t,
    65 as ::core::ffi::c_int as uint8_t,
    66 as ::core::ffi::c_int as uint8_t,
    67 as ::core::ffi::c_int as uint8_t,
    68 as ::core::ffi::c_int as uint8_t,
    69 as ::core::ffi::c_int as uint8_t,
    70 as ::core::ffi::c_int as uint8_t,
    71 as ::core::ffi::c_int as uint8_t,
    72 as ::core::ffi::c_int as uint8_t,
    73 as ::core::ffi::c_int as uint8_t,
    74 as ::core::ffi::c_int as uint8_t,
    75 as ::core::ffi::c_int as uint8_t,
    76 as ::core::ffi::c_int as uint8_t,
    77 as ::core::ffi::c_int as uint8_t,
    78 as ::core::ffi::c_int as uint8_t,
    79 as ::core::ffi::c_int as uint8_t,
    80 as ::core::ffi::c_int as uint8_t,
    81 as ::core::ffi::c_int as uint8_t,
    82 as ::core::ffi::c_int as uint8_t,
    83 as ::core::ffi::c_int as uint8_t,
    84 as ::core::ffi::c_int as uint8_t,
    85 as ::core::ffi::c_int as uint8_t,
    86 as ::core::ffi::c_int as uint8_t,
    87 as ::core::ffi::c_int as uint8_t,
    88 as ::core::ffi::c_int as uint8_t,
    89 as ::core::ffi::c_int as uint8_t,
    90 as ::core::ffi::c_int as uint8_t,
    123 as ::core::ffi::c_int as uint8_t,
    124 as ::core::ffi::c_int as uint8_t,
    125 as ::core::ffi::c_int as uint8_t,
    126 as ::core::ffi::c_int as uint8_t,
    127 as ::core::ffi::c_int as uint8_t,
    128 as ::core::ffi::c_int as uint8_t,
    129 as ::core::ffi::c_int as uint8_t,
    130 as ::core::ffi::c_int as uint8_t,
    131 as ::core::ffi::c_int as uint8_t,
    132 as ::core::ffi::c_int as uint8_t,
    133 as ::core::ffi::c_int as uint8_t,
    134 as ::core::ffi::c_int as uint8_t,
    135 as ::core::ffi::c_int as uint8_t,
    136 as ::core::ffi::c_int as uint8_t,
    137 as ::core::ffi::c_int as uint8_t,
    138 as ::core::ffi::c_int as uint8_t,
    139 as ::core::ffi::c_int as uint8_t,
    140 as ::core::ffi::c_int as uint8_t,
    141 as ::core::ffi::c_int as uint8_t,
    142 as ::core::ffi::c_int as uint8_t,
    143 as ::core::ffi::c_int as uint8_t,
    144 as ::core::ffi::c_int as uint8_t,
    145 as ::core::ffi::c_int as uint8_t,
    146 as ::core::ffi::c_int as uint8_t,
    147 as ::core::ffi::c_int as uint8_t,
    148 as ::core::ffi::c_int as uint8_t,
    149 as ::core::ffi::c_int as uint8_t,
    150 as ::core::ffi::c_int as uint8_t,
    151 as ::core::ffi::c_int as uint8_t,
    152 as ::core::ffi::c_int as uint8_t,
    153 as ::core::ffi::c_int as uint8_t,
    154 as ::core::ffi::c_int as uint8_t,
    155 as ::core::ffi::c_int as uint8_t,
    156 as ::core::ffi::c_int as uint8_t,
    157 as ::core::ffi::c_int as uint8_t,
    158 as ::core::ffi::c_int as uint8_t,
    159 as ::core::ffi::c_int as uint8_t,
    160 as ::core::ffi::c_int as uint8_t,
    161 as ::core::ffi::c_int as uint8_t,
    162 as ::core::ffi::c_int as uint8_t,
    163 as ::core::ffi::c_int as uint8_t,
    164 as ::core::ffi::c_int as uint8_t,
    165 as ::core::ffi::c_int as uint8_t,
    166 as ::core::ffi::c_int as uint8_t,
    167 as ::core::ffi::c_int as uint8_t,
    168 as ::core::ffi::c_int as uint8_t,
    169 as ::core::ffi::c_int as uint8_t,
    170 as ::core::ffi::c_int as uint8_t,
    171 as ::core::ffi::c_int as uint8_t,
    172 as ::core::ffi::c_int as uint8_t,
    173 as ::core::ffi::c_int as uint8_t,
    174 as ::core::ffi::c_int as uint8_t,
    175 as ::core::ffi::c_int as uint8_t,
    176 as ::core::ffi::c_int as uint8_t,
    177 as ::core::ffi::c_int as uint8_t,
    178 as ::core::ffi::c_int as uint8_t,
    179 as ::core::ffi::c_int as uint8_t,
    180 as ::core::ffi::c_int as uint8_t,
    181 as ::core::ffi::c_int as uint8_t,
    182 as ::core::ffi::c_int as uint8_t,
    183 as ::core::ffi::c_int as uint8_t,
    184 as ::core::ffi::c_int as uint8_t,
    185 as ::core::ffi::c_int as uint8_t,
    186 as ::core::ffi::c_int as uint8_t,
    187 as ::core::ffi::c_int as uint8_t,
    188 as ::core::ffi::c_int as uint8_t,
    189 as ::core::ffi::c_int as uint8_t,
    190 as ::core::ffi::c_int as uint8_t,
    191 as ::core::ffi::c_int as uint8_t,
    192 as ::core::ffi::c_int as uint8_t,
    193 as ::core::ffi::c_int as uint8_t,
    194 as ::core::ffi::c_int as uint8_t,
    195 as ::core::ffi::c_int as uint8_t,
    196 as ::core::ffi::c_int as uint8_t,
    197 as ::core::ffi::c_int as uint8_t,
    198 as ::core::ffi::c_int as uint8_t,
    199 as ::core::ffi::c_int as uint8_t,
    200 as ::core::ffi::c_int as uint8_t,
    201 as ::core::ffi::c_int as uint8_t,
    202 as ::core::ffi::c_int as uint8_t,
    203 as ::core::ffi::c_int as uint8_t,
    204 as ::core::ffi::c_int as uint8_t,
    205 as ::core::ffi::c_int as uint8_t,
    206 as ::core::ffi::c_int as uint8_t,
    207 as ::core::ffi::c_int as uint8_t,
    208 as ::core::ffi::c_int as uint8_t,
    209 as ::core::ffi::c_int as uint8_t,
    210 as ::core::ffi::c_int as uint8_t,
    211 as ::core::ffi::c_int as uint8_t,
    212 as ::core::ffi::c_int as uint8_t,
    213 as ::core::ffi::c_int as uint8_t,
    214 as ::core::ffi::c_int as uint8_t,
    215 as ::core::ffi::c_int as uint8_t,
    216 as ::core::ffi::c_int as uint8_t,
    217 as ::core::ffi::c_int as uint8_t,
    218 as ::core::ffi::c_int as uint8_t,
    219 as ::core::ffi::c_int as uint8_t,
    220 as ::core::ffi::c_int as uint8_t,
    221 as ::core::ffi::c_int as uint8_t,
    222 as ::core::ffi::c_int as uint8_t,
    223 as ::core::ffi::c_int as uint8_t,
    224 as ::core::ffi::c_int as uint8_t,
    225 as ::core::ffi::c_int as uint8_t,
    226 as ::core::ffi::c_int as uint8_t,
    227 as ::core::ffi::c_int as uint8_t,
    228 as ::core::ffi::c_int as uint8_t,
    229 as ::core::ffi::c_int as uint8_t,
    230 as ::core::ffi::c_int as uint8_t,
    231 as ::core::ffi::c_int as uint8_t,
    232 as ::core::ffi::c_int as uint8_t,
    233 as ::core::ffi::c_int as uint8_t,
    234 as ::core::ffi::c_int as uint8_t,
    235 as ::core::ffi::c_int as uint8_t,
    236 as ::core::ffi::c_int as uint8_t,
    237 as ::core::ffi::c_int as uint8_t,
    238 as ::core::ffi::c_int as uint8_t,
    239 as ::core::ffi::c_int as uint8_t,
    240 as ::core::ffi::c_int as uint8_t,
    241 as ::core::ffi::c_int as uint8_t,
    242 as ::core::ffi::c_int as uint8_t,
    243 as ::core::ffi::c_int as uint8_t,
    244 as ::core::ffi::c_int as uint8_t,
    245 as ::core::ffi::c_int as uint8_t,
    246 as ::core::ffi::c_int as uint8_t,
    247 as ::core::ffi::c_int as uint8_t,
    248 as ::core::ffi::c_int as uint8_t,
    249 as ::core::ffi::c_int as uint8_t,
    250 as ::core::ffi::c_int as uint8_t,
    251 as ::core::ffi::c_int as uint8_t,
    252 as ::core::ffi::c_int as uint8_t,
    253 as ::core::ffi::c_int as uint8_t,
    254 as ::core::ffi::c_int as uint8_t,
    255 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x3e as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x3 as ::core::ffi::c_int as uint8_t,
    0x7e as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x7e as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x3 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x7 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x7 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x3 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x87 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x7 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x7f as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0x7f as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xfe as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xfc as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xf8 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x78 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0xff as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x80 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x1 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0x18 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0x12 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x10 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0x16 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
    0 as ::core::ffi::c_int as uint8_t,
];
