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
    pub const UCD_BLOCK_SIZE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
    use super::stdint_intn_h::int32_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
    extern "C" {
        pub static _pcre2_ucd_records_8: [ucd_record; 0];
        pub static _pcre2_ucd_stage1_8: [uint16_t; 0];
        pub static _pcre2_ucd_stage2_8: [uint16_t; 0];
        pub static _pcre2_ucp_gbtable_8: [uint32_t; 0];
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
    pub const ucp_gbExtended_Pictographic: C2RustUnnamed = 14;
    pub const ucp_gbZWJ: C2RustUnnamed = 13;
    pub const ucp_gbOther: C2RustUnnamed = 12;
    pub const ucp_gbRegional_Indicator: C2RustUnnamed = 11;
    pub const ucp_gbLVT: C2RustUnnamed = 10;
    pub const ucp_gbLV: C2RustUnnamed = 9;
    pub const ucp_gbT: C2RustUnnamed = 8;
    pub const ucp_gbV: C2RustUnnamed = 7;
    pub const ucp_gbL: C2RustUnnamed = 6;
    pub const ucp_gbSpacingMark: C2RustUnnamed = 5;
    pub const ucp_gbPrepend: C2RustUnnamed = 4;
    pub const ucp_gbExtend: C2RustUnnamed = 3;
    pub const ucp_gbControl: C2RustUnnamed = 2;
    pub const ucp_gbLF: C2RustUnnamed = 1;
    pub const ucp_gbCR: C2RustUnnamed = 0;
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
pub use self::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
pub use self::pcre2_internal_h::{
    _pcre2_ucd_records_8, _pcre2_ucd_stage1_8, _pcre2_ucd_stage2_8, _pcre2_ucp_gbtable_8,
    ucd_record, BOOL, FALSE, UCD_BLOCK_SIZE,
};
pub use self::pcre2_ucp_h::{
    ucp_gbCR, ucp_gbControl, ucp_gbExtend, ucp_gbExtended_Pictographic, ucp_gbL, ucp_gbLF,
    ucp_gbLV, ucp_gbLVT, ucp_gbOther, ucp_gbPrepend, ucp_gbRegional_Indicator, ucp_gbSpacingMark,
    ucp_gbT, ucp_gbV, ucp_gbZWJ, C2RustUnnamed,
};
pub use self::stddef_h::{size_t, NULL, NULL_0};
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
pub unsafe extern "C" fn _pcre2_extuni_8(
    mut c: uint32_t,
    mut eptr: PCRE2_SPTR8,
    mut start_subject: PCRE2_SPTR8,
    mut end_subject: PCRE2_SPTR8,
    mut utf: BOOL,
    mut xcount: *mut ::core::ffi::c_int,
) -> PCRE2_SPTR8 {
    let mut was_ep_ZWJ: BOOL = FALSE;
    let mut lgb: ::core::ffi::c_int = (*(&raw const _pcre2_ucd_records_8 as *const ucd_record)
        .offset(
            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                    .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                    as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        ))
    .gbprop as ::core::ffi::c_int;
    while eptr < end_subject {
        let mut rgb: ::core::ffi::c_int = 0;
        let mut len: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
        if utf == 0 {
            c = *eptr as uint32_t;
        } else {
            c = *eptr as uint32_t;
            if c >= 0xc0 as uint32_t {
                if c & 0x20 as uint32_t == 0 as uint32_t {
                    c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                        | *eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    len += 1;
                } else if c & 0x10 as uint32_t == 0 as uint32_t {
                    c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                        | (*eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *eptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    len += 2 as ::core::ffi::c_int;
                } else if c & 0x8 as uint32_t == 0 as uint32_t {
                    c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                        | (*eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*eptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *eptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    len += 3 as ::core::ffi::c_int;
                } else if c & 0x4 as uint32_t == 0 as uint32_t {
                    c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                        | (*eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 18 as ::core::ffi::c_int
                        | (*eptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*eptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *eptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    len += 4 as ::core::ffi::c_int;
                } else {
                    c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                        | (*eptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 24 as ::core::ffi::c_int
                        | (*eptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 18 as ::core::ffi::c_int
                        | (*eptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 12 as ::core::ffi::c_int
                        | (*eptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t)
                            << 6 as ::core::ffi::c_int
                        | *eptr.offset(5 as ::core::ffi::c_int as isize) as uint32_t
                            & 0x3f as uint32_t;
                    len += 5 as ::core::ffi::c_int;
                }
            }
        }
        rgb = (*(&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
            *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                    .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                    as ::core::ffi::c_int
                    * UCD_BLOCK_SIZE
                    + c as ::core::ffi::c_int % UCD_BLOCK_SIZE) as isize,
            ) as ::core::ffi::c_int as isize,
        ))
        .gbprop as ::core::ffi::c_int;
        if *(&raw const _pcre2_ucp_gbtable_8 as *const uint32_t).offset(lgb as isize)
            & (1 as uint32_t) << rgb
            == 0 as uint32_t
        {
            break;
        }
        if lgb == ucp_gbZWJ as ::core::ffi::c_int
            && rgb == ucp_gbExtended_Pictographic as ::core::ffi::c_int
            && was_ep_ZWJ == 0
        {
            break;
        }
        if lgb == ucp_gbRegional_Indicator as ::core::ffi::c_int
            && rgb == ucp_gbRegional_Indicator as ::core::ffi::c_int
        {
            let mut ricount: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut bptr: PCRE2_SPTR8 = eptr.offset(-(1 as ::core::ffi::c_int as isize));
            if utf != 0 {
                while *bptr as ::core::ffi::c_uint & 0xc0 as ::core::ffi::c_uint
                    == 0x80 as ::core::ffi::c_uint
                {
                    bptr = bptr.offset(-1);
                }
            }
            while bptr > start_subject {
                bptr = bptr.offset(-1);
                if utf != 0 {
                    while *bptr as ::core::ffi::c_uint & 0xc0 as ::core::ffi::c_uint
                        == 0x80 as ::core::ffi::c_uint
                    {
                        bptr = bptr.offset(-1);
                    }
                    c = *bptr as uint32_t;
                    if c >= 0xc0 as uint32_t {
                        if c & 0x20 as uint32_t == 0 as uint32_t {
                            c = (c & 0x1f as uint32_t) << 6 as ::core::ffi::c_int
                                | *bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                        } else if c & 0x10 as uint32_t == 0 as uint32_t {
                            c = (c & 0xf as uint32_t) << 12 as ::core::ffi::c_int
                                | (*bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *bptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                        } else if c & 0x8 as uint32_t == 0 as uint32_t {
                            c = (c & 0x7 as uint32_t) << 18 as ::core::ffi::c_int
                                | (*bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*bptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *bptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                        } else if c & 0x4 as uint32_t == 0 as uint32_t {
                            c = (c & 0x3 as uint32_t) << 24 as ::core::ffi::c_int
                                | (*bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*bptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*bptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *bptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                        } else {
                            c = (c & 0x1 as uint32_t) << 30 as ::core::ffi::c_int
                                | (*bptr.offset(1 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 24 as ::core::ffi::c_int
                                | (*bptr.offset(2 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 18 as ::core::ffi::c_int
                                | (*bptr.offset(3 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 12 as ::core::ffi::c_int
                                | (*bptr.offset(4 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t)
                                    << 6 as ::core::ffi::c_int
                                | *bptr.offset(5 as ::core::ffi::c_int as isize) as uint32_t
                                    & 0x3f as uint32_t;
                        }
                    }
                } else {
                    c = *bptr as uint32_t;
                }
                if (*(&raw const _pcre2_ucd_records_8 as *const ucd_record).offset(
                    *(&raw const _pcre2_ucd_stage2_8 as *const uint16_t).offset(
                        (*(&raw const _pcre2_ucd_stage1_8 as *const uint16_t)
                            .offset((c as ::core::ffi::c_int / UCD_BLOCK_SIZE) as isize)
                            as ::core::ffi::c_int
                            * UCD_BLOCK_SIZE
                            + c as ::core::ffi::c_int % UCD_BLOCK_SIZE)
                            as isize,
                    ) as ::core::ffi::c_int as isize,
                ))
                .gbprop as ::core::ffi::c_int
                    != ucp_gbRegional_Indicator as ::core::ffi::c_int
                {
                    break;
                }
                ricount += 1;
            }
            if ricount & 1 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                break;
            }
        }
        was_ep_ZWJ = (lgb == ucp_gbExtended_Pictographic as ::core::ffi::c_int
            && rgb == ucp_gbZWJ as ::core::ffi::c_int) as ::core::ffi::c_int
            as BOOL;
        if rgb != ucp_gbExtend as ::core::ffi::c_int
            || lgb != ucp_gbExtended_Pictographic as ::core::ffi::c_int
        {
            lgb = rgb;
        }
        eptr = eptr.offset(len as isize);
        if !xcount.is_null() {
            *xcount += 1 as ::core::ffi::c_int;
        }
    }
    return eptr;
}
