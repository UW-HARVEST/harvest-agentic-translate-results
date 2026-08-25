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
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub const PCRE2_ERROR_BADDATA: ::core::ffi::c_int = -(29 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
    use super::stdint_uintn_h::uint8_t;
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
pub mod pcre2_internal_h {
    pub const COMPILE_ERROR_BASE: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
    pub const CHAR_NUL: ::core::ffi::c_int = '\0' as i32;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::{PCRE2_ERROR_BADDATA, PCRE2_ERROR_NOMEMORY, PCRE2_UCHAR8};
pub use self::pcre2_internal_h::{CHAR_NUL, COMPILE_ERROR_BASE};
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
static mut compile_error_texts: [::core::ffi::c_uchar; 5687] = unsafe {
    ::core::mem::transmute::<
        [u8; 5687],
        [::core::ffi::c_uchar; 5687],
    >(
        *b"no error\0\\ at end of pattern\0\\c at end of pattern\0unrecognized character follows \\\0numbers out of order in {} quantifier\0number too big in {} quantifier\0missing terminating ] for character class\0escape sequence is invalid in character class\0range out of order in character class\0quantifier does not follow a repeatable item\0internal error: unexpected repeat\0unrecognized character after (? or (?-\0POSIX named classes are supported only within a class\0POSIX collating elements are not supported\0missing closing parenthesis\0reference to non-existent subpattern\0pattern passed as NULL with non-zero length\0unrecognised compile-time option bit(s)\0missing ) after (?# comment\0parentheses are too deeply nested\0regular expression is too large\0failed to allocate heap memory\0unmatched closing parenthesis\0internal error: code overflow\0missing closing parenthesis for condition\0length of lookbehind assertion is not limited\0a relative value of zero is not allowed\0conditional subpattern contains more than two branches\0atomic assertion expected after (?( or (?(?C)\0digit expected after (?+\0unknown POSIX class name\0internal error in pcre2_study(): should not occur\0this version of PCRE2 does not have Unicode support\0parentheses are too deeply nested (stack check)\0character code point value in \\x{} or \\o{} is too large\0lookbehind is too complicated\0\\C is not allowed in a lookbehind assertion in UTF-8 mode\0PCRE2 does not support \\F, \\L, \\l, \\N{name}, \\U, or \\u\0number after (?C is greater than 255\0closing parenthesis for (?C expected\0invalid escape sequence in (*VERB) name\0unrecognized character after (?P\0syntax error in subpattern name (missing terminator?)\0two named subpatterns have the same name (PCRE2_DUPNAMES not set)\0subpattern name must start with a non-digit\0this version of PCRE2 does not have support for \\P, \\p, or \\X\0malformed \\P or \\p sequence\0unknown property after \\P or \\p\0subpattern name is too long (maximum 128 code units)\0too many named subpatterns (maximum 10000)\0invalid range in character class\0octal value is greater than \\377 in 8-bit non-UTF-8 mode\0internal error: overran compiling workspace\0internal error: previously-checked referenced subpattern not found\0DEFINE subpattern contains more than one branch\0missing opening brace after \\o\0internal error: unknown newline setting\0\\g is not followed by a braced, angle-bracketed, or quoted name/number or by a plain number\0(?R (recursive pattern call) must be followed by a closing parenthesis\0obsolete error (should not occur)\0(*VERB) not recognized or malformed\0subpattern number is too big\0subpattern name expected\0internal error: parsed pattern overflow\0non-octal character in \\o{} (closing brace missing?)\0different names for subpatterns of the same number are not allowed\0(*MARK) must have an argument\0non-hex character in \\x{} (closing brace missing?)\0\\c must be followed by a printable ASCII character\0\\k is not followed by a braced, angle-bracketed, or quoted name\0internal error: unknown meta code in check_lookbehinds()\0\\N is not supported in a class\0callout string is too long\0disallowed Unicode code point (>= 0xd800 && <= 0xdfff)\0using UTF is disabled by the application\0using UCP is disabled by the application\0name is too long in (*MARK), (*PRUNE), (*SKIP), or (*THEN)\0character code point value in \\u.... sequence is too large\0digits missing after \\x or in \\x{} or \\o{} or \\N{U+}\0syntax error or number too big in (?(VERSION condition\0internal error: unknown opcode in auto_possessify()\0missing terminating delimiter for callout with string argument\0unrecognized string delimiter follows (?C\0using \\C is disabled by the application\0(?| and/or (?J: or (?x: parentheses are too deeply nested\0using \\C is disabled in this PCRE2 library\0regular expression is too complicated\0lookbehind assertion is too long\0pattern string is longer than the limit set by the application\0internal error: unknown code in parsed pattern\0internal error: bad code value in parsed_skip()\0PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES is not allowed in UTF-16 mode\0invalid option bits with PCRE2_LITERAL\0\\N{U+dddd} is supported only in Unicode (UTF) mode\0invalid hyphen in option setting\0(*alpha_assertion) not recognized\0script runs require Unicode support, which this version of PCRE2 does not have\0too many capturing groups (maximum 65535)\0octal digit missing after \\0 (PCRE2_EXTRA_NO_BS0 is set)\0\\K is not allowed in lookarounds (but see PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK)\0branch too long in variable-length lookbehind assertion\0compiled pattern would be longer than the limit set by the application\0octal value given by \\ddd is greater than \\377 (forbidden by PCRE2_EXTRA_PYTHON_OCTAL)\0using callouts is disabled by the application\0PCRE2_EXTRA_TURKISH_CASING require Unicode (UTF or UCP) mode\0PCRE2_EXTRA_TURKISH_CASING requires UTF in 8-bit mode\0PCRE2_EXTRA_TURKISH_CASING and PCRE2_EXTRA_CASELESS_RESTRICT are not compatible\0extended character class nesting is too deep\0invalid operator in extended character class\0unexpected operator in extended character class (no preceding operand)\0expected operand after operator in extended character class\0square brackets needed to clarify operator precedence in extended character class\0missing terminating ] for extended character class (note '[' must be escaped under PCRE2_ALT_EXTENDED_CLASS)\0unexpected expression in extended character class (no preceding operator)\0empty expression in extended character class\0terminating ] with no following closing parenthesis in (?[...]\0unexpected character in (?[...]) extended character class\0expected capture group number or name\0missing opening parenthesis\0syntax error in subpattern number (missing terminator?)\0erroroffset passed as NULL\0\0",
    )
};
static mut match_error_texts: [::core::ffi::c_uchar; 2946] = unsafe {
    ::core::mem::transmute::<
        [u8; 2946],
        [::core::ffi::c_uchar; 2946],
    >(
        *b"no error\0no match\0partial match\0UTF-8 error: 1 byte missing at end\0UTF-8 error: 2 bytes missing at end\0UTF-8 error: 3 bytes missing at end\0UTF-8 error: 4 bytes missing at end\0UTF-8 error: 5 bytes missing at end\0UTF-8 error: byte 2 top bits not 0x80\0UTF-8 error: byte 3 top bits not 0x80\0UTF-8 error: byte 4 top bits not 0x80\0UTF-8 error: byte 5 top bits not 0x80\0UTF-8 error: byte 6 top bits not 0x80\0UTF-8 error: 5-byte character is not allowed (RFC 3629)\0UTF-8 error: 6-byte character is not allowed (RFC 3629)\0UTF-8 error: code points greater than 0x10ffff are not defined\0UTF-8 error: code points 0xd800-0xdfff are not defined\0UTF-8 error: overlong 2-byte sequence\0UTF-8 error: overlong 3-byte sequence\0UTF-8 error: overlong 4-byte sequence\0UTF-8 error: overlong 5-byte sequence\0UTF-8 error: overlong 6-byte sequence\0UTF-8 error: isolated byte with 0x80 bit set\0UTF-8 error: illegal byte (0xfe or 0xff)\0UTF-16 error: missing low surrogate at end\0UTF-16 error: invalid low surrogate\0UTF-16 error: isolated low surrogate\0UTF-32 error: code points 0xd800-0xdfff are not defined\0UTF-32 error: code points greater than 0x10ffff are not defined\0bad data value\0patterns do not all use the same character tables\0magic number missing\0pattern compiled in wrong mode: 8/16/32-bit error\0bad offset value\0bad option value\0invalid replacement string\0bad offset into UTF string\0callout error code\0invalid data in workspace for DFA restart\0too much recursion for DFA matching\0backreference condition or recursion test is not supported for DFA matching\0function is not supported for DFA matching\0pattern contains an item that is not supported for DFA matching\0workspace size exceeded in DFA matching\0internal error - pattern overwritten?\0bad JIT option\0JIT stack limit reached\0match limit exceeded\0no more memory\0unknown substring\0non-unique substring name\0NULL argument passed with non-zero length\0nested recursion at the same subject position\0matching depth limit exceeded\0requested value is not available\0requested value is not set\0offset limit set without PCRE2_USE_OFFSET_LIMIT\0bad escape sequence in replacement string\0expected closing curly bracket in replacement string\0bad substitution in replacement string\0match with end before start or start moved backwards is not supported\0too many replacements (more than INT_MAX)\0bad serialized data\0heap limit exceeded\0invalid syntax\0internal error: duplicate substitution match\0PCRE2_MATCH_INVALID_UTF is not supported for DFA matching\0internal error: invalid substring offset\0feature is not supported by the JIT compiler\0error performing replacement case transformation\0replacement too large (longer than PCRE2_SIZE)\0substitute pattern differs from prior match call\0substitute subject differs from prior match call\0substitute start offset differs from prior match call\0substitute options differ from prior match call\0disallowed use of \\K in lookaround\0replacement $' or $_ not supported with partial match\0\0",
    )
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_get_error_message_8(
    mut enumber: ::core::ffi::c_int,
    mut buffer: *mut PCRE2_UCHAR8,
    mut size: size_t,
) -> ::core::ffi::c_int {
    let mut message: *const ::core::ffi::c_uchar = ::core::ptr::null::<::core::ffi::c_uchar>();
    let mut i: size_t = 0;
    let mut n: ::core::ffi::c_int = 0;
    let mut rc: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if size == 0 as size_t {
        return PCRE2_ERROR_NOMEMORY;
    }
    if enumber >= COMPILE_ERROR_BASE {
        message = &raw const compile_error_texts as *const ::core::ffi::c_uchar;
        n = enumber - COMPILE_ERROR_BASE;
    } else if enumber < 0 as ::core::ffi::c_int {
        message = &raw const match_error_texts as *const ::core::ffi::c_uchar;
        n = -enumber;
    } else {
        message = b"\0\0" as *const u8 as *const ::core::ffi::c_char as *const ::core::ffi::c_uchar;
        n = 1 as ::core::ffi::c_int;
    }
    while n > 0 as ::core::ffi::c_int {
        loop {
            let fresh6 = message;
            message = message.offset(1);
            if !(*fresh6 as ::core::ffi::c_int != CHAR_NUL) {
                break;
            }
        }
        if *message as ::core::ffi::c_int == CHAR_NUL {
            return PCRE2_ERROR_BADDATA;
        }
        n -= 1;
    }
    i = 0 as size_t;
    while *message as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        if i >= size.wrapping_sub(1 as size_t) {
            rc = PCRE2_ERROR_NOMEMORY;
            break;
        } else {
            let fresh7 = message;
            message = message.offset(1);
            *buffer.offset(i as isize) = *fresh7 as PCRE2_UCHAR8;
            i = i.wrapping_add(1);
        }
    }
    *buffer.offset(i as isize) = 0 as PCRE2_UCHAR8;
    return if rc != 0 { rc } else { i as ::core::ffi::c_int };
}
