pub mod internal {
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct __va_list_tag {
        pub gp_offset: ::core::ffi::c_uint,
        pub fp_offset: ::core::ffi::c_uint,
        pub overflow_arg_area: *mut ::core::ffi::c_void,
        pub reg_save_area: *mut ::core::ffi::c_void,
    }
    pub const PCRE2_CODE_UNIT_WIDTH: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
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
    pub type uint16_t = __uint16_t;
    pub type uint32_t = __uint32_t;
    use super::types_h::{__uint16_t, __uint32_t, __uint8_t};
}
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
    pub type pcre2_code_8 = pcre2_real_code_8;
    pub type pcre2_match_data_8 = pcre2_real_match_data_8;
    pub const PCRE2_ERROR_PARTIAL: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_DFA_UFUNC: ::core::ffi::c_int = -(41 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NOMEMORY: ::core::ffi::c_int = -(48 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NOSUBSTRING: ::core::ffi::c_int = -(49 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_NOUNIQUESUBSTRING: ::core::ffi::c_int = -(50 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_UNAVAILABLE: ::core::ffi::c_int = -(54 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_UNSET: ::core::ffi::c_int = -(55 as ::core::ffi::c_int);
    pub const PCRE2_ERROR_INVALIDOFFSET: ::core::ffi::c_int = -(67 as ::core::ffi::c_int);
    pub const PCRE2_UNSET: size_t = !(0 as ::core::ffi::c_int as size_t);
    use super::pcre2_intmodedep_h::{pcre2_real_code_8, pcre2_real_match_data_8};
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::uint8_t;
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
    pub const PCRE2_MATCHEDBY_DFA_INTERPRETER: C2RustUnnamed_14 = 1;
    pub type C2RustUnnamed_14 = ::core::ffi::c_uint;
    pub const PCRE2_MATCHEDBY_JIT: C2RustUnnamed_14 = 2;
    pub const PCRE2_MATCHEDBY_INTERPRETER: C2RustUnnamed_14 = 0;
    use super::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
    use super::stddef_h::size_t;
    extern "C" {
        pub fn _pcre2_memctl_malloc_8(_: size_t, _: *mut pcre2_memctl) -> *mut ::core::ffi::c_void;
        pub fn _pcre2_strcmp_8(_: PCRE2_SPTR8, _: PCRE2_SPTR8) -> ::core::ffi::c_int;
    }
}
pub mod pcre2_intmodedep_h {
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
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct pcre2_real_match_data_8 {
        pub memctl: pcre2_memctl,
        pub code: *const pcre2_real_code_8,
        pub subject: PCRE2_SPTR8,
        pub mark: PCRE2_SPTR8,
        pub heapframes: *mut heapframe,
        pub heapframes_size: size_t,
        pub subject_length: size_t,
        pub start_offset: size_t,
        pub leftchar: size_t,
        pub rightchar: size_t,
        pub startchar: size_t,
        pub matchedby: uint8_t,
        pub flags: uint8_t,
        pub oveccount: uint16_t,
        pub options: uint32_t,
        pub rc: ::core::ffi::c_int,
        pub ovector: [size_t; 131072],
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct heapframe {
        pub ecode: PCRE2_SPTR8,
        pub back_frame: size_t,
        pub rdepth: uint32_t,
        pub group_frame_type: uint32_t,
        pub return_id: uint8_t,
        pub op: uint8_t,
        pub byte1: uint8_t,
        pub byte2: uint8_t,
        pub fields: C2RustUnnamed,
        pub eptr: PCRE2_SPTR8,
        pub start_match: PCRE2_SPTR8,
        pub mark: PCRE2_SPTR8,
        pub recurse_last_used: PCRE2_SPTR8,
        pub current_recurse: uint32_t,
        pub capture_last: uint32_t,
        pub last_group_offset: size_t,
        pub offset_top: size_t,
        pub ovector: [size_t; 131072],
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union C2RustUnnamed {
        pub char_repeat: C2RustUnnamed_12,
        pub charnot_repeat: C2RustUnnamed_11,
        pub class_repeat: C2RustUnnamed_10,
        pub xclass_repeat: C2RustUnnamed_9,
        pub eclass_repeat: C2RustUnnamed_8,
        pub type_repeat: C2RustUnnamed_7,
        pub ref_repeat: C2RustUnnamed_6,
        pub op_bra: C2RustUnnamed_5,
        pub op_brapos: C2RustUnnamed_4,
        pub op_recurse: C2RustUnnamed_3,
        pub op_assert_scs: C2RustUnnamed_2,
        pub op_cond: C2RustUnnamed_1,
        pub op_vreverse: C2RustUnnamed_0,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_0 {
        pub min: uint32_t,
        pub max: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_1 {
        pub start_branch: PCRE2_SPTR8,
        pub length: size_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_2 {
        pub saved_end_subject: PCRE2_SPTR8,
        pub saved_eptr: PCRE2_SPTR8,
        pub true_end_extra: size_t,
        pub saved_moptions: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_3 {
        pub start_branch: PCRE2_SPTR8,
        pub frame_type: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_4 {
        pub start_eptr: PCRE2_SPTR8,
        pub start_group: PCRE2_SPTR8,
        pub frame_type: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_5 {
        pub frame_type: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_6 {
        pub start: PCRE2_SPTR8,
        pub offset: size_t,
        pub length: size_t,
        pub min: uint32_t,
        pub max: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_7 {
        pub start_eptr: PCRE2_SPTR8,
        pub min: uint32_t,
        pub max: uint32_t,
        pub ctype: uint32_t,
        pub propvalue: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_8 {
        pub start_eptr: PCRE2_SPTR8,
        pub eclass_data: PCRE2_SPTR8,
        pub eclass_len: size_t,
        pub min: uint32_t,
        pub max: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_9 {
        pub start_eptr: PCRE2_SPTR8,
        pub xclass_data: PCRE2_SPTR8,
        pub min: uint32_t,
        pub max: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_10 {
        pub start_eptr: PCRE2_SPTR8,
        pub byte_map_address: PCRE2_SPTR8,
        pub min: uint32_t,
        pub max: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_11 {
        pub start_eptr: PCRE2_SPTR8,
        pub min: uint32_t,
        pub max: uint32_t,
        pub c: uint32_t,
        pub oc: uint32_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct C2RustUnnamed_12 {
        pub start_eptr: PCRE2_SPTR8,
        pub charptr: PCRE2_SPTR8,
        pub min: uint32_t,
        pub max: uint32_t,
        pub c: uint32_t,
        pub oc: C2RustUnnamed_13,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union C2RustUnnamed_13 {
        pub oc: uint32_t,
        pub occu: [PCRE2_UCHAR8; 4],
    }
    pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    use super::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
    use super::pcre2_internal_h::pcre2_memctl;
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
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
    }
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::{__va_list_tag, PCRE2_CODE_UNIT_WIDTH};
pub use self::pcre2_h::{
    pcre2_code_8, pcre2_match_data_8, PCRE2_ERROR_DFA_UFUNC, PCRE2_ERROR_INVALIDOFFSET,
    PCRE2_ERROR_NOMEMORY, PCRE2_ERROR_NOSUBSTRING, PCRE2_ERROR_NOUNIQUESUBSTRING,
    PCRE2_ERROR_PARTIAL, PCRE2_ERROR_UNAVAILABLE, PCRE2_ERROR_UNSET, PCRE2_SPTR8, PCRE2_UCHAR8,
    PCRE2_UNSET,
};
pub use self::pcre2_internal_h::{
    _pcre2_memctl_malloc_8, _pcre2_strcmp_8, pcre2_memctl, C2RustUnnamed_14,
    PCRE2_MATCHEDBY_DFA_INTERPRETER, PCRE2_MATCHEDBY_INTERPRETER, PCRE2_MATCHEDBY_JIT,
};
pub use self::pcre2_intmodedep_h::{
    heapframe, pcre2_real_code_8, pcre2_real_match_data_8, C2RustUnnamed, C2RustUnnamed_0,
    C2RustUnnamed_1, C2RustUnnamed_10, C2RustUnnamed_11, C2RustUnnamed_12, C2RustUnnamed_13,
    C2RustUnnamed_2, C2RustUnnamed_3, C2RustUnnamed_4, C2RustUnnamed_5, C2RustUnnamed_6,
    C2RustUnnamed_7, C2RustUnnamed_8, C2RustUnnamed_9, IMM2_SIZE,
};
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, strtod, strtol, strtoll};
use self::string_h::memcpy;
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut buffer: *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_copy_bynumber_8(match_data, n, buffer, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_copy_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut buffer: *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &raw mut size);
    if rc < 0 as ::core::ffi::c_int {
        return rc;
    }
    if size.wrapping_add(1 as size_t) > *sizeptr {
        return PCRE2_ERROR_NOMEMORY;
    }
    if size != 0 as size_t {
        memcpy(
            buffer as *mut ::core::ffi::c_void,
            (*match_data).subject.offset(
                (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] as isize,
            ) as *const ::core::ffi::c_void,
            size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
        );
    }
    *buffer.offset(size as isize) = 0 as PCRE2_UCHAR8;
    *sizeptr = size;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut stringptr: *mut *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_get_bynumber_8(match_data, n, stringptr, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_get_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut stringptr: *mut *mut PCRE2_UCHAR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut rc: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    let mut yield_0: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    rc = pcre2_substring_length_bynumber_8(match_data, stringnumber, &raw mut size);
    if rc < 0 as ::core::ffi::c_int {
        return rc;
    }
    yield_0 = _pcre2_memctl_malloc_8(
        (::core::mem::size_of::<pcre2_memctl>() as size_t).wrapping_add(
            size.wrapping_add(1 as size_t)
                .wrapping_mul(PCRE2_CODE_UNIT_WIDTH as size_t),
        ),
        match_data as *mut pcre2_memctl,
    ) as *mut PCRE2_UCHAR8;
    if yield_0.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }
    yield_0 = (yield_0 as *mut ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
        as *mut PCRE2_UCHAR8;
    if size != 0 as size_t {
        memcpy(
            yield_0 as *mut ::core::ffi::c_void,
            (*match_data).subject.offset(
                (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] as isize,
            ) as *const ::core::ffi::c_void,
            size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
        );
    }
    *yield_0.offset(size as isize) = 0 as PCRE2_UCHAR8;
    *stringptr = yield_0;
    *sizeptr = size;
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_free_8(mut string: *mut PCRE2_UCHAR8) {
    if !string.is_null() {
        let mut memctl: *mut pcre2_memctl = (string as *mut ::core::ffi::c_char)
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_byname_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringname: PCRE2_SPTR8,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut entry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut failrc: ::core::ffi::c_int = 0;
    let mut entrysize: ::core::ffi::c_int = 0;
    if (*match_data).matchedby as ::core::ffi::c_int
        == PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        return PCRE2_ERROR_DFA_UFUNC;
    }
    entrysize = pcre2_substring_nametable_scan_8(
        (*match_data).code as *const pcre2_code_8,
        stringname,
        &raw mut first,
        &raw mut last,
    );
    if entrysize < 0 as ::core::ffi::c_int {
        return entrysize;
    }
    failrc = PCRE2_ERROR_UNAVAILABLE;
    entry = first;
    while entry <= last {
        let mut n: uint32_t = ((*entry.offset(0 as ::core::ffi::c_int as isize)
            as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *entry.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as uint32_t;
        if n < (*match_data).oveccount as uint32_t {
            if (*match_data).ovector[n.wrapping_mul(2 as uint32_t) as usize] != PCRE2_UNSET {
                return pcre2_substring_length_bynumber_8(match_data, n, sizeptr);
            }
            failrc = PCRE2_ERROR_UNSET;
        }
        entry = entry.offset(entrysize as isize);
    }
    return failrc;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_length_bynumber_8(
    mut match_data: *mut pcre2_match_data_8,
    mut stringnumber: uint32_t,
    mut sizeptr: *mut size_t,
) -> ::core::ffi::c_int {
    let mut left: size_t = 0;
    let mut right: size_t = 0;
    let mut count: ::core::ffi::c_int = (*match_data).rc;
    if count == PCRE2_ERROR_PARTIAL {
        if stringnumber > 0 as uint32_t {
            return PCRE2_ERROR_PARTIAL;
        }
        count = 0 as ::core::ffi::c_int;
    } else if count < 0 as ::core::ffi::c_int {
        return count;
    }
    if (*match_data).matchedby as ::core::ffi::c_int
        != PCRE2_MATCHEDBY_DFA_INTERPRETER as ::core::ffi::c_int
    {
        if stringnumber > (*(*match_data).code).top_bracket as uint32_t {
            return PCRE2_ERROR_NOSUBSTRING;
        }
        if stringnumber >= (*match_data).oveccount as uint32_t {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize] == PCRE2_UNSET {
            return PCRE2_ERROR_UNSET;
        }
    } else {
        if stringnumber >= (*match_data).oveccount as uint32_t {
            return PCRE2_ERROR_UNAVAILABLE;
        }
        if count != 0 as ::core::ffi::c_int && stringnumber >= count as uint32_t {
            return PCRE2_ERROR_UNSET;
        }
    }
    left = (*match_data).ovector[stringnumber.wrapping_mul(2 as uint32_t) as usize];
    right = (*match_data).ovector[stringnumber
        .wrapping_mul(2 as uint32_t)
        .wrapping_add(1 as uint32_t) as usize];
    if left > (*match_data).subject_length || right > (*match_data).subject_length {
        return PCRE2_ERROR_INVALIDOFFSET;
    }
    if !sizeptr.is_null() {
        *sizeptr = if left > right {
            0 as size_t
        } else {
            right.wrapping_sub(left)
        };
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_get_8(
    mut match_data: *mut pcre2_match_data_8,
    mut listptr: *mut *mut *mut PCRE2_UCHAR8,
    mut lengthsptr: *mut *mut size_t,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    let mut count: ::core::ffi::c_int = 0;
    let mut count2: ::core::ffi::c_int = 0;
    let mut size: size_t = 0;
    let mut lensp: *mut size_t = ::core::ptr::null_mut::<size_t>();
    let mut memp: *mut pcre2_memctl = ::core::ptr::null_mut::<pcre2_memctl>();
    let mut listp: *mut *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<*mut PCRE2_UCHAR8>();
    let mut sp: *mut PCRE2_UCHAR8 = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    let mut ovector: *mut size_t = ::core::ptr::null_mut::<size_t>();
    count = (*match_data).rc;
    if count < 0 as ::core::ffi::c_int {
        return count;
    }
    if count == 0 as ::core::ffi::c_int {
        count = (*match_data).oveccount as ::core::ffi::c_int;
    }
    count2 = 2 as ::core::ffi::c_int * count;
    ovector = &raw mut (*match_data).ovector as *mut size_t;
    size = (::core::mem::size_of::<pcre2_memctl>() as usize)
        .wrapping_add(::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize) as size_t;
    if !lengthsptr.is_null() {
        size = (size as ::core::ffi::c_ulong).wrapping_add(
            (::core::mem::size_of::<size_t>() as usize).wrapping_mul(count as usize)
                as ::core::ffi::c_ulong,
        ) as size_t as size_t;
    }
    i = 0 as ::core::ffi::c_int;
    while i < count2 {
        size = (size as ::core::ffi::c_ulong).wrapping_add(
            (::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize).wrapping_add(
                (1 as ::core::ffi::c_int * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int))
                    as usize,
            ) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if *ovector.offset((i + 1 as ::core::ffi::c_int) as isize) > *ovector.offset(i as isize) {
            size = (size as ::core::ffi::c_ulong).wrapping_add(
                (*ovector.offset((i + 1 as ::core::ffi::c_int) as isize))
                    .wrapping_sub(*ovector.offset(i as isize))
                    .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t)
                    as ::core::ffi::c_ulong,
            ) as size_t as size_t;
        }
        i += 2 as ::core::ffi::c_int;
    }
    memp = _pcre2_memctl_malloc_8(size, match_data as *mut pcre2_memctl) as *mut pcre2_memctl;
    if memp.is_null() {
        return PCRE2_ERROR_NOMEMORY;
    }
    listp = (memp as *mut ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_memctl>() as usize as isize)
        as *mut *mut PCRE2_UCHAR8;
    *listptr = listp;
    lensp = (listp as *mut ::core::ffi::c_char).offset(
        (::core::mem::size_of::<*mut PCRE2_UCHAR8>() as usize)
            .wrapping_mul((count + 1 as ::core::ffi::c_int) as usize) as isize,
    ) as *mut size_t;
    if lengthsptr.is_null() {
        sp = lensp as *mut PCRE2_UCHAR8;
        lensp = ::core::ptr::null_mut::<size_t>();
    } else {
        *lengthsptr = lensp;
        sp = (lensp as *mut ::core::ffi::c_char).offset(
            (::core::mem::size_of::<size_t>() as usize).wrapping_mul(count as usize) as isize,
        ) as *mut PCRE2_UCHAR8;
    }
    i = 0 as ::core::ffi::c_int;
    while i < count2 {
        size = if *ovector.offset((i + 1 as ::core::ffi::c_int) as isize)
            > *ovector.offset(i as isize)
        {
            (*ovector.offset((i + 1 as ::core::ffi::c_int) as isize))
                .wrapping_sub(*ovector.offset(i as isize))
        } else {
            0 as size_t
        };
        if size != 0 as size_t {
            memcpy(
                sp as *mut ::core::ffi::c_void,
                (*match_data)
                    .subject
                    .offset(*ovector.offset(i as isize) as isize)
                    as *const ::core::ffi::c_void,
                size.wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as size_t),
            );
        }
        let fresh6 = listp;
        listp = listp.offset(1);
        *fresh6 = sp;
        if !lensp.is_null() {
            let fresh7 = lensp;
            lensp = lensp.offset(1);
            *fresh7 = size;
        }
        sp = sp.offset(size as isize);
        let fresh8 = sp;
        sp = sp.offset(1);
        *fresh8 = 0 as PCRE2_UCHAR8;
        i += 2 as ::core::ffi::c_int;
    }
    *listp = ::core::ptr::null_mut::<PCRE2_UCHAR8>();
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_list_free_8(mut list: *mut *mut PCRE2_UCHAR8) {
    if !list.is_null() {
        let mut memctl: *mut pcre2_memctl = (list as *mut ::core::ffi::c_char)
            .offset(-(::core::mem::size_of::<pcre2_memctl>() as usize as isize))
            as *mut pcre2_memctl;
        (*memctl).free.expect("non-null function pointer")(
            memctl as *mut ::core::ffi::c_void,
            (*memctl).memory_data,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_nametable_scan_8(
    mut code: *const pcre2_code_8,
    mut stringname: PCRE2_SPTR8,
    mut firstptr: *mut PCRE2_SPTR8,
    mut lastptr: *mut PCRE2_SPTR8,
) -> ::core::ffi::c_int {
    let mut bot: uint16_t = 0 as uint16_t;
    let mut top: uint16_t = (*code).name_count;
    let mut entrysize: uint16_t = (*code).name_entry_size;
    let mut nametable: PCRE2_SPTR8 = (code as *const ::core::ffi::c_char)
        .offset(::core::mem::size_of::<pcre2_real_code_8>() as usize as isize)
        as PCRE2_SPTR8;
    while top as ::core::ffi::c_int > bot as ::core::ffi::c_int {
        let mut mid: uint16_t = ((top as ::core::ffi::c_int + bot as ::core::ffi::c_int)
            / 2 as ::core::ffi::c_int) as uint16_t;
        let mut entry: PCRE2_SPTR8 = nametable
            .offset((entrysize as ::core::ffi::c_int * mid as ::core::ffi::c_int) as isize);
        let mut c: ::core::ffi::c_int =
            _pcre2_strcmp_8(stringname, entry.offset(IMM2_SIZE as isize));
        if c == 0 as ::core::ffi::c_int {
            let mut first: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut last: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            let mut lastentry: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
            lastentry = nametable.offset(
                (entrysize as ::core::ffi::c_int
                    * ((*code).name_count as ::core::ffi::c_int - 1 as ::core::ffi::c_int))
                    as isize,
            );
            last = entry;
            first = last;
            while first > nametable {
                if _pcre2_strcmp_8(
                    stringname,
                    first
                        .offset(-(entrysize as ::core::ffi::c_int as isize))
                        .offset(IMM2_SIZE as isize),
                ) != 0 as ::core::ffi::c_int
                {
                    break;
                }
                first = first.offset(-(entrysize as ::core::ffi::c_int as isize));
            }
            while last < lastentry {
                if _pcre2_strcmp_8(
                    stringname,
                    last.offset(entrysize as ::core::ffi::c_int as isize)
                        .offset(IMM2_SIZE as isize),
                ) != 0 as ::core::ffi::c_int
                {
                    break;
                }
                last = last.offset(entrysize as ::core::ffi::c_int as isize);
            }
            if firstptr.is_null() {
                return if first == last {
                    ((*entry.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *entry
                            .offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as ::core::ffi::c_int
                } else {
                    PCRE2_ERROR_NOUNIQUESUBSTRING
                };
            }
            *firstptr = first;
            *lastptr = last;
            return entrysize as ::core::ffi::c_int;
        }
        if c > 0 as ::core::ffi::c_int {
            bot = (mid as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as uint16_t;
        } else {
            top = mid;
        }
    }
    return PCRE2_ERROR_NOSUBSTRING;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pcre2_substring_number_from_name_8(
    mut code: *const pcre2_code_8,
    mut stringname: PCRE2_SPTR8,
) -> ::core::ffi::c_int {
    return pcre2_substring_nametable_scan_8(
        code,
        stringname,
        ::core::ptr::null_mut::<PCRE2_SPTR8>(),
        ::core::ptr::null_mut::<PCRE2_SPTR8>(),
    );
}
