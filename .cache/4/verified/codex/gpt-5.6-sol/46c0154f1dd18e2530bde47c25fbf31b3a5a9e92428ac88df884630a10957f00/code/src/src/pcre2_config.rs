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
    pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
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
    pub type uint32_t = __uint32_t;
    use super::types_h::{__uint32_t, __uint8_t};
}
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub const PCRE2_BSR_UNICODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
    pub const PCRE2_CONFIG_BSR: uint32_t = 0 as uint32_t;
    pub const PCRE2_CONFIG_JIT: uint32_t = 1 as uint32_t;
    pub const PCRE2_CONFIG_JITTARGET: uint32_t = 2 as uint32_t;
    pub const PCRE2_CONFIG_LINKSIZE: uint32_t = 3 as uint32_t;
    pub const PCRE2_CONFIG_MATCHLIMIT: uint32_t = 4 as uint32_t;
    pub const PCRE2_CONFIG_NEWLINE: uint32_t = 5 as uint32_t;
    pub const PCRE2_CONFIG_PARENSLIMIT: uint32_t = 6 as uint32_t;
    pub const PCRE2_CONFIG_DEPTHLIMIT: uint32_t = 7 as uint32_t;
    pub const PCRE2_CONFIG_STACKRECURSE: uint32_t = 8 as uint32_t;
    pub const PCRE2_CONFIG_UNICODE: uint32_t = 9 as uint32_t;
    pub const PCRE2_CONFIG_UNICODE_VERSION: uint32_t = 10 as uint32_t;
    pub const PCRE2_CONFIG_VERSION: uint32_t = 11 as uint32_t;
    pub const PCRE2_CONFIG_HEAPLIMIT: uint32_t = 12 as uint32_t;
    pub const PCRE2_CONFIG_NEVER_BACKSLASH_C: uint32_t = 13 as uint32_t;
    pub const PCRE2_CONFIG_COMPILED_WIDTHS: uint32_t = 14 as uint32_t;
    pub const PCRE2_CONFIG_TABLES_LENGTH: uint32_t = 15 as uint32_t;
    pub const PCRE2_CONFIG_EFFECTIVE_LINKSIZE: uint32_t = 16 as uint32_t;
    use super::stdint_uintn_h::{uint32_t, uint8_t};
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
pub mod string_h {
    use super::stddef_h::size_t;
    extern "C" {
        pub fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    }
}
pub mod pcre2_internal_h {
    pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
    pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
    pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
    pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
    use super::pcre2_h::PCRE2_UCHAR8;
    use super::stddef_h::size_t;
    extern "C" {
        pub static mut _pcre2_unicode_version_8: *const ::core::ffi::c_char;
        pub fn _pcre2_strcpy_c8_8(_: *mut PCRE2_UCHAR8, _: *const ::core::ffi::c_char) -> size_t;
    }
}
pub mod config_h {
    pub const HEAP_LIMIT: ::core::ffi::c_int = 20000000 as ::core::ffi::c_int;
    pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    pub const MATCH_LIMIT: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
    pub const MATCH_LIMIT_DEPTH: ::core::ffi::c_int = MATCH_LIMIT;
    pub const NEWLINE_DEFAULT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    pub const PARENS_NEST_LIMIT: ::core::ffi::c_int = 250 as ::core::ffi::c_int;
}
pub mod pcre2_intmodedep_h {
    pub const CONFIGURED_LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::config_h::{
    HEAP_LIMIT, LINK_SIZE, MATCH_LIMIT, MATCH_LIMIT_DEPTH, NEWLINE_DEFAULT, PARENS_NEST_LIMIT,
};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::{
    PCRE2_BSR_UNICODE, PCRE2_CONFIG_BSR, PCRE2_CONFIG_COMPILED_WIDTHS, PCRE2_CONFIG_DEPTHLIMIT,
    PCRE2_CONFIG_EFFECTIVE_LINKSIZE, PCRE2_CONFIG_HEAPLIMIT, PCRE2_CONFIG_JIT,
    PCRE2_CONFIG_JITTARGET, PCRE2_CONFIG_LINKSIZE, PCRE2_CONFIG_MATCHLIMIT,
    PCRE2_CONFIG_NEVER_BACKSLASH_C, PCRE2_CONFIG_NEWLINE, PCRE2_CONFIG_PARENSLIMIT,
    PCRE2_CONFIG_STACKRECURSE, PCRE2_CONFIG_TABLES_LENGTH, PCRE2_CONFIG_UNICODE,
    PCRE2_CONFIG_UNICODE_VERSION, PCRE2_CONFIG_VERSION, PCRE2_ERROR_BADOPTION, PCRE2_UCHAR8,
};
pub use self::pcre2_internal_h::{
    _pcre2_strcpy_c8_8, _pcre2_unicode_version_8, cbit_length, cbits_offset, ctypes_offset,
    TABLES_LENGTH,
};
pub use self::pcre2_intmodedep_h::CONFIGURED_LINK_SIZE;
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_uintn_h::{uint32_t, uint8_t};
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, strtod, strtol, strtoll};
use self::string_h::strlen;
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_config_8(
    mut what: uint32_t,
    mut where_0: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if where_0.is_null() {
        match what {
            0 | 14 | 7 | 16 | 12 | 1 | 3 | 4 | 13 | 5 | 6 | 8 | 15 | 9 => {
                return ::core::mem::size_of::<uint32_t>() as ::core::ffi::c_int;
            }
            2 | 10 | 11 => {}
            _ => return PCRE2_ERROR_BADOPTION,
        }
    }
    match what {
        0 => {
            *(where_0 as *mut uint32_t) = PCRE2_BSR_UNICODE as uint32_t;
        }
        14 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        7 => {
            *(where_0 as *mut uint32_t) = MATCH_LIMIT_DEPTH as uint32_t;
        }
        16 => {
            *(where_0 as *mut uint32_t) = (LINK_SIZE as usize)
                .wrapping_mul(::core::mem::size_of::<PCRE2_UCHAR8>() as usize)
                as uint32_t;
        }
        12 => {
            *(where_0 as *mut uint32_t) = HEAP_LIMIT as uint32_t;
        }
        1 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        2 => return PCRE2_ERROR_BADOPTION,
        3 => {
            *(where_0 as *mut uint32_t) = CONFIGURED_LINK_SIZE as uint32_t;
        }
        4 => {
            *(where_0 as *mut uint32_t) = MATCH_LIMIT as uint32_t;
        }
        5 => {
            *(where_0 as *mut uint32_t) = NEWLINE_DEFAULT as uint32_t;
        }
        13 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        6 => {
            *(where_0 as *mut uint32_t) = PARENS_NEST_LIMIT as uint32_t;
        }
        8 => {
            *(where_0 as *mut uint32_t) = 0 as uint32_t;
        }
        15 => {
            *(where_0 as *mut uint32_t) = TABLES_LENGTH as uint32_t;
        }
        10 => {
            let mut v: *const ::core::ffi::c_char = _pcre2_unicode_version_8;
            return (1 as size_t).wrapping_add(
                (if where_0.is_null() {
                    strlen(v)
                } else {
                    _pcre2_strcpy_c8_8(where_0 as *mut PCRE2_UCHAR8, v)
                }),
            ) as ::core::ffi::c_int;
        }
        9 => {
            *(where_0 as *mut uint32_t) = 1 as uint32_t;
        }
        11 => {
            let mut v_0: *const ::core::ffi::c_char =
                if ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"Z -DEV\0")
                    [1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    b"10.48 2025-10-21\0" as *const u8 as *const ::core::ffi::c_char
                } else {
                    b"10.48-DEV 2025-10-21\0" as *const u8 as *const ::core::ffi::c_char
                };
            return (1 as size_t).wrapping_add(
                (if where_0.is_null() {
                    strlen(v_0)
                } else {
                    _pcre2_strcpy_c8_8(where_0 as *mut PCRE2_UCHAR8, v_0)
                }),
            ) as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADOPTION,
    }
    return 0 as ::core::ffi::c_int;
}
