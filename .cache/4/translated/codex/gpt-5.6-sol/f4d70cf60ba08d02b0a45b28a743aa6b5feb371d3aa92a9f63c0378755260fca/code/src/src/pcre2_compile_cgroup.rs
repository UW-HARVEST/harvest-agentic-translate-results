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
pub mod pcre2_internal_h {
    pub type BOOL = ::core::ffi::c_int;
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
    pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    use super::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
    use super::stddef_h::size_t;
    extern "C" {
        pub fn _pcre2_strncmp_8(_: PCRE2_SPTR8, _: PCRE2_SPTR8, _: size_t) -> ::core::ffi::c_int;
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
pub mod pcre2_intmodedep_h {
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
    pub struct named_group_8 {
        pub name: PCRE2_SPTR8,
        pub number: uint32_t,
        pub length: uint16_t,
        pub hash_dup: uint16_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct compile_data {
        pub next: *mut compile_data,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct recurse_arguments {
        pub header: compile_data,
        pub size: size_t,
        pub skip_size: size_t,
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub union class_bits_storage {
        pub classbits: [uint8_t; 32],
        pub classwords: [uint32_t; 8],
    }
    #[derive(Copy, Clone)]
    #[repr(C)]
    pub struct compile_block_8 {
        pub cx: *mut pcre2_real_compile_context_8,
        pub lcc: *const uint8_t,
        pub fcc: *const uint8_t,
        pub cbits: *const uint8_t,
        pub ctypes: *const uint8_t,
        pub start_workspace: *mut PCRE2_UCHAR8,
        pub start_code: *mut PCRE2_UCHAR8,
        pub start_pattern: PCRE2_SPTR8,
        pub end_pattern: PCRE2_SPTR8,
        pub name_table: *mut PCRE2_UCHAR8,
        pub workspace_size: size_t,
        pub small_ref_offset: [size_t; 10],
        pub erroroffset: size_t,
        pub classbits: class_bits_storage,
        pub names_found: uint16_t,
        pub name_entry_size: uint16_t,
        pub parens_depth: uint16_t,
        pub assert_depth: uint16_t,
        pub named_groups: *mut named_group_8,
        pub named_group_list_size: uint32_t,
        pub external_options: uint32_t,
        pub external_flags: uint32_t,
        pub bracount: uint32_t,
        pub lastcapture: uint32_t,
        pub parsed_pattern: *mut uint32_t,
        pub parsed_pattern_end: *mut uint32_t,
        pub groupinfo: *mut uint32_t,
        pub top_backref: uint32_t,
        pub backref_map: uint32_t,
        pub nltype: uint32_t,
        pub nllen: uint32_t,
        pub nl: [PCRE2_UCHAR8; 4],
        pub class_op_used: [uint8_t; 15],
        pub req_varyopt: uint32_t,
        pub max_varlookbehind: uint32_t,
        pub max_lookbehind: ::core::ffi::c_int,
        pub had_accept: BOOL,
        pub had_pruneorskip: BOOL,
        pub had_recurse: BOOL,
        pub dupnames: BOOL,
        pub first_data: *mut compile_data,
        pub last_data: *mut compile_data,
        pub char_lists_size: size_t,
    }
    pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
    use super::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
    use super::pcre2_internal_h::{pcre2_memctl, BOOL};
    use super::stddef_h::size_t;
    use super::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
}
pub mod pcre2_compile_h {
    pub type C2RustUnnamed = ::core::ffi::c_uint;
    pub const ERR120: C2RustUnnamed = 220;
    pub const ERR119: C2RustUnnamed = 219;
    pub const ERR118: C2RustUnnamed = 218;
    pub const ERR117: C2RustUnnamed = 217;
    pub const ERR116: C2RustUnnamed = 216;
    pub const ERR115: C2RustUnnamed = 215;
    pub const ERR114: C2RustUnnamed = 214;
    pub const ERR113: C2RustUnnamed = 213;
    pub const ERR112: C2RustUnnamed = 212;
    pub const ERR111: C2RustUnnamed = 211;
    pub const ERR110: C2RustUnnamed = 210;
    pub const ERR109: C2RustUnnamed = 209;
    pub const ERR108: C2RustUnnamed = 208;
    pub const ERR107: C2RustUnnamed = 207;
    pub const ERR106: C2RustUnnamed = 206;
    pub const ERR105: C2RustUnnamed = 205;
    pub const ERR104: C2RustUnnamed = 204;
    pub const ERR103: C2RustUnnamed = 203;
    pub const ERR102: C2RustUnnamed = 202;
    pub const ERR101: C2RustUnnamed = 201;
    pub const ERR100: C2RustUnnamed = 200;
    pub const ERR99: C2RustUnnamed = 199;
    pub const ERR98: C2RustUnnamed = 198;
    pub const ERR97: C2RustUnnamed = 197;
    pub const ERR96: C2RustUnnamed = 196;
    pub const ERR95: C2RustUnnamed = 195;
    pub const ERR94: C2RustUnnamed = 194;
    pub const ERR93: C2RustUnnamed = 193;
    pub const ERR92: C2RustUnnamed = 192;
    pub const ERR91: C2RustUnnamed = 191;
    pub const ERR90: C2RustUnnamed = 190;
    pub const ERR89: C2RustUnnamed = 189;
    pub const ERR88: C2RustUnnamed = 188;
    pub const ERR87: C2RustUnnamed = 187;
    pub const ERR86: C2RustUnnamed = 186;
    pub const ERR85: C2RustUnnamed = 185;
    pub const ERR84: C2RustUnnamed = 184;
    pub const ERR83: C2RustUnnamed = 183;
    pub const ERR82: C2RustUnnamed = 182;
    pub const ERR81: C2RustUnnamed = 181;
    pub const ERR80: C2RustUnnamed = 180;
    pub const ERR79: C2RustUnnamed = 179;
    pub const ERR78: C2RustUnnamed = 178;
    pub const ERR77: C2RustUnnamed = 177;
    pub const ERR76: C2RustUnnamed = 176;
    pub const ERR75: C2RustUnnamed = 175;
    pub const ERR74: C2RustUnnamed = 174;
    pub const ERR73: C2RustUnnamed = 173;
    pub const ERR72: C2RustUnnamed = 172;
    pub const ERR71: C2RustUnnamed = 171;
    pub const ERR70: C2RustUnnamed = 170;
    pub const ERR69: C2RustUnnamed = 169;
    pub const ERR68: C2RustUnnamed = 168;
    pub const ERR67: C2RustUnnamed = 167;
    pub const ERR66: C2RustUnnamed = 166;
    pub const ERR65: C2RustUnnamed = 165;
    pub const ERR64: C2RustUnnamed = 164;
    pub const ERR63: C2RustUnnamed = 163;
    pub const ERR62: C2RustUnnamed = 162;
    pub const ERR61: C2RustUnnamed = 161;
    pub const ERR60: C2RustUnnamed = 160;
    pub const ERR59: C2RustUnnamed = 159;
    pub const ERR58: C2RustUnnamed = 158;
    pub const ERR57: C2RustUnnamed = 157;
    pub const ERR56: C2RustUnnamed = 156;
    pub const ERR55: C2RustUnnamed = 155;
    pub const ERR54: C2RustUnnamed = 154;
    pub const ERR53: C2RustUnnamed = 153;
    pub const ERR52: C2RustUnnamed = 152;
    pub const ERR51: C2RustUnnamed = 151;
    pub const ERR50: C2RustUnnamed = 150;
    pub const ERR49: C2RustUnnamed = 149;
    pub const ERR48: C2RustUnnamed = 148;
    pub const ERR47: C2RustUnnamed = 147;
    pub const ERR46: C2RustUnnamed = 146;
    pub const ERR45: C2RustUnnamed = 145;
    pub const ERR44: C2RustUnnamed = 144;
    pub const ERR43: C2RustUnnamed = 143;
    pub const ERR42: C2RustUnnamed = 142;
    pub const ERR41: C2RustUnnamed = 141;
    pub const ERR40: C2RustUnnamed = 140;
    pub const ERR39: C2RustUnnamed = 139;
    pub const ERR38: C2RustUnnamed = 138;
    pub const ERR37: C2RustUnnamed = 137;
    pub const ERR36: C2RustUnnamed = 136;
    pub const ERR35: C2RustUnnamed = 135;
    pub const ERR34: C2RustUnnamed = 134;
    pub const ERR33: C2RustUnnamed = 133;
    pub const ERR32: C2RustUnnamed = 132;
    pub const ERR31: C2RustUnnamed = 131;
    pub const ERR30: C2RustUnnamed = 130;
    pub const ERR29: C2RustUnnamed = 129;
    pub const ERR28: C2RustUnnamed = 128;
    pub const ERR27: C2RustUnnamed = 127;
    pub const ERR26: C2RustUnnamed = 126;
    pub const ERR25: C2RustUnnamed = 125;
    pub const ERR24: C2RustUnnamed = 124;
    pub const ERR23: C2RustUnnamed = 123;
    pub const ERR22: C2RustUnnamed = 122;
    pub const ERR21: C2RustUnnamed = 121;
    pub const ERR20: C2RustUnnamed = 120;
    pub const ERR19: C2RustUnnamed = 119;
    pub const ERR18: C2RustUnnamed = 118;
    pub const ERR17: C2RustUnnamed = 117;
    pub const ERR16: C2RustUnnamed = 116;
    pub const ERR15: C2RustUnnamed = 115;
    pub const ERR14: C2RustUnnamed = 114;
    pub const ERR13: C2RustUnnamed = 113;
    pub const ERR12: C2RustUnnamed = 112;
    pub const ERR11: C2RustUnnamed = 111;
    pub const ERR10: C2RustUnnamed = 110;
    pub const ERR9: C2RustUnnamed = 109;
    pub const ERR8: C2RustUnnamed = 108;
    pub const ERR7: C2RustUnnamed = 107;
    pub const ERR6: C2RustUnnamed = 106;
    pub const ERR5: C2RustUnnamed = 105;
    pub const ERR4: C2RustUnnamed = 104;
    pub const ERR3: C2RustUnnamed = 103;
    pub const ERR2: C2RustUnnamed = 102;
    pub const ERR1: C2RustUnnamed = 101;
    pub const ERR0: C2RustUnnamed = 100;
    pub const META_OFFSET: ::core::ffi::c_uint = 2148925440;
    pub const META_CAPTURE_NAME: ::core::ffi::c_uint = 2149056512;
    pub const META_CAPTURE_NUMBER: ::core::ffi::c_uint = 2149122048;
    pub const NAMED_GROUP_HASH_MASK: uint16_t = 0x7fff as ::core::ffi::c_int as uint16_t;
    pub const NAMED_GROUP_IS_DUPNAME: uint16_t = 0x8000 as ::core::ffi::c_int as uint16_t;
    use super::stdint_uintn_h::uint16_t;
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
        pub fn memmove(
            __dest: *mut ::core::ffi::c_void,
            __src: *const ::core::ffi::c_void,
            __n: size_t,
        ) -> *mut ::core::ffi::c_void;
        pub fn memset(
            __s: *mut ::core::ffi::c_void,
            __c: ::core::ffi::c_int,
            __n: size_t,
        ) -> *mut ::core::ffi::c_void;
        pub fn memcmp(
            __s1: *const ::core::ffi::c_void,
            __s2: *const ::core::ffi::c_void,
            __n: size_t,
        ) -> ::core::ffi::c_int;
    }
}
pub use self::bits_stdio_h::{
    feof_unlocked, ferror_unlocked, fgetc_unlocked, fputc_unlocked, getc_unlocked, getchar,
    getchar_unlocked, getline, putc_unlocked, putchar, putchar_unlocked, vprintf,
};
pub use self::byteswap_h::{__bswap_16, __bswap_32, __bswap_64};
pub use self::ctype_h::{__ctype_tolower_loc, __ctype_toupper_loc, tolower, toupper};
pub use self::internal::{__va_list_tag, PCRE2_CODE_UNIT_WIDTH};
pub use self::pcre2_compile_h::{
    C2RustUnnamed, ERR0, ERR1, ERR10, ERR100, ERR101, ERR102, ERR103, ERR104, ERR105, ERR106,
    ERR107, ERR108, ERR109, ERR11, ERR110, ERR111, ERR112, ERR113, ERR114, ERR115, ERR116, ERR117,
    ERR118, ERR119, ERR12, ERR120, ERR13, ERR14, ERR15, ERR16, ERR17, ERR18, ERR19, ERR2, ERR20,
    ERR21, ERR22, ERR23, ERR24, ERR25, ERR26, ERR27, ERR28, ERR29, ERR3, ERR30, ERR31, ERR32,
    ERR33, ERR34, ERR35, ERR36, ERR37, ERR38, ERR39, ERR4, ERR40, ERR41, ERR42, ERR43, ERR44,
    ERR45, ERR46, ERR47, ERR48, ERR49, ERR5, ERR50, ERR51, ERR52, ERR53, ERR54, ERR55, ERR56,
    ERR57, ERR58, ERR59, ERR6, ERR60, ERR61, ERR62, ERR63, ERR64, ERR65, ERR66, ERR67, ERR68,
    ERR69, ERR7, ERR70, ERR71, ERR72, ERR73, ERR74, ERR75, ERR76, ERR77, ERR78, ERR79, ERR8, ERR80,
    ERR81, ERR82, ERR83, ERR84, ERR85, ERR86, ERR87, ERR88, ERR89, ERR9, ERR90, ERR91, ERR92,
    ERR93, ERR94, ERR95, ERR96, ERR97, ERR98, ERR99, META_CAPTURE_NAME, META_CAPTURE_NUMBER,
    META_OFFSET, NAMED_GROUP_HASH_MASK, NAMED_GROUP_IS_DUPNAME,
};
pub use self::pcre2_h::{PCRE2_SPTR8, PCRE2_UCHAR8};
pub use self::pcre2_internal_h::{_pcre2_strncmp_8, pcre2_memctl, BOOL, FALSE, TRUE};
pub use self::pcre2_intmodedep_h::{
    class_bits_storage, compile_block_8, compile_data, named_group_8, pcre2_real_compile_context_8,
    recurse_arguments, IMM2_SIZE,
};
pub use self::stddef_h::{size_t, NULL, NULL_0};
pub use self::stdint_uintn_h::{uint16_t, uint32_t, uint8_t};
use self::stdio_h::{__getdelim, __overflow, __uflow, getc, putc, stdin, stdout, vfprintf};
pub use self::stdlib_bsearch_h::bsearch;
pub use self::stdlib_float_h::atof;
pub use self::stdlib_h::{__compar_fn_t, atoi, atol, atoll, strtod, strtol, strtoll};
use self::string_h::{memcmp, memcpy, memmove, memset};
pub use self::struct_FILE_h::{
    _IO_codecvt, _IO_lock_t, _IO_marker, _IO_wide_data, _IO_EOF_SEEN, _IO_ERR_SEEN, _IO_FILE,
};
pub use self::types_h::{
    __int32_t, __off64_t, __off_t, __ssize_t, __uint16_t, __uint32_t, __uint64_t, __uint8_t,
};
pub use self::uintn_identity_h::{__uint16_identity, __uint32_identity, __uint64_identity};
pub use self::FILE_h::FILE;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_get_hash_from_name8(
    mut name: PCRE2_SPTR8,
    mut length: uint32_t,
) -> uint16_t {
    let mut hash: uint16_t = 0;
    hash = (*name.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        & 0x7f as ::core::ffi::c_int
        | (*name.offset(length.wrapping_sub(1 as uint32_t) as isize) as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int)
            << 7 as ::core::ffi::c_int) as uint16_t;
    return hash;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_named_group8(
    mut name: PCRE2_SPTR8,
    mut length: uint32_t,
    mut cb: *mut compile_block_8,
) -> *mut named_group_8 {
    let mut hash: uint16_t = _pcre2_compile_get_hash_from_name8(name, length);
    let mut ng: *mut named_group_8 = ::core::ptr::null_mut::<named_group_8>();
    let mut end: *mut named_group_8 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    ng = (*cb).named_groups;
    while ng < end {
        if length == (*ng).length as uint32_t
            && hash as ::core::ffi::c_int
                == (*ng).hash_dup as ::core::ffi::c_int
                    & NAMED_GROUP_HASH_MASK as ::core::ffi::c_int
            && _pcre2_strncmp_8(name, (*ng).name, length as size_t) == 0 as ::core::ffi::c_int
        {
            return ng;
        }
        ng = ng.offset(1);
    }
    return ::core::ptr::null_mut::<named_group_8>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_add_name_to_table8(
    mut cb: *mut compile_block_8,
    mut ng: *mut named_group_8,
    mut tablecount: uint32_t,
) -> uint32_t {
    let mut i: uint32_t = 0;
    let mut name: PCRE2_SPTR8 = (*ng).name;
    let mut length: ::core::ffi::c_int = (*ng).length as ::core::ffi::c_int;
    let mut duplicate_count: uint32_t = 1 as uint32_t;
    let mut slot: *mut PCRE2_UCHAR8 = (*cb).name_table;
    if (*ng).hash_dup as ::core::ffi::c_int & NAMED_GROUP_IS_DUPNAME as ::core::ffi::c_int
        != 0 as ::core::ffi::c_int
    {
        let mut ng_it: *mut named_group_8 = ::core::ptr::null_mut::<named_group_8>();
        let mut end: *mut named_group_8 = (*cb)
            .named_groups
            .offset((*cb).names_found as ::core::ffi::c_int as isize);
        ng_it = ng.offset(1 as ::core::ffi::c_int as isize);
        while ng_it < end {
            if (*ng_it).name == name {
                duplicate_count = duplicate_count.wrapping_add(1);
            }
            ng_it = ng_it.offset(1);
        }
    }
    i = 0 as uint32_t;
    while i < tablecount {
        let mut crc: ::core::ffi::c_int = memcmp(
            name as *const ::core::ffi::c_void,
            slot.offset(IMM2_SIZE as isize) as *const ::core::ffi::c_void,
            (length * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        if crc == 0 as ::core::ffi::c_int
            && *slot.offset((IMM2_SIZE + length) as isize) as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            crc = -(1 as ::core::ffi::c_int);
        }
        if crc < 0 as ::core::ffi::c_int {
            memmove(
                slot.offset(
                    ((*cb).name_entry_size as uint32_t).wrapping_mul(duplicate_count) as isize,
                ) as *mut ::core::ffi::c_void,
                slot as *const ::core::ffi::c_void,
                tablecount
                    .wrapping_sub(i)
                    .wrapping_mul((*cb).name_entry_size as uint32_t)
                    .wrapping_mul((PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int) as uint32_t)
                    as size_t,
            );
            break;
        } else {
            slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
            i = i.wrapping_add(1);
        }
    }
    tablecount = (tablecount as ::core::ffi::c_uint)
        .wrapping_add(duplicate_count as ::core::ffi::c_uint) as uint32_t
        as uint32_t;
    loop {
        *slot.offset(0 as ::core::ffi::c_int as isize) =
            ((*ng).number >> 8 as ::core::ffi::c_int) as PCRE2_UCHAR8;
        *slot.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize) =
            ((*ng).number & 255 as uint32_t) as PCRE2_UCHAR8;
        memcpy(
            slot.offset(IMM2_SIZE as isize) as *mut ::core::ffi::c_void,
            name as *const ::core::ffi::c_void,
            (length * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        memset(
            slot.offset(IMM2_SIZE as isize).offset(length as isize) as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            (((*cb).name_entry_size as ::core::ffi::c_int - length - 2 as ::core::ffi::c_int)
                * (PCRE2_CODE_UNIT_WIDTH / 8 as ::core::ffi::c_int)) as size_t,
        );
        duplicate_count = duplicate_count.wrapping_sub(1);
        if duplicate_count == 0 as uint32_t {
            break;
        }
        loop {
            ng = ng.offset(1);
            if (*ng).name == name {
                break;
            }
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
    }
    return tablecount;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_find_dupname_details8(
    mut name: PCRE2_SPTR8,
    mut length: uint32_t,
    mut indexptr: *mut ::core::ffi::c_int,
    mut countptr: *mut ::core::ffi::c_int,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
) -> BOOL {
    let mut i: uint32_t = 0;
    let mut groupnumber: uint32_t = 0;
    let mut count: ::core::ffi::c_int = 0;
    let mut slot: *mut PCRE2_UCHAR8 = (*cb).name_table;
    i = 0 as uint32_t;
    while i < (*cb).names_found as uint32_t {
        if _pcre2_strncmp_8(
            name,
            slot.offset(IMM2_SIZE as isize) as PCRE2_SPTR8,
            length as size_t,
        ) == 0 as ::core::ffi::c_int
            && *slot.offset((IMM2_SIZE as uint32_t).wrapping_add(length) as isize)
                as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int
        {
            break;
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
        i = i.wrapping_add(1);
    }
    if i >= (*cb).names_found as uint32_t {
        *errorcodeptr = ERR53 as ::core::ffi::c_int;
        (*cb).erroroffset = name.offset_from((*cb).start_pattern) as ::core::ffi::c_long as size_t;
        return FALSE;
    }
    *indexptr = i as ::core::ffi::c_int;
    count = 0 as ::core::ffi::c_int;
    loop {
        count += 1;
        groupnumber = ((*slot.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int
            | *slot.offset((0 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int) as ::core::ffi::c_uint as uint32_t;
        (*cb).backref_map = ((*cb).backref_map as ::core::ffi::c_uint
            | if groupnumber < 32 as uint32_t {
                (1 as ::core::ffi::c_uint) << groupnumber
            } else {
                1 as ::core::ffi::c_uint
            }) as uint32_t;
        if groupnumber > (*cb).top_backref {
            (*cb).top_backref = groupnumber;
        }
        i = i.wrapping_add(1);
        if i >= (*cb).names_found as uint32_t {
            break;
        }
        slot = slot.offset((*cb).name_entry_size as ::core::ffi::c_int as isize);
        if _pcre2_strncmp_8(
            name,
            slot.offset(IMM2_SIZE as isize) as PCRE2_SPTR8,
            length as size_t,
        ) != 0 as ::core::ffi::c_int
            || *slot.offset(IMM2_SIZE as isize).offset(length as isize) as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            break;
        }
    }
    *countptr = count;
    return TRUE;
}
unsafe extern "C" fn _pcre2_compile_process_capture_list(
    mut pptr: *mut uint32_t,
    mut offset: size_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
) -> size_t {
    let mut i: size_t = 0;
    let mut size: size_t = 0 as size_t;
    let mut ng: *mut named_group_8 = ::core::ptr::null_mut::<named_group_8>();
    let mut name: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut length: uint32_t = 0;
    let mut end: *mut named_group_8 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    loop {
        pptr = pptr.offset(1);
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                offset = (*pptr.offset(1 as ::core::ffi::c_int as isize) as size_t)
                    << 32 as ::core::ffi::c_int
                    | *pptr.offset(2 as ::core::ffi::c_int as isize) as size_t;
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                offset = (offset as ::core::ffi::c_ulong)
                    .wrapping_add((*pptr & 0xffff as uint32_t) as ::core::ffi::c_ulong)
                    as size_t as size_t;
                pptr = pptr.offset(1);
                length = *pptr;
                name = (*cb).start_pattern.offset(offset as isize);
                ng = _pcre2_compile_find_named_group8(name, length, cb);
                if ng.is_null() {
                    *errorcodeptr = ERR15 as ::core::ffi::c_int;
                    (*cb).erroroffset = offset;
                    return 0 as size_t;
                }
                if (*ng).hash_dup as ::core::ffi::c_int
                    & NAMED_GROUP_IS_DUPNAME as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NUMBER as uint32_t;
                    *pptr.offset(0 as ::core::ffi::c_int as isize) = (*ng).number;
                    size = size.wrapping_add(1);
                } else {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NAME as uint32_t;
                    *pptr.offset(0 as ::core::ffi::c_int as isize) =
                        ng.offset_from((*cb).named_groups) as ::core::ffi::c_long as uint32_t;
                    size = size.wrapping_add(1);
                    name = (*ng).name;
                    loop {
                        ng = ng.offset(1);
                        if !(ng < end) {
                            break;
                        }
                        if (*ng).name == name {
                            size = size.wrapping_add(1);
                        }
                    }
                }
            }
            META_CAPTURE_NUMBER => {
                offset = (offset as ::core::ffi::c_ulong)
                    .wrapping_add((*pptr & 0xffff as uint32_t) as ::core::ffi::c_ulong)
                    as size_t as size_t;
                pptr = pptr.offset(1);
                i = *pptr as size_t;
                if i > (*cb).bracount as size_t {
                    *errorcodeptr = ERR15 as ::core::ffi::c_int;
                    (*cb).erroroffset = offset;
                    return 0 as size_t;
                }
                if i > (*cb).top_backref as size_t {
                    (*cb).top_backref = i as uint16_t as uint32_t;
                }
                size = size.wrapping_add(1);
            }
            _ => return size,
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_scan_substr_args8(
    mut pptr: *mut uint32_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
    mut lengthptr: *mut size_t,
) -> *mut uint32_t {
    let mut captures: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut capture_ptr: *mut uint8_t = ::core::ptr::null_mut::<uint8_t>();
    let mut bit: uint8_t = 0;
    let mut name: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut ng: *mut named_group_8 = ::core::ptr::null_mut::<named_group_8>();
    let mut end: *mut named_group_8 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    let mut all_found: BOOL = 0;
    let mut size: size_t = 0;
    if _pcre2_compile_process_capture_list(
        pptr.offset(-(1 as ::core::ffi::c_int as isize)),
        0 as size_t,
        errorcodeptr,
        cb,
    ) == 0 as size_t
    {
        return ::core::ptr::null_mut::<uint32_t>();
    }
    size = ((*cb)
        .bracount
        .wrapping_add(1 as uint32_t)
        .wrapping_add(7 as uint32_t)
        >> 3 as ::core::ffi::c_int) as size_t;
    captures = (*(*cb).cx)
        .memctl
        .malloc
        .expect("non-null function pointer")(size, (*(*cb).cx).memctl.memory_data)
        as *mut uint8_t;
    if captures.is_null() {
        *errorcodeptr = ERR21 as ::core::ffi::c_int;
        (*cb).erroroffset = (*pptr.offset(1 as ::core::ffi::c_int as isize) as size_t)
            << 32 as ::core::ffi::c_int
            | *pptr.offset(2 as ::core::ffi::c_int as isize) as size_t;
        return ::core::ptr::null_mut::<uint32_t>();
    }
    memset(
        captures as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        size,
    );
    loop {
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                pptr = pptr.offset(1);
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                ng = (*cb)
                    .named_groups
                    .offset(*pptr.offset(1 as ::core::ffi::c_int as isize) as isize);
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
                name = (*ng).name;
                all_found = TRUE as BOOL;
                loop {
                    if !((*ng).name != name) {
                        capture_ptr =
                            captures.offset(((*ng).number >> 3 as ::core::ffi::c_int) as isize);
                        bit = ((1 as ::core::ffi::c_int) << ((*ng).number & 0x7 as uint32_t))
                            as uint8_t;
                        if *capture_ptr as ::core::ffi::c_int & bit as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                        {
                            *capture_ptr = (*capture_ptr as ::core::ffi::c_int
                                | bit as ::core::ffi::c_int)
                                as uint8_t;
                            all_found = FALSE as BOOL;
                        }
                    }
                    ng = ng.offset(1);
                    if !(ng < end) {
                        break;
                    }
                }
                if all_found == 0 {
                    *lengthptr = (*lengthptr as ::core::ffi::c_ulong).wrapping_add(
                        (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * IMM2_SIZE)
                            as ::core::ffi::c_ulong,
                    ) as size_t as size_t;
                } else {
                    *pptr.offset(-(2 as ::core::ffi::c_int) as isize) =
                        META_CAPTURE_NUMBER as uint32_t;
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) = 0 as uint32_t;
                }
            }
            META_CAPTURE_NUMBER => {
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
                capture_ptr = captures.offset(
                    (*pptr.offset(-(1 as ::core::ffi::c_int) as isize) >> 3 as ::core::ffi::c_int)
                        as isize,
                );
                bit = ((1 as ::core::ffi::c_int)
                    << (*pptr.offset(-(1 as ::core::ffi::c_int) as isize) & 0x7 as uint32_t))
                    as uint8_t;
                if *capture_ptr as ::core::ffi::c_int & bit as ::core::ffi::c_int
                    != 0 as ::core::ffi::c_int
                {
                    *pptr.offset(-(1 as ::core::ffi::c_int) as isize) = 0 as uint32_t;
                } else {
                    *capture_ptr =
                        (*capture_ptr as ::core::ffi::c_int | bit as ::core::ffi::c_int) as uint8_t;
                    *lengthptr = (*lengthptr as ::core::ffi::c_ulong)
                        .wrapping_add((1 as ::core::ffi::c_int + IMM2_SIZE) as ::core::ffi::c_ulong)
                        as size_t as size_t;
                }
            }
            _ => {
                break;
            }
        }
    }
    (*(*cb).cx).memctl.free.expect("non-null function pointer")(
        captures as *mut ::core::ffi::c_void,
        (*(*cb).cx).memctl.memory_data,
    );
    return pptr.offset(-(1 as ::core::ffi::c_int as isize));
}
unsafe extern "C" fn do_heapify_u16(mut captures: *mut uint16_t, mut size: size_t, mut i: size_t) {
    let mut max: size_t = 0;
    let mut left: size_t = 0;
    let mut right: size_t = 0;
    let mut tmp: uint16_t = 0;
    loop {
        max = i;
        left = (i << 1 as ::core::ffi::c_int).wrapping_add(1 as size_t);
        right = left.wrapping_add(1 as size_t);
        if left < size
            && *captures.offset(left as isize) as ::core::ffi::c_int
                > *captures.offset(max as isize) as ::core::ffi::c_int
        {
            max = left;
        }
        if right < size
            && *captures.offset(right as isize) as ::core::ffi::c_int
                > *captures.offset(max as isize) as ::core::ffi::c_int
        {
            max = right;
        }
        if i == max {
            return;
        }
        tmp = *captures.offset(i as isize);
        *captures.offset(i as isize) = *captures.offset(max as isize);
        *captures.offset(max as isize) = tmp;
        i = max;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_compile_parse_recurse_args8(
    mut pptr_start: *mut uint32_t,
    mut offset: size_t,
    mut errorcodeptr: *mut ::core::ffi::c_int,
    mut cb: *mut compile_block_8,
) -> BOOL {
    let mut pptr: *mut uint32_t = pptr_start;
    let mut i: size_t = 0;
    let mut size: size_t = 0;
    let mut name: PCRE2_SPTR8 = ::core::ptr::null::<PCRE2_UCHAR8>();
    let mut ng: *mut named_group_8 = ::core::ptr::null_mut::<named_group_8>();
    let mut end: *mut named_group_8 = (*cb)
        .named_groups
        .offset((*cb).names_found as ::core::ffi::c_int as isize);
    let mut args: *mut recurse_arguments = ::core::ptr::null_mut::<recurse_arguments>();
    let mut captures: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut current: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut captures_end: *mut uint16_t = ::core::ptr::null_mut::<uint16_t>();
    let mut tmp: uint16_t = 0;
    size = _pcre2_compile_process_capture_list(pptr, offset, errorcodeptr, cb);
    if size == 0 as size_t {
        return FALSE;
    }
    args = (*(*cb).cx)
        .memctl
        .malloc
        .expect("non-null function pointer")(
        (::core::mem::size_of::<recurse_arguments>() as size_t)
            .wrapping_add(size.wrapping_mul(::core::mem::size_of::<uint16_t>() as size_t)),
        (*(*cb).cx).memctl.memory_data,
    ) as *mut recurse_arguments;
    if args.is_null() {
        *errorcodeptr = ERR21 as ::core::ffi::c_int;
        (*cb).erroroffset = offset;
        return FALSE;
    }
    (*args).header.next = ::core::ptr::null_mut::<compile_data>();
    (*args).size = size;
    if !(*cb).last_data.is_null() {
        (*(*cb).last_data).next = &raw mut (*args).header as *mut compile_data;
    } else {
        (*cb).first_data = &raw mut (*args).header;
    }
    (*cb).last_data = &raw mut (*args).header;
    captures = args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t;
    loop {
        pptr = pptr.offset(1);
        match *pptr & 0xffff0000 as uint32_t {
            META_OFFSET => {
                pptr = pptr.offset(2 as ::core::ffi::c_int as isize);
            }
            META_CAPTURE_NAME => {
                pptr = pptr.offset(1);
                ng = (*cb).named_groups.offset(*pptr as isize);
                let fresh6 = captures;
                captures = captures.offset(1);
                *fresh6 = (*ng).number as uint16_t;
                name = (*ng).name;
                loop {
                    ng = ng.offset(1);
                    if !(ng < end) {
                        break;
                    }
                    if (*ng).name == name {
                        let fresh7 = captures;
                        captures = captures.offset(1);
                        *fresh7 = (*ng).number as uint16_t;
                    }
                }
            }
            META_CAPTURE_NUMBER => {
                pptr = pptr.offset(1);
                let fresh8 = captures;
                captures = captures.offset(1);
                *fresh8 = *pptr as uint16_t;
            }
            _ => {
                break;
            }
        }
    }
    (*args).skip_size =
        (pptr.offset_from(pptr_start) as ::core::ffi::c_long as size_t).wrapping_sub(1 as size_t);
    if size == 1 as size_t {
        return TRUE;
    }
    captures = args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t;
    i = (size >> 1 as ::core::ffi::c_int).wrapping_sub(1 as size_t);
    loop {
        do_heapify_u16(captures, size, i);
        if i == 0 as size_t {
            break;
        }
        i = i.wrapping_sub(1);
    }
    i = size.wrapping_sub(1 as size_t);
    while i > 0 as size_t {
        tmp = *captures.offset(0 as ::core::ffi::c_int as isize);
        *captures.offset(0 as ::core::ffi::c_int as isize) = *captures.offset(i as isize);
        *captures.offset(i as isize) = tmp;
        do_heapify_u16(captures, i, 0 as size_t);
        i = i.wrapping_sub(1);
    }
    captures_end = captures.offset(size as isize);
    let fresh9 = captures;
    captures = captures.offset(1);
    tmp = *fresh9;
    current = captures;
    while current < captures_end {
        if *current as ::core::ffi::c_int != tmp as ::core::ffi::c_int {
            tmp = *current;
            let fresh10 = captures;
            captures = captures.offset(1);
            *fresh10 = tmp;
        }
        current = current.offset(1);
    }
    (*args).size = captures
        .offset_from(args.offset(1 as ::core::ffi::c_int as isize) as *mut uint16_t)
        as ::core::ffi::c_long as size_t;
    return TRUE;
}
