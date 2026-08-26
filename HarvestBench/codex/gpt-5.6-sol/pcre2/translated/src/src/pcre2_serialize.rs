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
pub mod stdint_intn_h {
    pub type int32_t = __int32_t;
    use super::types_h::__int32_t;
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
    pub type uint16_t = __uint16_t;
    pub type uint32_t = __uint32_t;
    use super::types_h::{__uint16_t, __uint32_t, __uint8_t};
}
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub type pcre2_general_context_8 = pcre2_real_general_context_8;
    pub type pcre2_compile_context_8 = pcre2_real_compile_context_8;
    pub type pcre2_code_8 = pcre2_real_code_8;
    pub const PCRE2_MAJOR: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
    pub const PCRE2_MINOR: ::core::ffi::c_int = 48 as ::core::ffi::c_int;
    pub const PCRE2_ERROR_BADDATA: ::core::ffi::c_int = -(29 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_MIXEDTABLES: ::core::ffi::c_int = -(30 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_BADMAGIC: ::core::ffi::c_int = -(31 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_BADMODE: ::core::ffi::c_int = -(32 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NULL: ::core::ffi::c_int = -(51 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_BADSERIALIZEDDATA: ::core::ffi::c_int = -(62 as ::core::ffi::c_int);
    use super::pcre2_intmodedep_h::{
        pcre2_real_code_8, pcre2_real_compile_context_8, pcre2_real_general_context_8,
    };
    use super::stdint_uintn_h::uint8_t;
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
    pub struct pcre2_real_code_8 {
        pub memctl: pcre2_memctl,
        pub tables: *const uint8_t,
        pub executable_jit: *mut ::core::ffi::c_void,
        pub start_bitmap: [uint8_t; 32],
        pub blocksize: size_t,
        pub code_start: size_t,
        pub magic_number: uint32_t,
        pub compile_options: uint32_t,
        pub overall_options: uint32_t,
        pub extra_options: uint32_t,
        pub flags: uint32_t,
        pub limit_heap: uint32_t,
        pub limit_match: uint32_t,
        pub limit_depth: uint32_t,
        pub first_codeunit: uint32_t,
        pub last_codeunit: uint32_t,
        pub bsr_convention: uint16_t,
        pub newline_convention: uint16_t,
        pub max_lookbehind: uint16_t,
        pub minlength: uint16_t,
        pub top_bracket: uint16_t,
        pub top_backref: uint16_t,
        pub name_entry_size: uint16_t,
        pub name_count: uint16_t,
        pub optimization_flags: uint32_t,
    }
    pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
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
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_serialized_data {
        pub magic: uint32_t,
        pub version: uint32_t,
        pub config: uint32_t,
        pub number_of_codes: int32_t,
    }
    pub const PCRE2_DEREF_TABLES: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
    pub const MAGIC_NUMBER: ::core::ffi::c_ulong = 0x50435245 as ::core::ffi::c_ulong;
    pub const cbit_length: ::core::ffi::c_int = 320 as ::core::ffi::c_int;
    pub const cbits_offset: ::core::ffi::c_int = 512 as ::core::ffi::c_int;
    pub const ctypes_offset: ::core::ffi::c_int = cbits_offset + cbit_length;
    pub const TABLES_LENGTH: ::core::ffi::c_int = ctypes_offset + 256 as ::core::ffi::c_int;
    use super::pcre2_h::pcre2_compile_context_8;
    use super::stddef_h::size_t;
    use super::stdint_intn_h::int32_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
    extern "C" {
        pub static mut _pcre2_default_compile_context_8: pcre2_compile_context_8;
        pub fn _pcre2_memctl_malloc_8(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
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
        pub fn memset(
            __s: *mut ::core::ffi::c_void,
            __c: ::core::ffi::c_int,
            __n: size_t,
        ) -> *mut ::core::ffi::c_void;
    }
}
pub mod config_h {
    pub const MAX_NAME_COUNT: ::core::ffi::c_int = 10000 as ::core::ffi::c_int;
    pub const MAX_NAME_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::config_h::{MAX_NAME_COUNT, MAX_NAME_SIZE};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::{
    pcre2_code_8, pcre2_compile_context_8, pcre2_general_context_8, PCRE2_ERROR_BADDATA,
    PCRE2_ERROR_BADMAGIC, PCRE2_ERROR_BADMODE, PCRE2_ERROR_BADSERIALIZEDDATA,
    PCRE2_ERROR_MIXEDTABLES, PCRE2_ERROR_NOMEMORY, PCRE2_ERROR_NULL, PCRE2_MAJOR, PCRE2_MINOR,
    PCRE2_UCHAR8,
};
pub use self::pcre2_internal_h::{
    _pcre2_default_compile_context_8, _pcre2_memctl_malloc_8, cbit_length, cbits_offset,
    ctypes_offset, pcre2_memctl, pcre2_serialized_data, MAGIC_NUMBER, PCRE2_DEREF_TABLES,
    TABLES_LENGTH,
};
pub use self::pcre2_intmodedep_h::{
    pcre2_real_code_8, pcre2_real_compile_context_8, pcre2_real_general_context_8, IMM2_SIZE,
};
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_intn_h::int32_t;
pub use self::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, strtod, strtol, strtoll};
use self::string_h::{memcpy, memset};
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
pub const SERIALIZED_DATA_MAGIC: ::core::ffi::c_uint = 0x50523253 as ::core::ffi::c_uint;
pub const SERIALIZED_DATA_VERSION: ::core::ffi::c_int =
    10 as ::core::ffi::c_int | (48 as ::core::ffi::c_int) << 16 as ::core::ffi::c_int;
pub const SERIALIZED_DATA_CONFIG: usize = ::core::mem::size_of::<PCRE2_UCHAR8>() as usize
    | (::core::mem::size_of::<*mut ::core::ffi::c_void>() as usize) << 8 as ::core::ffi::c_int
    | (::core::mem::size_of::<size_t>() as usize) << 16 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_encode_8(
    mut codes: *mut *const pcre2_code_8,
    mut number_of_codes: int32_t,
    mut serialized_bytes: *mut *mut uint8_t,
    mut serialized_size: *mut size_t,
    mut gcontext: *mut pcre2_general_context_8,
) -> int32_t {
    let mut bytes: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut dst_bytes: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut i: int32_t = 0;
    let mut total_size: size_t = 0;
    let mut re: *const pcre2_real_code_8 = ::core::ptr::null::<pcre2_real_code_8>();
    let mut tables: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut data: *mut pcre2_serialized_data = ::core::ptr::null_mut::<pcre2_serialized_data>();
    let mut memctl: *const pcre2_memctl = if !gcontext.is_null() {
        &raw mut (*gcontext).memctl
    } else {
        &raw mut _pcre2_default_compile_context_8.memctl
    };
    if codes.is_null() || serialized_bytes.is_null() || serialized_size.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADDATA as int32_t;
    }
    total_size = (::core::mem::size_of::<pcre2_serialized_data>() as usize)
        .wrapping_add(TABLES_LENGTH as usize) as size_t;
    tables = ::core::ptr::null::<uint8_t>();
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < number_of_codes {
        if (*codes.offset(i as isize)).is_null() {
            return PCRE2_ERROR_NULL as int32_t;
        }
        re = *codes.offset(i as isize) as *const pcre2_real_code_8;
        if (*re).magic_number as ::core::ffi::c_ulong != MAGIC_NUMBER {
            return PCRE2_ERROR_BADMAGIC as int32_t;
        }
        if tables.is_null() {
            tables = (*re).tables;
        } else if tables != (*re).tables {
            return PCRE2_ERROR_MIXEDTABLES as int32_t;
        }
        total_size = (total_size as ::core::ffi::c_ulong)
            .wrapping_add((*re).blocksize as ::core::ffi::c_ulong) as size_t
            as size_t;
        i += 1;
    }
    bytes = (*memctl).malloc.expect("non-null function pointer")(
        total_size.wrapping_add(::core::mem::size_of::<pcre2_memctl>() as size_t),
        (*memctl).memory_data,
    ) as *mut uint8_t;
    if bytes.is_null() {
        return PCRE2_ERROR_NOMEMORY as int32_t;
    }
    memcpy(
        bytes as *mut ::core::ffi::c_void,
        memctl as *const ::core::ffi::c_void,
        ::core::mem::size_of::<pcre2_memctl>() as size_t,
    );
    bytes = bytes.offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize);
    data = bytes as *mut pcre2_serialized_data;
    (*data).magic = SERIALIZED_DATA_MAGIC as uint32_t;
    (*data).version = SERIALIZED_DATA_VERSION as uint32_t;
    (*data).config = SERIALIZED_DATA_CONFIG as uint32_t;
    (*data).number_of_codes = number_of_codes;
    dst_bytes = bytes.offset(::core::mem::size_of::<pcre2_serialized_data>() as usize as isize);
    memcpy(
        dst_bytes as *mut ::core::ffi::c_void,
        tables as *const ::core::ffi::c_void,
        TABLES_LENGTH as size_t,
    );
    dst_bytes = dst_bytes.offset(TABLES_LENGTH as isize);
    i = 0 as ::core::ffi::c_int as int32_t;
    while i < number_of_codes {
        re = *codes.offset(i as isize) as *const pcre2_real_code_8;
        memcpy(
            dst_bytes as *mut ::core::ffi::c_void,
            re as *const ::core::ffi::c_char as *const ::core::ffi::c_void,
            (*re).blocksize,
        );
        memset(
            dst_bytes.offset(0 as ::core::ffi::c_ulong as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<pcre2_memctl>() as size_t,
        );
        memset(
            dst_bytes.offset(24 as ::core::ffi::c_ulong as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        );
        memset(
            dst_bytes.offset(32 as ::core::ffi::c_ulong as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<*mut ::core::ffi::c_void>() as size_t,
        );
        dst_bytes = dst_bytes.offset((*re).blocksize as isize);
        i += 1;
    }
    *serialized_bytes = bytes;
    *serialized_size = total_size;
    return number_of_codes;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_decode_8(
    mut codes: *mut *mut pcre2_code_8,
    mut number_of_codes: int32_t,
    mut bytes: *const uint8_t,
    mut gcontext: *mut pcre2_general_context_8,
) -> int32_t {
    let mut current_block: u64;
    let mut data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;
    let mut memctl: *const pcre2_memctl = if !gcontext.is_null() {
        &raw mut (*gcontext).memctl
    } else {
        &raw mut _pcre2_default_compile_context_8.memctl
    };
    let mut src_bytes: *const uint8_t = ::core::ptr::null::<uint8_t>();
    let mut dst_re: *mut pcre2_real_code_8 = ::core::ptr::null_mut::<pcre2_real_code_8>();
    let mut tables: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut i: int32_t = 0;
    let mut j: int32_t = 0;
    let mut error: int32_t = 0;
    if data.is_null() || codes.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADDATA as int32_t;
    }
    if (*data).number_of_codes <= 0 as int32_t {
        return PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
    }
    if (*data).magic != SERIALIZED_DATA_MAGIC as uint32_t {
        return PCRE2_ERROR_BADMAGIC as int32_t;
    }
    if (*data).version != SERIALIZED_DATA_VERSION as uint32_t {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if (*data).config as usize != SERIALIZED_DATA_CONFIG {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if number_of_codes > (*data).number_of_codes {
        number_of_codes = (*data).number_of_codes;
    }
    src_bytes = bytes.offset(::core::mem::size_of::<pcre2_serialized_data>() as usize as isize);
    tables = (*memctl).malloc.expect("non-null function pointer")(
        (TABLES_LENGTH as size_t).wrapping_add(::core::mem::size_of::<size_t>() as size_t),
        (*memctl).memory_data,
    ) as *mut uint8_t;
    if tables.is_null() {
        return PCRE2_ERROR_NOMEMORY as int32_t;
    }
    memcpy(
        tables as *mut ::core::ffi::c_void,
        src_bytes as *const ::core::ffi::c_void,
        TABLES_LENGTH as size_t,
    );
    *(tables.offset(TABLES_LENGTH as isize) as *mut size_t) = number_of_codes as size_t;
    src_bytes = src_bytes.offset(TABLES_LENGTH as isize);
    i = 0 as ::core::ffi::c_int as int32_t;
    loop {
        if !(i < number_of_codes) {
            current_block = 17281240262373992796;
            break;
        }
        let mut blocksize: size_t = 0;
        memcpy(
            &raw mut blocksize as *mut ::core::ffi::c_void,
            src_bytes.offset(72 as ::core::ffi::c_ulong as isize) as *const ::core::ffi::c_void,
            ::core::mem::size_of::<size_t>() as size_t,
        );
        if blocksize <= ::core::mem::size_of::<pcre2_real_code_8>() as usize {
            error = PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
            current_block = 12923115575694731468;
            break;
        } else {
            dst_re = _pcre2_memctl_malloc_8(blocksize, gcontext as *mut pcre2_memctl)
                as *mut pcre2_real_code_8;
            if dst_re.is_null() {
                error = PCRE2_ERROR_NOMEMORY as int32_t;
                current_block = 12923115575694731468;
                break;
            } else {
                memcpy(
                    (dst_re as *mut uint8_t)
                        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
                        as *mut ::core::ffi::c_void,
                    src_bytes.offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
                        as *const ::core::ffi::c_void,
                    blocksize.wrapping_sub(::core::mem::size_of::<pcre2_memctl>() as size_t),
                );
                if (*dst_re).magic_number as ::core::ffi::c_ulong != MAGIC_NUMBER
                    || (*dst_re).name_entry_size as ::core::ffi::c_int
                        > MAX_NAME_SIZE + IMM2_SIZE + 1 as ::core::ffi::c_int
                    || (*dst_re).name_count as ::core::ffi::c_int > MAX_NAME_COUNT
                {
                    error = PCRE2_ERROR_BADSERIALIZEDDATA as int32_t;
                    current_block = 12923115575694731468;
                    break;
                } else {
                    (*dst_re).tables = tables;
                    (*dst_re).executable_jit = NULL_0;
                    (*dst_re).flags =
                        ((*dst_re).flags as ::core::ffi::c_uint | PCRE2_DEREF_TABLES) as uint32_t;
                    let ref mut fresh6 = *codes.offset(i as isize);
                    *fresh6 = dst_re as *mut pcre2_code_8;
                    dst_re = ::core::ptr::null_mut::<pcre2_real_code_8>();
                    src_bytes = src_bytes.offset(blocksize as isize);
                    i += 1;
                }
            }
        }
    }
    match current_block {
        17281240262373992796 => return number_of_codes,
        _ => {
            if !dst_re.is_null() {
                (*memctl).free.expect("non-null function pointer")(
                    dst_re as *mut ::core::ffi::c_void,
                    (*memctl).memory_data,
                );
            }
            (*memctl).free.expect("non-null function pointer")(
                tables as *mut ::core::ffi::c_void,
                (*memctl).memory_data,
            );
            j = 0 as ::core::ffi::c_int as int32_t;
            while j < i {
                (*memctl).free.expect("non-null function pointer")(
                    *codes.offset(j as isize) as *mut ::core::ffi::c_void,
                    (*memctl).memory_data,
                );
                let ref mut fresh7 = *codes.offset(j as isize);
                *fresh7 = ::core::ptr::null_mut::<pcre2_code_8>();
                j += 1;
            }
            return error;
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_get_number_of_codes_8(
    mut bytes: *const uint8_t,
) -> int32_t {
    let mut data: *const pcre2_serialized_data = bytes as *const pcre2_serialized_data;
    if data.is_null() {
        return PCRE2_ERROR_NULL as int32_t;
    }
    if (*data).magic != SERIALIZED_DATA_MAGIC as uint32_t {
        return PCRE2_ERROR_BADMAGIC as int32_t;
    }
    if (*data).version != SERIALIZED_DATA_VERSION as uint32_t {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    if (*data).config as usize != SERIALIZED_DATA_CONFIG {
        return PCRE2_ERROR_BADMODE as int32_t;
    }
    return (*data).number_of_codes;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_serialize_free_8(mut bytes: *mut uint8_t) {
    if !bytes.is_null() {
        let mut memctl: *mut pcre2_memctl = bytes
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
