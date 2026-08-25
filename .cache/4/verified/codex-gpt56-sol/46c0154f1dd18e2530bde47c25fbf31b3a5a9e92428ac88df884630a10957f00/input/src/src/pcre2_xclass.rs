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
pub mod pcre2_internal_h {
    pub type BOOL = ::core::ffi::c_int;
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct ucd_record {
        pub script: uint8_t,
        pub chartype: uint8_t,
        pub gbprop: uint8_t,
        pub caseset: uint8_t,
        pub other_case: int32_t,
        pub scriptx_bidiclass: uint16_t,
        pub bprops: uint16_t,
    }
    pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    pub const CHAR_HT: uint32_t = 9 as uint32_t;
    pub const CHAR_VT: uint32_t = 11 as uint32_t;
    pub const CHAR_FF: uint32_t = 12 as uint32_t;
    pub const CHAR_CR: uint32_t = 13 as uint32_t;
    pub const CHAR_LF: uint32_t = 10 as uint32_t;
    pub const CHAR_NEL: uint32_t = 133 as uint32_t;
    pub const CHAR_SPACE: uint32_t = 32 as uint32_t;
    pub const CHAR_DOLLAR_SIGN: ::core::ffi::c_int = '$' as i32;
    pub const CHAR_0: ::core::ffi::c_int = '0' as i32;
    pub const CHAR_9: ::core::ffi::c_int = '9' as i32;
    pub const CHAR_COMMERCIAL_AT: ::core::ffi::c_int = '@' as i32;
    pub const CHAR_A: ::core::ffi::c_int = 'A' as i32;
    pub const CHAR_F: ::core::ffi::c_int = 'F' as i32;
    pub const CHAR_GRAVE_ACCENT: ::core::ffi::c_int = '`' as i32;
    pub const CHAR_a: ::core::ffi::c_int = 'a' as i32;
    pub const CHAR_f: ::core::ffi::c_int = 'f' as i32;
    pub const CHAR_NBSP: uint32_t = 160 as uint32_t;
    pub const PT_LAMP: ::core::ffi::c_int = 0;
    pub const PT_GC: ::core::ffi::c_int = 1;
    pub const PT_PC: ::core::ffi::c_int = 2;
    pub const PT_SC: ::core::ffi::c_int = 3;
    pub const PT_SCX: ::core::ffi::c_int = 4;
    pub const PT_ALNUM: ::core::ffi::c_int = 5;
    pub const PT_SPACE: ::core::ffi::c_int = 6;
    pub const PT_PXSPACE: ::core::ffi::c_int = 7;
    pub const PT_WORD: ::core::ffi::c_int = 8;
    pub const PT_UCNC: ::core::ffi::c_int = 10;
    pub const PT_BIDICL: ::core::ffi::c_int = 11;
    pub const PT_BOOL: ::core::ffi::c_int = 12;
    pub const PT_PXGRAPH: ::core::ffi::c_int = 14;
    pub const PT_PXPRINT: ::core::ffi::c_int = 15;
    pub const PT_PXPUNCT: ::core::ffi::c_int = 16;
    pub const PT_PXXDIGIT: ::core::ffi::c_int = 17;
    pub const XCL_NOT: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
    pub const XCL_MAP: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
    pub const XCL_END: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    pub const XCL_SINGLE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    pub const XCL_PROP: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
    pub const XCL_NOTPROP: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
    pub const XCL_CHAR_LIST_HIGH_16_START: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
    pub const XCL_CHAR_LIST_LOW_32_START: ::core::ffi::c_int = 0x10000 as ::core::ffi::c_int;
    pub const XCL_TYPE_MASK: ::core::ffi::c_int = 0xfff as ::core::ffi::c_int;
    pub const XCL_TYPE_BIT_LEN: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
    pub const XCL_BEGIN_WITH_RANGE: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
    pub const XCL_ITEM_COUNT_MASK: ::core::ffi::c_int = 0x3 as ::core::ffi::c_int;
    pub const XCL_CHAR_END: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
    pub const XCL_CHAR_SHIFT: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    pub const ECL_MAP: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
    pub const ECL_AND: ::core::ffi::c_int = 1;
    pub const ECL_OR: ::core::ffi::c_int = 2;
    pub const ECL_XOR: ::core::ffi::c_int = 3;
    pub const ECL_NOT: ::core::ffi::c_int = 4;
    pub const ECL_XCLASS: ::core::ffi::c_int = 5;
    pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
    pub const UCD_BIDICLASS_SHIFT: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
    use super::stdint_intn_h::int32_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
    extern "C" {
        pub static _pcre2_ucd_boolprop_sets_8: [uint32_t; 0];
        pub static _pcre2_ucd_script_sets_8: [uint32_t; 0];
        pub static _pcre2_ucd_records_8: [ucd_record; 0];
        pub static _pcre2_ucd_stage1_8: [uint16_t; 0];
        pub static _pcre2_ucd_stage2_8: [uint16_t; 0];
        pub static _pcre2_ucp_gentype_8: [uint32_t; 0];
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
    use super::stdint_uintn_h::uint8_t;
}
pub mod pcre2_ucp_h {
    pub type C2RustUnnamed = ::core::ffi::c_uint;
    pub const ucp_Z: C2RustUnnamed = 6;
    pub const ucp_S: C2RustUnnamed = 5;
    pub const ucp_P: C2RustUnnamed = 4;
    pub const ucp_N: C2RustUnnamed = 3;
    pub const ucp_M: C2RustUnnamed = 2;
    pub const ucp_L: C2RustUnnamed = 1;
    pub const ucp_C: C2RustUnnamed = 0;
    pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
    pub const ucp_Zs: C2RustUnnamed_0 = 29;
    pub const ucp_Zp: C2RustUnnamed_0 = 28;
    pub const ucp_Zl: C2RustUnnamed_0 = 27;
    pub const ucp_So: C2RustUnnamed_0 = 26;
    pub const ucp_Sm: C2RustUnnamed_0 = 25;
    pub const ucp_Sk: C2RustUnnamed_0 = 24;
    pub const ucp_Sc: C2RustUnnamed_0 = 23;
    pub const ucp_Ps: C2RustUnnamed_0 = 22;
    pub const ucp_Po: C2RustUnnamed_0 = 21;
    pub const ucp_Pi: C2RustUnnamed_0 = 20;
    pub const ucp_Pf: C2RustUnnamed_0 = 19;
    pub const ucp_Pe: C2RustUnnamed_0 = 18;
    pub const ucp_Pd: C2RustUnnamed_0 = 17;
    pub const ucp_Pc: C2RustUnnamed_0 = 16;
    pub const ucp_No: C2RustUnnamed_0 = 15;
    pub const ucp_Nl: C2RustUnnamed_0 = 14;
    pub const ucp_Nd: C2RustUnnamed_0 = 13;
    pub const ucp_Mn: C2RustUnnamed_0 = 12;
    pub const ucp_Me: C2RustUnnamed_0 = 11;
    pub const ucp_Mc: C2RustUnnamed_0 = 10;
    pub const ucp_Lu: C2RustUnnamed_0 = 9;
    pub const ucp_Lt: C2RustUnnamed_0 = 8;
    pub const ucp_Lo: C2RustUnnamed_0 = 7;
    pub const ucp_Lm: C2RustUnnamed_0 = 6;
    pub const ucp_Ll: C2RustUnnamed_0 = 5;
    pub const ucp_Cs: C2RustUnnamed_0 = 4;
    pub const ucp_Co: C2RustUnnamed_0 = 3;
    pub const ucp_Cn: C2RustUnnamed_0 = 2;
    pub const ucp_Cf: C2RustUnnamed_0 = 1;
    pub const ucp_Cc: C2RustUnnamed_0 = 0;
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
pub mod config_h {
    pub const LINK_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::config_h::LINK_SIZE;
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::__va_list_tag;
pub use self::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
pub use self::pcre2_internal_h::{
    _pcre2_ucd_boolprop_sets_8, _pcre2_ucd_records_8, _pcre2_ucd_script_sets_8,
    _pcre2_ucd_stage1_8, _pcre2_ucd_stage2_8, _pcre2_ucp_gentype_8, ucd_record, CHAR_a, CHAR_f,
    BOOL, CHAR_0, CHAR_9, CHAR_A, CHAR_COMMERCIAL_AT, CHAR_CR, CHAR_DOLLAR_SIGN, CHAR_F, CHAR_FF,
    CHAR_GRAVE_ACCENT, CHAR_HT, CHAR_LF, CHAR_NBSP, CHAR_NEL, CHAR_SPACE, CHAR_VT, ECL_AND,
    ECL_MAP, ECL_NOT, ECL_OR, ECL_XCLASS, ECL_XOR, FALSE, PT_ALNUM, PT_BIDICL, PT_BOOL, PT_GC,
    PT_LAMP, PT_PC, PT_PXGRAPH, PT_PXPRINT, PT_PXPUNCT, PT_PXSPACE, PT_PXXDIGIT, PT_SC, PT_SCX,
    PT_SPACE, PT_UCNC, PT_WORD, TRUE, UCD_BIDICLASS_SHIFT, UCD_BLOCK_SIZE, XCL_BEGIN_WITH_RANGE,
    XCL_CHAR_END, XCL_CHAR_LIST_HIGH_16_START, XCL_CHAR_LIST_LOW_32_START, XCL_CHAR_SHIFT, XCL_END,
    XCL_ITEM_COUNT_MASK, XCL_MAP, XCL_NOT, XCL_NOTPROP, XCL_PROP, XCL_SINGLE, XCL_TYPE_BIT_LEN,
    XCL_TYPE_MASK,
};
pub use self::pcre2_ucp_h::{
    ucp_C, ucp_Cc, ucp_Cf, ucp_Cn, ucp_Co, ucp_Cs, ucp_L, ucp_Ll, ucp_Lm, ucp_Lo, ucp_Lt, ucp_Lu,
    ucp_M, ucp_Mc, ucp_Me, ucp_Mn, ucp_N, ucp_Nd, ucp_Nl, ucp_No, ucp_P, ucp_Pc, ucp_Pd, ucp_Pe,
    ucp_Pf, ucp_Pi, ucp_Po, ucp_Ps, ucp_S, ucp_Sc, ucp_Sk, ucp_Sm, ucp_So, ucp_Z, ucp_Zl, ucp_Zp,
    ucp_Zs, C2RustUnnamed, C2RustUnnamed_0,
};
pub use self::stddef_h::{size_t, NULL};
pub use self::stdint_intn_h::int32_t;
pub use self::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
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
pub unsafe extern "C" fn _pcre2_xclass_8(
    mut c: uint32_t,
    mut data: PCRE2_SPTR8,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut t: PCRE2_UCHAR8 = 0;
    let mut not_negated: BOOL =
        (*data as ::core::ffi::c_int & XCL_NOT == 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let mut type_0: uint32_t = 0;
    let mut max_index: uint32_t = 0;
    let mut min_index: uint32_t = 0;
    let mut value: uint32_t = 0;
    let mut next_char: *const uint8_t = ::core::ptr::null::<uint8_t>();
    utf = TRUE as BOOL;
    let fresh6 = data;
    data = data.offset(1);
    if *fresh6 as ::core::ffi::c_int & XCL_MAP != 0 as ::core::ffi::c_int {
        if c < 256 as uint32_t {
            return (*(data as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        data = data.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize) as isize,
        );
    }
    if *data as ::core::ffi::c_int == XCL_PROP || *data as ::core::ffi::c_int == XCL_NOTPROP {
        let mut prop: *const ucd_record = (&raw const _pcre2_ucd_records_8 as *const ucd_record)
            .offset(
                *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                    (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                        .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                        as ::core::ffi::c_int
                        * UCD_BLOCK_SIZE
                        + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
                ) as ::core::ffi::c_int as isize,
            );
        loop {
            let mut chartype: ::core::ffi::c_int = 0;
            let fresh7 = data;
            data = data.offset(1);
            let mut isprop: BOOL =
                (*fresh7 as ::core::ffi::c_int == XCL_PROP) as ::core::ffi::c_int;
            let mut ok: BOOL = 0;
            match *data as ::core::ffi::c_int {
                PT_LAMP => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (chartype == ucp_Lu as ::core::ffi::c_int
                        || chartype == ucp_Ll as ::core::ffi::c_int
                        || chartype == ucp_Lt as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_GC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                        == *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset((*prop).chartype as isize))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).chartype as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SC => {
                    if (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).script as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SCX => {
                    ok = (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == (*prop).script as ::core::ffi::c_int
                        || *(&raw const _pcre2_ucd_script_sets_8 as *const uint32_t)
                            .offset(
                                ((*prop).scriptx_bidiclass as ::core::ffi::c_int
                                    & 0x3ff as ::core::ffi::c_int)
                                    as isize,
                            )
                            .offset(
                                (*data.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    / 32 as ::core::ffi::c_int)
                                    as isize,
                            )
                            & (1 as uint32_t)
                                << *data.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                    % 32 as ::core::ffi::c_int
                            != 0 as uint32_t) as ::core::ffi::c_int
                        as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                PT_ALNUM => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset(chartype as isize)
                            == ucp_N as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_SPACE | PT_PXSPACE => match c {
                    9 | 32 | 160 | 5760 | 6158 | 8192 | 8193 | 8194 | 8195 | 8196 | 8197 | 8198
                    | 8199 | 8200 | 8201 | 8202 | 8239 | 8287 | 12288 | 10 | 11 | 12 | 13 | 133
                    | 8232 | 8233 => {
                        if isprop != 0 {
                            return not_negated;
                        }
                    }
                    _ => {
                        if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset((*prop).chartype as isize)
                            == ucp_Z as ::core::ffi::c_int as uint32_t)
                            as ::core::ffi::c_int
                            == isprop
                        {
                            return not_negated;
                        }
                    }
                },
                PT_WORD => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_L as ::core::ffi::c_int as uint32_t
                        || *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset(chartype as isize)
                            == ucp_N as ::core::ffi::c_int as uint32_t
                        || chartype == ucp_Mn as ::core::ffi::c_int
                        || chartype == ucp_Pc as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_UCNC => {
                    if c < 0xa0 as uint32_t {
                        if (c == CHAR_DOLLAR_SIGN as uint32_t
                            || c == CHAR_COMMERCIAL_AT as uint32_t
                            || c == CHAR_GRAVE_ACCENT as uint32_t)
                            as ::core::ffi::c_int
                            == isprop
                        {
                            return not_negated;
                        }
                    } else if (c < 0xd800 as uint32_t || c > 0xdfff as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_BIDICL => {
                    if ((*prop).scriptx_bidiclass as ::core::ffi::c_int >> UCD_BIDICLASS_SHIFT
                        == *data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_BOOL => {
                    ok = (*(&raw const _pcre2_ucd_boolprop_sets_8 as *const uint32_t)
                        .offset(
                            ((*prop).bprops as ::core::ffi::c_int & 0xfff as ::core::ffi::c_int)
                                as isize,
                        )
                        .offset(
                            (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                / 32 as ::core::ffi::c_int) as isize,
                        )
                        & (1 as uint32_t)
                            << *data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                % 32 as ::core::ffi::c_int
                        != 0 as uint32_t) as ::core::ffi::c_int as BOOL;
                    if ok == isprop {
                        return not_negated;
                    }
                }
                PT_PXGRAPH => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        != ucp_Z as ::core::ffi::c_int as uint32_t
                        && (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset(chartype as isize)
                            != ucp_C as ::core::ffi::c_int as uint32_t
                            || chartype == ucp_Cf as ::core::ffi::c_int
                                && c != 0x61c as uint32_t
                                && c != 0x180e as uint32_t
                                && (c < 0x2066 as uint32_t || c > 0x2069 as uint32_t)))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PXPRINT => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (chartype != ucp_Zl as ::core::ffi::c_int
                        && chartype != ucp_Zp as ::core::ffi::c_int
                        && (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                            .offset(chartype as isize)
                            != ucp_C as ::core::ffi::c_int as uint32_t
                            || chartype == ucp_Cf as ::core::ffi::c_int
                                && c != 0x61c as uint32_t
                                && (c < 0x2066 as uint32_t || c > 0x2069 as uint32_t)))
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PXPUNCT => {
                    chartype = (*prop).chartype as ::core::ffi::c_int;
                    if (*(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                        .offset(chartype as isize)
                        == ucp_P as ::core::ffi::c_int as uint32_t
                        || c < 128 as uint32_t
                            && *(&raw const _pcre2_ucp_gentype_8 as *const uint32_t)
                                .offset(chartype as isize)
                                == ucp_S as ::core::ffi::c_int as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                PT_PXXDIGIT => {
                    if (c >= CHAR_0 as uint32_t && c <= CHAR_9 as uint32_t
                        || c >= CHAR_A as uint32_t && c <= CHAR_F as uint32_t
                        || c >= CHAR_a as uint32_t && c <= CHAR_f as uint32_t
                        || c >= 0xff10 as uint32_t && c <= 0xff19 as uint32_t
                        || c >= 0xff21 as uint32_t && c <= 0xff26 as uint32_t
                        || c >= 0xff41 as uint32_t && c <= 0xff46 as uint32_t)
                        as ::core::ffi::c_int
                        == isprop
                    {
                        return not_negated;
                    }
                }
                _ => return FALSE,
            }
            data = data.offset(2 as ::core::ffi::c_int as isize);
            if !(*data as ::core::ffi::c_int == XCL_PROP
                || *data as ::core::ffi::c_int == XCL_NOTPROP)
            {
                break;
            }
        }
    }
    if (*data as ::core::ffi::c_int)
        < (if ::core::mem::size_of::<PCRE2_UCHAR8>() as usize == 1 as usize {
            0x10 as ::core::ffi::c_int
        } else {
            0x1000 as ::core::ffi::c_int
        })
    {
        loop {
            let fresh8 = data;
            data = data.offset(1);
            t = *fresh8;
            if !(t as ::core::ffi::c_int != XCL_END) {
                break;
            }
            let mut x: uint32_t = 0;
            let mut y: uint32_t = 0;
            if utf != 0 {
                let fresh9 = data;
                data = data.offset(1);
                x = *fresh9 as uint32_t;
                if x >= 0xc0 as uint32_t {
                    if x & 0x20 as uint32_t == 0 as uint32_t {
                        let fresh10 = data;
                        data = data.offset(1);
                        x = (x & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                            | *fresh10 as uint32_t & 0x3f as uint32_t;
                    } else if x & 0x10 as uint32_t == 0 as uint32_t {
                        x = (x & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                            | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(2 as ::core::ffi::c_int as isize);
                    } else if x & 0x8 as uint32_t == 0 as uint32_t {
                        x = (x & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 12 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(3 as ::core::ffi::c_int as isize);
                    } else if x & 0x4 as uint32_t == 0 as uint32_t {
                        x = (x & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 18 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 12 as ::core::ffi::c_int
                            | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(4 as ::core::ffi::c_int as isize);
                    } else {
                        x = (x & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                            | (*data as uint32_t & 0x3f as uint32_t) << 24 as ::core::ffi::c_int
                            | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 18 as ::core::ffi::c_int
                            | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 12 as ::core::ffi::c_int
                            | (*data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t)
                                << 6 as ::core::ffi::c_int
                            | *data.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                & 0x3f as uint32_t;
                        data = data.offset(5 as ::core::ffi::c_int as isize);
                    }
                }
            } else {
                let fresh11 = data;
                data = data.offset(1);
                x = *fresh11 as uint32_t;
            }
            if t as ::core::ffi::c_int == XCL_SINGLE {
                if c <= x {
                    return if c == x {
                        not_negated
                    } else {
                        (not_negated == 0) as ::core::ffi::c_int
                    };
                }
            } else {
                if utf != 0 {
                    let fresh12 = data;
                    data = data.offset(1);
                    y = *fresh12 as uint32_t;
                    if y >= 0xc0 as uint32_t {
                        if y & 0x20 as uint32_t == 0 as uint32_t {
                            let fresh13 = data;
                            data = data.offset(1);
                            y = (y & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                                | *fresh13 as uint32_t & 0x3f as uint32_t;
                        } else if y & 0x10 as uint32_t == 0 as uint32_t {
                            y = (y & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t) << 6 as ::core::ffi::c_int
                                | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(2 as ::core::ffi::c_int as isize);
                        } else if y & 0x8 as uint32_t == 0 as uint32_t {
                            y = (y & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(3 as ::core::ffi::c_int as isize);
                        } else if y & 0x4 as uint32_t == 0 as uint32_t {
                            y = (y & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(4 as ::core::ffi::c_int as isize);
                        } else {
                            y = (y & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                                | (*data as uint32_t & 0x3f as uint32_t)
                                    << 24 as ::core::ffi::c_int
                                | (*data.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*data.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*data.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *data.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                            data = data.offset(5 as ::core::ffi::c_int as isize);
                        }
                    }
                } else {
                    let fresh14 = data;
                    data = data.offset(1);
                    y = *fresh14 as uint32_t;
                }
                if c <= y {
                    return if c >= x {
                        not_negated
                    } else {
                        (not_negated == 0) as ::core::ffi::c_int
                    };
                }
            }
        }
        return (not_negated == 0) as ::core::ffi::c_int;
    }
    type_0 = ((*data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
        << 8 as ::core::ffi::c_int) as uint32_t
        | *data.offset(1 as ::core::ffi::c_int as isize) as uint32_t;
    data = data.offset(2 as ::core::ffi::c_int as isize);
    next_char = char_lists_end.offset(
        -(((((*data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *data.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as ::core::ffi::c_uint)
            << 1 as ::core::ffi::c_int) as isize),
    );
    type_0 = (type_0 as ::core::ffi::c_uint & XCL_TYPE_MASK as ::core::ffi::c_uint) as uint32_t;
    if c >= XCL_CHAR_LIST_HIGH_16_START as uint32_t {
        max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
        if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
            max_index = *(next_char as *const uint16_t) as uint32_t;
            next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
        }
        next_char = next_char.offset((max_index << 1 as ::core::ffi::c_int) as isize);
        type_0 >>= XCL_TYPE_BIT_LEN;
    }
    if c < XCL_CHAR_LIST_LOW_32_START as uint32_t {
        max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
        c = (c << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t) as uint16_t as uint32_t;
        if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
            max_index = *(next_char as *const uint16_t) as uint32_t;
            next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
        }
        if max_index == 0 as uint32_t || c < *(next_char as *const uint16_t) as uint32_t {
            return ((type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
        min_index = 0 as uint32_t;
        max_index = max_index.wrapping_sub(1);
        value = *(next_char as *const uint16_t).offset(max_index as isize) as uint32_t;
        if c >= value {
            return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
        max_index = max_index.wrapping_sub(1);
        loop {
            let mut mid_index: uint32_t =
                min_index.wrapping_add(max_index) >> 1 as ::core::ffi::c_int;
            value = *(next_char as *const uint16_t).offset(mid_index as isize) as uint32_t;
            if c < value {
                max_index = mid_index.wrapping_sub(1 as uint32_t);
            } else if *(next_char as *const uint16_t)
                .offset(mid_index.wrapping_add(1 as uint32_t) as isize)
                as uint32_t
                <= c
            {
                min_index = mid_index.wrapping_add(1 as uint32_t);
            } else {
                return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                    as ::core::ffi::c_int
                    == not_negated) as ::core::ffi::c_int;
            }
        }
    }
    max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
    if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
        max_index = *(next_char as *const uint16_t) as uint32_t;
        next_char = next_char.offset(2 as ::core::ffi::c_int as isize);
    }
    next_char = next_char.offset((max_index << 1 as ::core::ffi::c_int) as isize);
    type_0 >>= XCL_TYPE_BIT_LEN;
    max_index = type_0 & XCL_ITEM_COUNT_MASK as uint32_t;
    c = c << XCL_CHAR_SHIFT | XCL_CHAR_END as uint32_t;
    if max_index == XCL_ITEM_COUNT_MASK as uint32_t {
        max_index = *(next_char as *const uint32_t);
        next_char = next_char.offset(4 as ::core::ffi::c_int as isize);
    }
    if max_index == 0 as uint32_t || c < *(next_char as *const uint32_t) {
        return ((type_0 & XCL_BEGIN_WITH_RANGE as uint32_t != 0 as uint32_t) as ::core::ffi::c_int
            == not_negated) as ::core::ffi::c_int;
    }
    min_index = 0 as uint32_t;
    max_index = max_index.wrapping_sub(1);
    value = *(next_char as *const uint32_t).offset(max_index as isize);
    if c >= value {
        return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
            as ::core::ffi::c_int
            == not_negated) as ::core::ffi::c_int;
    }
    max_index = max_index.wrapping_sub(1);
    loop {
        let mut mid_index_0: uint32_t =
            min_index.wrapping_add(max_index) >> 1 as ::core::ffi::c_int;
        value = *(next_char as *const uint32_t).offset(mid_index_0 as isize);
        if c < value {
            max_index = mid_index_0.wrapping_sub(1 as uint32_t);
        } else if *(next_char as *const uint32_t)
            .offset(mid_index_0.wrapping_add(1 as uint32_t) as isize)
            <= c
        {
            min_index = mid_index_0.wrapping_add(1 as uint32_t);
        } else {
            return ((value == c || value & XCL_CHAR_END as uint32_t == 0 as uint32_t)
                as ::core::ffi::c_int
                == not_negated) as ::core::ffi::c_int;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_eclass_8(
    mut c: uint32_t,
    mut data_start: PCRE2_SPTR8,
    mut data_end: PCRE2_SPTR8,
    mut char_lists_end: *const uint8_t,
    mut utf: BOOL,
) -> BOOL {
    let mut ptr: PCRE2_SPTR8 = data_start;
    let mut flags: PCRE2_UCHAR8 = 0;
    let mut stack: uint32_t = 0 as uint32_t;
    let mut stack_depth: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let fresh15 = ptr;
    ptr = ptr.offset(1);
    flags = *fresh15;
    if flags as ::core::ffi::c_int & ECL_MAP != 0 as ::core::ffi::c_int {
        if c < 256 as uint32_t {
            return (*(ptr as *const uint8_t).offset(c.wrapping_div(8 as uint32_t) as isize)
                as ::core::ffi::c_uint
                & (1 as ::core::ffi::c_uint) << (c & 7 as uint32_t)
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        }
        ptr = ptr.offset(
            (32 as usize).wrapping_div(::core::mem::size_of::<PCRE2_UCHAR8>() as usize) as isize,
        );
    }
    while ptr < data_end {
        match *ptr as ::core::ffi::c_int {
            ECL_AND => {
                ptr = ptr.offset(1);
                stack = stack >> 1 as ::core::ffi::c_int
                    & (stack | !(1 as ::core::ffi::c_uint as uint32_t));
                stack_depth -= 1;
            }
            ECL_OR => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int | stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            ECL_XOR => {
                ptr = ptr.offset(1);
                stack =
                    stack >> 1 as ::core::ffi::c_int ^ stack & 1 as ::core::ffi::c_uint as uint32_t;
                stack_depth -= 1;
            }
            ECL_NOT => {
                ptr = ptr.offset(1);
                stack = (stack as ::core::ffi::c_uint
                    ^ 1 as ::core::ffi::c_uint as uint32_t as ::core::ffi::c_uint)
                    as uint32_t;
            }
            ECL_XCLASS => {
                let mut matched: uint32_t = _pcre2_xclass_8(
                    c,
                    ptr.offset(1 as ::core::ffi::c_int as isize)
                        .offset(LINK_SIZE as isize),
                    char_lists_end,
                    utf,
                ) as uint32_t;
                ptr = ptr.offset(
                    ((*ptr.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int
                        | *ptr.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int) as ::core::ffi::c_uint
                        as isize,
                );
                stack = stack << 1 as ::core::ffi::c_int | matched;
                stack_depth += 1;
            }
            _ => return FALSE,
        }
    }
    return (stack & 1 as uint32_t != 0 as uint32_t) as ::core::ffi::c_int;
}
