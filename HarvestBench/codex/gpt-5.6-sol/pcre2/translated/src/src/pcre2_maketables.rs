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
pub mod ctype_h {
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
        pub fn __ctype_b_loc() -> *mut *const ::core::ffi::c_ushort;
        pub fn __ctype_tolower_loc() -> *mut *const __int32_t;
        pub fn __ctype_toupper_loc() -> *mut *const __int32_t;
    }
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
    use super::stddef_h::{size_t, NULL};
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
        pub fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
        pub fn free(__ptr: *mut ::core::ffi::c_void);
    }
}
pub mod stdint_uintn_h {
    pub type uint8_t = __uint8_t;
    use super::types_h::__uint8_t;
}
pub mod pcre2_intmodedep_h {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_general_context_8 {
        pub memctl: pcre2_memctl,
    }
    use super::pcre2_internal_h::pcre2_memctl;
}
pub mod pcre2_internal_h {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_memctl {
        pub malloc: Option<
            unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
        >,
        pub free:
            Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>,
        pub memory_data: *mut ::core::ffi::c_void,
    }
    pub const cbit_space: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    pub const cbit_xdigit: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
    pub const cbit_digit: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
    pub const cbit_upper: ::core::ffi::c_int = 96 as ::core::ffi::c_int;
    pub const cbit_lower: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
    pub const cbit_word: ::core::ffi::c_int = 160 as ::core::ffi::c_int;
    pub const cbit_graph: ::core::ffi::c_int = 192 as ::core::ffi::c_int;
    pub const cbit_print: ::core::ffi::c_int = 224 as ::core::ffi::c_int;
    pub const cbit_punct: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
    pub const cbit_cntrl: ::core::ffi::c_int = 288 as ::core::ffi::c_int;
    pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
    pub const ctype_space: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
    pub const ctype_letter: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
    pub const ctype_lcletter: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
    pub const ctype_digit: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
    pub const ctype_word: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
    pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
    pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
    pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
    pub const CHAR_UNDERSCORE: ::core::ffi::c_int = '_' as i32;
    use super::stddef_h::size_t;
}
pub mod pcre2_h {
    pub type pcre2_general_context_8 = pcre2_real_general_context_8;
    use super::pcre2_intmodedep_h::pcre2_real_general_context_8;
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
        pub fn memset(
            __s: *mut ::core::ffi::c_void,
            __c: ::core::ffi::c_int,
            __n: size_t,
        ) -> *mut ::core::ffi::c_void;
    }
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::ctype_h::{
    _ISalnum, _ISalpha, _ISblank, _IScntrl, _ISdigit, _ISgraph, _ISlower, _ISprint, _ISpunct,
    _ISspace, _ISupper, _ISxdigit, __ctype_b_loc, __ctype_tolower_loc, __ctype_toupper_loc,
    tolower, toupper, C2RustUnnamed,
};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::pcre2_general_context_8;
pub use self::pcre2_internal_h::{
    cbit_cntrl, cbit_digit, cbit_graph, cbit_length, cbit_lower, cbit_print, cbit_punct,
    cbit_space, cbit_upper, cbit_word, cbit_xdigit, cbits_offset, ctype_digit, ctype_lcletter,
    ctype_letter, ctype_space, ctype_word, ctypes_offset, pcre2_memctl, CHAR_UNDERSCORE,
    TABLES_LENGTH,
};
pub use self::pcre2_intmodedep_h::pcre2_real_general_context_8;
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_uintn_h::uint8_t;
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, free, malloc, strtod, strtol, strtoll};
use self::string_h::memset;
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *const uint8_t {
    let mut yield_0: *mut uint8_t = (if !gcontext.is_null() {
        (*gcontext)
            .memctl
            .malloc
            .expect("non-null function pointer")(
            TABLES_LENGTH as size_t,
            (*gcontext).memctl.memory_data,
        )
    } else {
        malloc(TABLES_LENGTH as size_t)
    }) as *mut uint8_t;
    let mut i: ::core::ffi::c_int = 0;
    let mut p: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    if yield_0.is_null() {
        return ::core::ptr::null::<uint8_t>();
    }
    p = yield_0;
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut c: ::core::ffi::c_int = ({
            let mut __res: ::core::ffi::c_int = 0;
            if ::core::mem::size_of::<::core::ffi::c_int>() as usize > 1 as usize {
                if 0 != 0 {
                    let mut __c: ::core::ffi::c_int = i;
                    __res =
                        (if __c < -(128 as ::core::ffi::c_int) || __c > 255 as ::core::ffi::c_int {
                            __c as __int32_t
                        } else {
                            *(*__ctype_tolower_loc()).offset(__c as isize)
                        }) as ::core::ffi::c_int;
                } else {
                    __res = tolower(i);
                }
            } else {
                __res = *(*__ctype_tolower_loc()).offset(i as isize) as ::core::ffi::c_int;
            }
            __res
        });
        let fresh6 = p;
        p = p.offset(1);
        *fresh6 = (if c < 256 as ::core::ffi::c_int { c } else { i }) as uint8_t;
        i += 1;
    }
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut c_0: ::core::ffi::c_int = if *(*__ctype_b_loc()).offset(i as isize)
            as ::core::ffi::c_int
            & _ISlower as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            ({
                let mut __res: ::core::ffi::c_int = 0;
                if ::core::mem::size_of::<::core::ffi::c_int>() as usize > 1 as usize {
                    if 0 != 0 {
                        let mut __c: ::core::ffi::c_int = i;
                        __res = (if __c < -(128 as ::core::ffi::c_int)
                            || __c > 255 as ::core::ffi::c_int
                        {
                            __c as __int32_t
                        } else {
                            *(*__ctype_toupper_loc()).offset(__c as isize)
                        }) as ::core::ffi::c_int;
                    } else {
                        __res = toupper(i);
                    }
                } else {
                    __res = *(*__ctype_toupper_loc()).offset(i as isize) as ::core::ffi::c_int;
                }
                __res
            })
        } else {
            ({
                let mut __res: ::core::ffi::c_int = 0;
                if ::core::mem::size_of::<::core::ffi::c_int>() as usize > 1 as usize {
                    if 0 != 0 {
                        let mut __c: ::core::ffi::c_int = i;
                        __res = (if __c < -(128 as ::core::ffi::c_int)
                            || __c > 255 as ::core::ffi::c_int
                        {
                            __c as __int32_t
                        } else {
                            *(*__ctype_tolower_loc()).offset(__c as isize)
                        }) as ::core::ffi::c_int;
                    } else {
                        __res = tolower(i);
                    }
                } else {
                    __res = *(*__ctype_tolower_loc()).offset(i as isize) as ::core::ffi::c_int;
                }
                __res
            })
        };
        let fresh7 = p;
        p = p.offset(1);
        *fresh7 = (if c_0 < 256 as ::core::ffi::c_int {
            c_0
        } else {
            i
        }) as uint8_t;
        i += 1;
    }
    memset(
        p as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        cbit_length as size_t,
    );
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh8 = *p.offset((cbit_digit + i / 8 as ::core::ffi::c_int) as isize);
            *fresh8 = (*fresh8 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISupper as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh9 = *p.offset((cbit_upper + i / 8 as ::core::ffi::c_int) as isize);
            *fresh9 = (*fresh9 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISlower as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh10 = *p.offset((cbit_lower + i / 8 as ::core::ffi::c_int) as isize);
            *fresh10 = (*fresh10 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISalnum as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh11 = *p.offset((cbit_word + i / 8 as ::core::ffi::c_int) as isize);
            *fresh11 = (*fresh11 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if i == CHAR_UNDERSCORE {
            let ref mut fresh12 = *p.offset((cbit_word + i / 8 as ::core::ffi::c_int) as isize);
            *fresh12 = (*fresh12 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISspace as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh13 = *p.offset((cbit_space + i / 8 as ::core::ffi::c_int) as isize);
            *fresh13 = (*fresh13 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISxdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh14 = *p.offset((cbit_xdigit + i / 8 as ::core::ffi::c_int) as isize);
            *fresh14 = (*fresh14 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISgraph as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh15 = *p.offset((cbit_graph + i / 8 as ::core::ffi::c_int) as isize);
            *fresh15 = (*fresh15 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISprint as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh16 = *p.offset((cbit_print + i / 8 as ::core::ffi::c_int) as isize);
            *fresh16 = (*fresh16 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISpunct as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh17 = *p.offset((cbit_punct + i / 8 as ::core::ffi::c_int) as isize);
            *fresh17 = (*fresh17 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _IScntrl as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            let ref mut fresh18 = *p.offset((cbit_cntrl + i / 8 as ::core::ffi::c_int) as isize);
            *fresh18 = (*fresh18 as ::core::ffi::c_uint
                | (1 as ::core::ffi::c_uint) << (i & 7 as ::core::ffi::c_int))
                as uint8_t;
        }
        i += 1;
    }
    p = p.offset(cbit_length as isize);
    i = 0 as ::core::ffi::c_int;
    while i < 256 as ::core::ffi::c_int {
        let mut x: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISspace as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            x += ctype_space;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISalpha as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            x += ctype_letter;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISlower as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            x += ctype_lcletter;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISdigit as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
        {
            x += ctype_digit;
        }
        if *(*__ctype_b_loc()).offset(i as isize) as ::core::ffi::c_int
            & _ISalnum as ::core::ffi::c_int as ::core::ffi::c_ushort as ::core::ffi::c_int
            != 0
            || i == CHAR_UNDERSCORE
        {
            x += ctype_word;
        }
        let fresh19 = p;
        p = p.offset(1);
        *fresh19 = x as uint8_t;
        i += 1;
    }
    return yield_0;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_maketables_free_8(
    mut gcontext: *mut pcre2_general_context_8,
    mut tables: *const uint8_t,
) {
    if !gcontext.is_null() {
        (*gcontext).memctl.free.expect("non-null function pointer")(
            tables as *mut ::core::ffi::c_void,
            (*gcontext).memctl.memory_data,
        );
    } else {
        free(tables as *mut ::core::ffi::c_void);
    };
}
