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
    pub type uint16_t = __uint16_t;
    pub type uint32_t = __uint32_t;
    use super::types_h::{__uint16_t, __uint32_t, __uint8_t};
}
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
    pub type pcre2_general_context_8 = pcre2_real_general_context_8;
    pub type pcre2_compile_context_8 = pcre2_real_compile_context_8;
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_substitute_callout_block_8 {
        pub version: uint32_t,
        pub input: PCRE2_SPTR8,
        pub output: PCRE2_SPTR8,
        pub output_offsets: [size_t; 2],
        pub ovector: *mut size_t,
        pub oveccount: uint32_t,
        pub subscount: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_callout_block_8 {
        pub version: uint32_t,
        pub callout_number: uint32_t,
        pub capture_top: uint32_t,
        pub capture_last: uint32_t,
        pub offset_vector: *mut size_t,
        pub mark: PCRE2_SPTR8,
        pub subject: PCRE2_SPTR8,
        pub subject_length: size_t,
        pub start_match: size_t,
        pub current_position: size_t,
        pub pattern_position: size_t,
        pub next_item_length: size_t,
        pub callout_string_offset: size_t,
        pub callout_string_length: size_t,
        pub callout_string: PCRE2_SPTR8,
        pub callout_flags: uint32_t,
    }
    pub type pcre2_match_context_8 = pcre2_real_match_context_8;
    pub type pcre2_convert_context_8 = pcre2_real_convert_context_8;
    pub const PCRE2_NEWLINE_CR: uint32_t = 1 as uint32_t;
    pub const PCRE2_NEWLINE_LF: uint32_t = 2 as uint32_t;
    pub const PCRE2_NEWLINE_CRLF: uint32_t = 3 as uint32_t;
    pub const PCRE2_NEWLINE_ANY: uint32_t = 4 as uint32_t;
    pub const PCRE2_NEWLINE_ANYCRLF: uint32_t = 5 as uint32_t;
    pub const PCRE2_NEWLINE_NUL: uint32_t = 6 as uint32_t;
    pub const PCRE2_BSR_UNICODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    pub const PCRE2_BSR_ANYCRLF: uint32_t = 2 as uint32_t;
    pub const PCRE2_ERROR_BADDATA: ::core::ffi::c_int = -(29 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_BADOPTION: ::core::ffi::c_int = -(34 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
    pub const PCRE2_OPTIMIZATION_NONE: uint32_t = 0 as uint32_t;
    pub const PCRE2_OPTIMIZATION_FULL: uint32_t = 1 as uint32_t;
    pub const PCRE2_AUTO_POSSESS: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
    pub const PCRE2_START_OPTIMIZE_OFF: ::core::ffi::c_int = 69 as ::core::ffi::c_int;
    pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
    use super::pcre2_intmodedep_h::{
        pcre2_real_compile_context_8, pcre2_real_convert_context_8, pcre2_real_general_context_8,
        pcre2_real_match_context_8,
    };
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::{uint32_t, uint8_t};
}
pub mod pcre2_intmodedep_h {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_general_context_8 {
        pub memctl: pcre2_memctl,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_compile_context_8 {
        pub memctl: pcre2_memctl,
        pub stack_guard:
            Option<unsafe extern "C" fn(uint32_t, *mut ::core::ffi::c_void) -> ::core::ffi::c_int>,
        pub stack_guard_data: *mut ::core::ffi::c_void,
        pub tables: *const uint8_t,
        pub max_pattern_length: size_t,
        pub max_pattern_compiled_length: size_t,
        pub bsr_convention: uint16_t,
        pub newline_convention: uint16_t,
        pub parens_nest_limit: uint32_t,
        pub extra_options: uint32_t,
        pub max_varlookbehind: uint32_t,
        pub optimization_flags: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_match_context_8 {
        pub memctl: pcre2_memctl,
        pub callout: Option<
            unsafe extern "C" fn(
                *mut pcre2_callout_block_8,
                *mut ::core::ffi::c_void,
            ) -> ::core::ffi::c_int,
        >,
        pub callout_data: *mut ::core::ffi::c_void,
        pub substitute_callout: Option<
            unsafe extern "C" fn(
                *mut pcre2_substitute_callout_block_8,
                *mut ::core::ffi::c_void,
            ) -> ::core::ffi::c_int,
        >,
        pub substitute_callout_data: *mut ::core::ffi::c_void,
        pub substitute_case_callout: Option<
            unsafe extern "C" fn(
                PCRE2_SPTR8,
                size_t,
                *mut PCRE2_UCHAR8,
                size_t,
                ::core::ffi::c_int,
                *mut ::core::ffi::c_void,
            ) -> size_t,
        >,
        pub substitute_case_callout_data: *mut ::core::ffi::c_void,
        pub offset_limit: size_t,
        pub heap_limit: uint32_t,
        pub match_limit: uint32_t,
        pub depth_limit: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_convert_context_8 {
        pub memctl: pcre2_memctl,
        pub glob_separator: uint32_t,
        pub glob_escape: uint32_t,
    }
    use super::pcre2_h::{
        pcre2_callout_block_8, pcre2_substitute_callout_block_8, PCRE2_SPTR8, PCRE2_UCHAR8,
    };
    use super::pcre2_internal_h::pcre2_memctl;
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
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
    pub const BSR_DEFAULT: ::core::ffi::c_int = PCRE2_BSR_UNICODE;
    pub const PCRE2_OPTIMIZATION_ALL: ::core::ffi::c_uint = 0x7 as ::core::ffi::c_uint;
    pub const CHAR_DOT: ::core::ffi::c_int = '.' as i32;
    pub const CHAR_SLASH: ::core::ffi::c_int = '/' as i32;
    pub const CHAR_BACKSLASH: ::core::ffi::c_int = '\\' as i32;
    use super::pcre2_h::PCRE2_BSR_UNICODE;
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::uint8_t;
    extern "C" {
        pub static _pcre2_default_tables_8: [uint8_t; 0];
    }
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
        pub fn memcpy(
            __dest: *mut ::core::ffi::c_void,
            __src: *const ::core::ffi::c_void,
            __n: size_t,
        ) -> *mut ::core::ffi::c_void;
        pub fn strchr(
            __s: *const ::core::ffi::c_char,
            __c: ::core::ffi::c_int,
        ) -> *mut ::core::ffi::c_char;
    }
}
pub mod config_h {
    pub const HEAP_LIMIT: ::core::ffi::c_int = 20000000 as ::core::ffi::c_int;
    pub const MATCH_LIMIT: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
    pub const MATCH_LIMIT_DEPTH: ::core::ffi::c_int = MATCH_LIMIT;
    pub const MAX_VARLOOKBEHIND: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
    pub const NEWLINE_DEFAULT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    pub const PARENS_NEST_LIMIT: ::core::ffi::c_int = 250 as ::core::ffi::c_int;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::config_h::{
    HEAP_LIMIT, MATCH_LIMIT, MATCH_LIMIT_DEPTH, MAX_VARLOOKBEHIND, NEWLINE_DEFAULT,
    PARENS_NEST_LIMIT,
};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::{
    pcre2_callout_block_8, pcre2_compile_context_8, pcre2_convert_context_8,
    pcre2_general_context_8, pcre2_match_context_8, pcre2_substitute_callout_block_8,
    PCRE2_AUTO_POSSESS, PCRE2_BSR_ANYCRLF, PCRE2_BSR_UNICODE, PCRE2_ERROR_BADDATA,
    PCRE2_ERROR_BADOPTION, PCRE2_ERROR_NULL, PCRE2_NEWLINE_ANY, PCRE2_NEWLINE_ANYCRLF,
    PCRE2_NEWLINE_CR, PCRE2_NEWLINE_CRLF, PCRE2_NEWLINE_LF, PCRE2_NEWLINE_NUL,
    PCRE2_OPTIMIZATION_FULL, PCRE2_OPTIMIZATION_NONE, PCRE2_SPTR8, PCRE2_START_OPTIMIZE_OFF,
    PCRE2_UCHAR8, PCRE2_UNSET,
};
pub use self::pcre2_internal_h::{
    _pcre2_default_tables_8, pcre2_memctl, BSR_DEFAULT, CHAR_BACKSLASH, CHAR_DOT, CHAR_SLASH,
    PCRE2_OPTIMIZATION_ALL,
};
pub use self::pcre2_intmodedep_h::{
    pcre2_real_compile_context_8, pcre2_real_convert_context_8, pcre2_real_general_context_8,
    pcre2_real_match_context_8,
};
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, free, malloc, strtod, strtol, strtoll};
use self::string_h::{memcpy, strchr};
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
unsafe extern "C" fn default_malloc(
    mut size: size_t,
    mut data: *mut ::core::ffi::c_void,
) -> *mut ::core::ffi::c_void {
    return malloc(size);
}
unsafe extern "C" fn default_free(
    mut block: *mut ::core::ffi::c_void,
    mut data: *mut ::core::ffi::c_void,
) {
    free(block);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_memctl_malloc_8(
    mut size: size_t,
    mut memctl: *mut pcre2_memctl,
) -> *mut ::core::ffi::c_void {
    let mut newmemctl: *mut pcre2_memctl = ::core::ptr::null_mut::<pcre2_memctl>();
    let mut yield_0: *mut ::core::ffi::c_void = if memctl.is_null() {
        malloc(size)
    } else {
        (*memctl).malloc.expect("non-null function pointer")(size, (*memctl).memory_data)
    };
    if yield_0.is_null() {
        return NULL_0;
    }
    newmemctl = yield_0 as *mut pcre2_memctl;
    if memctl.is_null() {
        (*newmemctl).malloc = Some(
            default_malloc
                as unsafe extern "C" fn(
                    size_t,
                    *mut ::core::ffi::c_void,
                ) -> *mut ::core::ffi::c_void,
        )
            as Option<
                unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
            >;
        (*newmemctl).free = Some(
            default_free
                as unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
        )
            as Option<
                unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
            >;
        (*newmemctl).memory_data = NULL_0;
    } else {
        *newmemctl = *memctl;
    }
    return yield_0;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_create_8(
    mut private_malloc: Option<
        unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
    >,
    mut private_free: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
    >,
    mut memory_data: *mut ::core::ffi::c_void,
) -> *mut pcre2_general_context_8 {
    let mut gcontext: *mut pcre2_general_context_8 =
        ::core::ptr::null_mut::<pcre2_general_context_8>();
    if private_malloc.is_none() {
        private_malloc = Some(
            default_malloc
                as unsafe extern "C" fn(
                    size_t,
                    *mut ::core::ffi::c_void,
                ) -> *mut ::core::ffi::c_void,
        )
            as Option<
                unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
            >;
    }
    if private_free.is_none() {
        private_free = Some(
            default_free
                as unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
        )
            as Option<
                unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
            >;
    }
    gcontext = private_malloc.expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
        memory_data,
    ) as *mut pcre2_general_context_8;
    if gcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_general_context_8>();
    }
    (*gcontext).memctl.malloc = private_malloc;
    (*gcontext).memctl.free = private_free;
    (*gcontext).memctl.memory_data = memory_data;
    return gcontext;
}
#[unsafe(no_mangle)]
pub static mut _pcre2_default_compile_context_8: pcre2_compile_context_8 = unsafe {
    pcre2_real_compile_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL_0,
        },
        stack_guard: None,
        stack_guard_data: NULL_0,
        tables: &raw const _pcre2_default_tables_8 as *const uint8_t,
        max_pattern_length: PCRE2_UNSET,
        max_pattern_compiled_length: PCRE2_UNSET,
        bsr_convention: BSR_DEFAULT as uint16_t,
        newline_convention: NEWLINE_DEFAULT as uint16_t,
        parens_nest_limit: PARENS_NEST_LIMIT as uint32_t,
        extra_options: 0 as uint32_t,
        max_varlookbehind: MAX_VARLOOKBEHIND as uint32_t,
        optimization_flags: PCRE2_OPTIMIZATION_ALL as uint32_t,
    }
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_compile_context_8 {
    let mut ccontext: *mut pcre2_compile_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_compile_context_8;
    if ccontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_compile_context_8>();
    }
    *ccontext = _pcre2_default_compile_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return ccontext;
}
#[unsafe(no_mangle)]
pub static mut _pcre2_default_match_context_8: pcre2_match_context_8 = unsafe {
    pcre2_real_match_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL_0,
        },
        callout: None,
        callout_data: NULL_0,
        substitute_callout: None,
        substitute_callout_data: NULL_0,
        substitute_case_callout: None,
        substitute_case_callout_data: NULL_0,
        offset_limit: PCRE2_UNSET,
        heap_limit: HEAP_LIMIT as uint32_t,
        match_limit: MATCH_LIMIT as uint32_t,
        depth_limit: MATCH_LIMIT_DEPTH as uint32_t,
    }
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_match_context_8 {
    let mut mcontext: *mut pcre2_match_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_match_context_8;
    if mcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_context_8>();
    }
    *mcontext = _pcre2_default_match_context_8;
    if !gcontext.is_null() {
        *(mcontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return mcontext;
}
#[unsafe(no_mangle)]
pub static mut _pcre2_default_convert_context_8: pcre2_convert_context_8 = unsafe {
    pcre2_real_convert_context_8 {
        memctl: pcre2_memctl {
            malloc: Some(
                default_malloc
                    as unsafe extern "C" fn(
                        size_t,
                        *mut ::core::ffi::c_void,
                    ) -> *mut ::core::ffi::c_void,
            ),
            free: Some(
                default_free
                    as unsafe extern "C" fn(
                        *mut ::core::ffi::c_void,
                        *mut ::core::ffi::c_void,
                    ) -> (),
            ),
            memory_data: NULL_0,
        },
        glob_separator: CHAR_SLASH as uint32_t,
        glob_escape: CHAR_BACKSLASH as uint32_t,
    }
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_create_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_convert_context_8 {
    let mut ccontext: *mut pcre2_convert_context_8 = _pcre2_memctl_malloc_8(
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
        gcontext as *mut pcre2_memctl,
    ) as *mut pcre2_convert_context_8;
    if ccontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_convert_context_8>();
    }
    *ccontext = _pcre2_default_convert_context_8;
    if !gcontext.is_null() {
        *(ccontext as *mut pcre2_memctl) = *(gcontext as *mut pcre2_memctl);
    }
    return ccontext;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_copy_8(
    mut gcontext: *mut pcre2_general_context_8,
) -> *mut pcre2_general_context_8 {
    let mut newcontext: *mut pcre2_general_context_8 = (*gcontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
        (*gcontext).memctl.memory_data,
    ) as *mut pcre2_general_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_general_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        gcontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_general_context_8>() as size_t,
    );
    return newcontext;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_copy_8(
    mut ccontext: *mut pcre2_compile_context_8,
) -> *mut pcre2_compile_context_8 {
    let mut newcontext: *mut pcre2_compile_context_8 = (*ccontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_compile_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_compile_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        ccontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_compile_context_8>() as size_t,
    );
    return newcontext;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_copy_8(
    mut mcontext: *mut pcre2_match_context_8,
) -> *mut pcre2_match_context_8 {
    let mut newcontext: *mut pcre2_match_context_8 = (*mcontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
        (*mcontext).memctl.memory_data,
    ) as *mut pcre2_match_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_match_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        mcontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_match_context_8>() as size_t,
    );
    return newcontext;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_copy_8(
    mut ccontext: *mut pcre2_convert_context_8,
) -> *mut pcre2_convert_context_8 {
    let mut newcontext: *mut pcre2_convert_context_8 = (*ccontext)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
        (*ccontext).memctl.memory_data,
    ) as *mut pcre2_convert_context_8;
    if newcontext.is_null() {
        return ::core::ptr::null_mut::<pcre2_convert_context_8>();
    }
    memcpy(
        newcontext as *mut ::core::ffi::c_void,
        ccontext as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_real_convert_context_8>() as size_t,
    );
    return newcontext;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_general_context_free_8(mut gcontext: *mut pcre2_general_context_8) {
    if !gcontext.is_null() {
        (*gcontext).memctl.free.expect("non-null function pointer")(
            gcontext as *mut ::core::ffi::c_void,
            (*gcontext).memctl.memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_compile_context_free_8(mut ccontext: *mut pcre2_compile_context_8) {
    if !ccontext.is_null() {
        (*ccontext).memctl.free.expect("non-null function pointer")(
            ccontext as *mut ::core::ffi::c_void,
            (*ccontext).memctl.memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_match_context_free_8(mut mcontext: *mut pcre2_match_context_8) {
    if !mcontext.is_null() {
        (*mcontext).memctl.free.expect("non-null function pointer")(
            mcontext as *mut ::core::ffi::c_void,
            (*mcontext).memctl.memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_convert_context_free_8(mut ccontext: *mut pcre2_convert_context_8) {
    if !ccontext.is_null() {
        (*ccontext).memctl.free.expect("non-null function pointer")(
            ccontext as *mut ::core::ffi::c_void,
            (*ccontext).memctl.memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_character_tables_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut tables: *const uint8_t,
) -> ::core::ffi::c_int {
    (*ccontext).tables = tables;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_bsr_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut value: uint32_t,
) -> ::core::ffi::c_int {
    match value {
        2 | 1 => {
            (*ccontext).bsr_convention = value as uint16_t;
            return 0 as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADDATA,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_length_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut length: size_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_pattern_length = length;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_pattern_compiled_length_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut length: size_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_pattern_compiled_length = length;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_newline_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut newline: uint32_t,
) -> ::core::ffi::c_int {
    match newline {
        1 | 2 | 3 | 4 | 5 | 6 => {
            (*ccontext).newline_convention = newline as uint16_t;
            return 0 as ::core::ffi::c_int;
        }
        _ => return PCRE2_ERROR_BADDATA,
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_max_varlookbehind_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).max_varlookbehind = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_parens_nest_limit_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).parens_nest_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_extra_options_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut options: uint32_t,
) -> ::core::ffi::c_int {
    (*ccontext).extra_options = options;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_compile_recursion_guard_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut guard: Option<
        unsafe extern "C" fn(uint32_t, *mut ::core::ffi::c_void) -> ::core::ffi::c_int,
    >,
    mut user_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*ccontext).stack_guard = guard;
    (*ccontext).stack_guard_data = user_data;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_optimize_8(
    mut ccontext: *mut pcre2_compile_context_8,
    mut directive: uint32_t,
) -> ::core::ffi::c_int {
    if ccontext.is_null() {
        return PCRE2_ERROR_NULL;
    }
    match directive {
        0 => {
            (*ccontext).optimization_flags = 0 as uint32_t;
        }
        1 => {
            (*ccontext).optimization_flags = PCRE2_OPTIMIZATION_ALL as uint32_t;
        }
        _ => {
            if directive >= PCRE2_AUTO_POSSESS as uint32_t
                && directive <= PCRE2_START_OPTIMIZE_OFF as uint32_t
            {
                if directive & 1 as uint32_t != 0 as uint32_t {
                    (*ccontext).optimization_flags = ((*ccontext).optimization_flags
                        as ::core::ffi::c_uint
                        & !((1 as ::core::ffi::c_uint)
                            << (directive >> 1 as ::core::ffi::c_int).wrapping_sub(32 as uint32_t)))
                        as uint32_t;
                } else {
                    (*ccontext).optimization_flags = ((*ccontext).optimization_flags
                        as ::core::ffi::c_uint
                        | (1 as ::core::ffi::c_uint)
                            << (directive >> 1 as ::core::ffi::c_int).wrapping_sub(32 as uint32_t))
                        as uint32_t;
                }
                return 0 as ::core::ffi::c_int;
            }
            return PCRE2_ERROR_BADOPTION;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).callout = callout;
    (*mcontext).callout_data = callout_data;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut substitute_callout: Option<
        unsafe extern "C" fn(
            *mut pcre2_substitute_callout_block_8,
            *mut ::core::ffi::c_void,
        ) -> ::core::ffi::c_int,
    >,
    mut substitute_callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).substitute_callout = substitute_callout;
    (*mcontext).substitute_callout_data = substitute_callout_data;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_substitute_case_callout_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut substitute_case_callout: Option<
        unsafe extern "C" fn(
            PCRE2_SPTR8,
            size_t,
            *mut PCRE2_UCHAR8,
            size_t,
            ::core::ffi::c_int,
            *mut ::core::ffi::c_void,
        ) -> size_t,
    >,
    mut substitute_case_callout_data: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    (*mcontext).substitute_case_callout = substitute_case_callout;
    (*mcontext).substitute_case_callout_data = substitute_case_callout_data;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_heap_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).heap_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_match_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).match_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_depth_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    (*mcontext).depth_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_offset_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: size_t,
) -> ::core::ffi::c_int {
    (*mcontext).offset_limit = limit;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_limit_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut limit: uint32_t,
) -> ::core::ffi::c_int {
    return pcre2_set_depth_limit_8(mcontext, limit);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_recursion_memory_management_8(
    mut mcontext: *mut pcre2_match_context_8,
    mut mymalloc: Option<
        unsafe extern "C" fn(size_t, *mut ::core::ffi::c_void) -> *mut ::core::ffi::c_void,
    >,
    mut myfree: Option<
        unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> (),
    >,
    mut mydata: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_separator_8(
    mut ccontext: *mut pcre2_convert_context_8,
    mut separator: uint32_t,
) -> ::core::ffi::c_int {
    if separator != CHAR_SLASH as uint32_t
        && separator != CHAR_BACKSLASH as uint32_t
        && separator != CHAR_DOT as uint32_t
    {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_separator = separator;
    return 0 as ::core::ffi::c_int;
}
static mut globpunct: *const ::core::ffi::c_char =
    b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~\0" as *const u8 as *const ::core::ffi::c_char;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_set_glob_escape_8(
    mut ccontext: *mut pcre2_convert_context_8,
    mut escape: uint32_t,
) -> ::core::ffi::c_int {
    if escape > 255 as uint32_t
        || escape != 0 as uint32_t && strchr(globpunct, escape as ::core::ffi::c_int).is_null()
    {
        return PCRE2_ERROR_BADDATA;
    }
    (*ccontext).glob_escape = escape;
    return 0 as ::core::ffi::c_int;
}
