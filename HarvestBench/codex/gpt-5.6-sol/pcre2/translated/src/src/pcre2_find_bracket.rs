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
pub mod pcre2_internal_h {
    pub type BOOL = ::core::ffi::c_int;
    pub type C2RustUnnamed = ::core::ffi::c_uint;
    pub const OP_TABLE_LENGTH: C2RustUnnamed = 173;
    pub const OP_UCP_WORD_BOUNDARY: C2RustUnnamed = 172;
    pub const OP_NOT_UCP_WORD_BOUNDARY: C2RustUnnamed = 171;
    pub const OP_DEFINE: C2RustUnnamed = 170;
    pub const OP_SKIPZERO: C2RustUnnamed = 169;
    pub const OP_CLOSE: C2RustUnnamed = 168;
    pub const OP_ASSERT_ACCEPT: C2RustUnnamed = 167;
    pub const OP_ACCEPT: C2RustUnnamed = 166;
    pub const OP_FAIL: C2RustUnnamed = 165;
    pub const OP_COMMIT_ARG: C2RustUnnamed = 164;
    pub const OP_COMMIT: C2RustUnnamed = 163;
    pub const OP_THEN_ARG: C2RustUnnamed = 162;
    pub const OP_THEN: C2RustUnnamed = 161;
    pub const OP_SKIP_ARG: C2RustUnnamed = 160;
    pub const OP_SKIP: C2RustUnnamed = 159;
    pub const OP_PRUNE_ARG: C2RustUnnamed = 158;
    pub const OP_PRUNE: C2RustUnnamed = 157;
    pub const OP_MARK: C2RustUnnamed = 156;
    pub const OP_BRAPOSZERO: C2RustUnnamed = 155;
    pub const OP_BRAMINZERO: C2RustUnnamed = 154;
    pub const OP_BRAZERO: C2RustUnnamed = 153;
    pub const OP_TRUE: C2RustUnnamed = 152;
    pub const OP_FALSE: C2RustUnnamed = 151;
    pub const OP_DNRREF: C2RustUnnamed = 150;
    pub const OP_RREF: C2RustUnnamed = 149;
    pub const OP_DNCREF: C2RustUnnamed = 148;
    pub const OP_CREF: C2RustUnnamed = 147;
    pub const OP_SCOND: C2RustUnnamed = 146;
    pub const OP_SCBRAPOS: C2RustUnnamed = 145;
    pub const OP_SCBRA: C2RustUnnamed = 144;
    pub const OP_SBRAPOS: C2RustUnnamed = 143;
    pub const OP_SBRA: C2RustUnnamed = 142;
    pub const OP_COND: C2RustUnnamed = 141;
    pub const OP_CBRAPOS: C2RustUnnamed = 140;
    pub const OP_CBRA: C2RustUnnamed = 139;
    pub const OP_BRAPOS: C2RustUnnamed = 138;
    pub const OP_BRA: C2RustUnnamed = 137;
    pub const OP_SCRIPT_RUN: C2RustUnnamed = 136;
    pub const OP_ONCE: C2RustUnnamed = 135;
    pub const OP_ASSERT_SCS: C2RustUnnamed = 134;
    pub const OP_ASSERTBACK_NA: C2RustUnnamed = 133;
    pub const OP_ASSERT_NA: C2RustUnnamed = 132;
    pub const OP_ASSERTBACK_NOT: C2RustUnnamed = 131;
    pub const OP_ASSERTBACK: C2RustUnnamed = 130;
    pub const OP_ASSERT_NOT: C2RustUnnamed = 129;
    pub const OP_ASSERT: C2RustUnnamed = 128;
    pub const OP_VREVERSE: C2RustUnnamed = 127;
    pub const OP_REVERSE: C2RustUnnamed = 126;
    pub const OP_KETRPOS: C2RustUnnamed = 125;
    pub const OP_KETRMIN: C2RustUnnamed = 124;
    pub const OP_KETRMAX: C2RustUnnamed = 123;
    pub const OP_KET: C2RustUnnamed = 122;
    pub const OP_ALT: C2RustUnnamed = 121;
    pub const OP_CALLOUT_STR: C2RustUnnamed = 120;
    pub const OP_CALLOUT: C2RustUnnamed = 119;
    pub const OP_RECURSE: C2RustUnnamed = 118;
    pub const OP_DNREFI: C2RustUnnamed = 117;
    pub const OP_DNREF: C2RustUnnamed = 116;
    pub const OP_REFI: C2RustUnnamed = 115;
    pub const OP_REF: C2RustUnnamed = 114;
    pub const OP_ECLASS: C2RustUnnamed = 113;
    pub const OP_XCLASS: C2RustUnnamed = 112;
    pub const OP_NCLASS: C2RustUnnamed = 111;
    pub const OP_CLASS: C2RustUnnamed = 110;
    pub const OP_CRPOSRANGE: C2RustUnnamed = 109;
    pub const OP_CRPOSQUERY: C2RustUnnamed = 108;
    pub const OP_CRPOSPLUS: C2RustUnnamed = 107;
    pub const OP_CRPOSSTAR: C2RustUnnamed = 106;
    pub const OP_CRMINRANGE: C2RustUnnamed = 105;
    pub const OP_CRRANGE: C2RustUnnamed = 104;
    pub const OP_CRMINQUERY: C2RustUnnamed = 103;
    pub const OP_CRQUERY: C2RustUnnamed = 102;
    pub const OP_CRMINPLUS: C2RustUnnamed = 101;
    pub const OP_CRPLUS: C2RustUnnamed = 100;
    pub const OP_CRMINSTAR: C2RustUnnamed = 99;
    pub const OP_CRSTAR: C2RustUnnamed = 98;
    pub const OP_TYPEPOSUPTO: C2RustUnnamed = 97;
    pub const OP_TYPEPOSQUERY: C2RustUnnamed = 96;
    pub const OP_TYPEPOSPLUS: C2RustUnnamed = 95;
    pub const OP_TYPEPOSSTAR: C2RustUnnamed = 94;
    pub const OP_TYPEEXACT: C2RustUnnamed = 93;
    pub const OP_TYPEMINUPTO: C2RustUnnamed = 92;
    pub const OP_TYPEUPTO: C2RustUnnamed = 91;
    pub const OP_TYPEMINQUERY: C2RustUnnamed = 90;
    pub const OP_TYPEQUERY: C2RustUnnamed = 89;
    pub const OP_TYPEMINPLUS: C2RustUnnamed = 88;
    pub const OP_TYPEPLUS: C2RustUnnamed = 87;
    pub const OP_TYPEMINSTAR: C2RustUnnamed = 86;
    pub const OP_TYPESTAR: C2RustUnnamed = 85;
    pub const OP_NOTPOSUPTOI: C2RustUnnamed = 84;
    pub const OP_NOTPOSQUERYI: C2RustUnnamed = 83;
    pub const OP_NOTPOSPLUSI: C2RustUnnamed = 82;
    pub const OP_NOTPOSSTARI: C2RustUnnamed = 81;
    pub const OP_NOTEXACTI: C2RustUnnamed = 80;
    pub const OP_NOTMINUPTOI: C2RustUnnamed = 79;
    pub const OP_NOTUPTOI: C2RustUnnamed = 78;
    pub const OP_NOTMINQUERYI: C2RustUnnamed = 77;
    pub const OP_NOTQUERYI: C2RustUnnamed = 76;
    pub const OP_NOTMINPLUSI: C2RustUnnamed = 75;
    pub const OP_NOTPLUSI: C2RustUnnamed = 74;
    pub const OP_NOTMINSTARI: C2RustUnnamed = 73;
    pub const OP_NOTSTARI: C2RustUnnamed = 72;
    pub const OP_NOTPOSUPTO: C2RustUnnamed = 71;
    pub const OP_NOTPOSQUERY: C2RustUnnamed = 70;
    pub const OP_NOTPOSPLUS: C2RustUnnamed = 69;
    pub const OP_NOTPOSSTAR: C2RustUnnamed = 68;
    pub const OP_NOTEXACT: C2RustUnnamed = 67;
    pub const OP_NOTMINUPTO: C2RustUnnamed = 66;
    pub const OP_NOTUPTO: C2RustUnnamed = 65;
    pub const OP_NOTMINQUERY: C2RustUnnamed = 64;
    pub const OP_NOTQUERY: C2RustUnnamed = 63;
    pub const OP_NOTMINPLUS: C2RustUnnamed = 62;
    pub const OP_NOTPLUS: C2RustUnnamed = 61;
    pub const OP_NOTMINSTAR: C2RustUnnamed = 60;
    pub const OP_NOTSTAR: C2RustUnnamed = 59;
    pub const OP_POSUPTOI: C2RustUnnamed = 58;
    pub const OP_POSQUERYI: C2RustUnnamed = 57;
    pub const OP_POSPLUSI: C2RustUnnamed = 56;
    pub const OP_POSSTARI: C2RustUnnamed = 55;
    pub const OP_EXACTI: C2RustUnnamed = 54;
    pub const OP_MINUPTOI: C2RustUnnamed = 53;
    pub const OP_UPTOI: C2RustUnnamed = 52;
    pub const OP_MINQUERYI: C2RustUnnamed = 51;
    pub const OP_QUERYI: C2RustUnnamed = 50;
    pub const OP_MINPLUSI: C2RustUnnamed = 49;
    pub const OP_PLUSI: C2RustUnnamed = 48;
    pub const OP_MINSTARI: C2RustUnnamed = 47;
    pub const OP_STARI: C2RustUnnamed = 46;
    pub const OP_POSUPTO: C2RustUnnamed = 45;
    pub const OP_POSQUERY: C2RustUnnamed = 44;
    pub const OP_POSPLUS: C2RustUnnamed = 43;
    pub const OP_POSSTAR: C2RustUnnamed = 42;
    pub const OP_EXACT: C2RustUnnamed = 41;
    pub const OP_MINUPTO: C2RustUnnamed = 40;
    pub const OP_UPTO: C2RustUnnamed = 39;
    pub const OP_MINQUERY: C2RustUnnamed = 38;
    pub const OP_QUERY: C2RustUnnamed = 37;
    pub const OP_MINPLUS: C2RustUnnamed = 36;
    pub const OP_PLUS: C2RustUnnamed = 35;
    pub const OP_MINSTAR: C2RustUnnamed = 34;
    pub const OP_STAR: C2RustUnnamed = 33;
    pub const OP_NOTI: C2RustUnnamed = 32;
    pub const OP_NOT: C2RustUnnamed = 31;
    pub const OP_CHARI: C2RustUnnamed = 30;
    pub const OP_CHAR: C2RustUnnamed = 29;
    pub const OP_CIRCM: C2RustUnnamed = 28;
    pub const OP_CIRC: C2RustUnnamed = 27;
    pub const OP_DOLLM: C2RustUnnamed = 26;
    pub const OP_DOLL: C2RustUnnamed = 25;
    pub const OP_EOD: C2RustUnnamed = 24;
    pub const OP_EODN: C2RustUnnamed = 23;
    pub const OP_EXTUNI: C2RustUnnamed = 22;
    pub const OP_VSPACE: C2RustUnnamed = 21;
    pub const OP_NOT_VSPACE: C2RustUnnamed = 20;
    pub const OP_HSPACE: C2RustUnnamed = 19;
    pub const OP_NOT_HSPACE: C2RustUnnamed = 18;
    pub const OP_ANYNL: C2RustUnnamed = 17;
    pub const OP_PROP: C2RustUnnamed = 16;
    pub const OP_NOTPROP: C2RustUnnamed = 15;
    pub const OP_ANYBYTE: C2RustUnnamed = 14;
    pub const OP_ALLANY: C2RustUnnamed = 13;
    pub const OP_ANY: C2RustUnnamed = 12;
    pub const OP_WORDCHAR: C2RustUnnamed = 11;
    pub const OP_NOT_WORDCHAR: C2RustUnnamed = 10;
    pub const OP_WHITESPACE: C2RustUnnamed = 9;
    pub const OP_NOT_WHITESPACE: C2RustUnnamed = 8;
    pub const OP_DIGIT: C2RustUnnamed = 7;
    pub const OP_NOT_DIGIT: C2RustUnnamed = 6;
    pub const OP_WORD_BOUNDARY: C2RustUnnamed = 5;
    pub const OP_NOT_WORD_BOUNDARY: C2RustUnnamed = 4;
    pub const OP_SET_SOM: C2RustUnnamed = 3;
    pub const OP_SOM: C2RustUnnamed = 2;
    pub const OP_SOD: C2RustUnnamed = 1;
    pub const OP_END: C2RustUnnamed = 0;
    use super::stdint_uintn_h::uint8_t;
    extern "C" {
        pub static _pcre2_utf8_table4: [uint8_t; 0];
        pub static _pcre2_OP_lengths_8: [uint8_t; 0];
    }
}
pub mod stdint_uintn_h {
    pub type uint8_t = __uint8_t;
    use super::types_h::__uint8_t;
}
pub mod pcre2_h {
    pub type PCRE2_UCHAR8 = uint8_t;
    pub type PCRE2_SPTR8 = *const PCRE2_UCHAR8;
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
pub mod pcre2_intmodedep_h {
    pub const IMM2_SIZE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
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
    _pcre2_OP_lengths_8, _pcre2_utf8_table4, C2RustUnnamed, BOOL, OP_ACCEPT, OP_ALLANY, OP_ALT,
    OP_ANY, OP_ANYBYTE, OP_ANYNL, OP_ASSERT, OP_ASSERTBACK, OP_ASSERTBACK_NA, OP_ASSERTBACK_NOT,
    OP_ASSERT_ACCEPT, OP_ASSERT_NA, OP_ASSERT_NOT, OP_ASSERT_SCS, OP_BRA, OP_BRAMINZERO, OP_BRAPOS,
    OP_BRAPOSZERO, OP_BRAZERO, OP_CALLOUT, OP_CALLOUT_STR, OP_CBRA, OP_CBRAPOS, OP_CHAR, OP_CHARI,
    OP_CIRC, OP_CIRCM, OP_CLASS, OP_CLOSE, OP_COMMIT, OP_COMMIT_ARG, OP_COND, OP_CREF,
    OP_CRMINPLUS, OP_CRMINQUERY, OP_CRMINRANGE, OP_CRMINSTAR, OP_CRPLUS, OP_CRPOSPLUS,
    OP_CRPOSQUERY, OP_CRPOSRANGE, OP_CRPOSSTAR, OP_CRQUERY, OP_CRRANGE, OP_CRSTAR, OP_DEFINE,
    OP_DIGIT, OP_DNCREF, OP_DNREF, OP_DNREFI, OP_DNRREF, OP_DOLL, OP_DOLLM, OP_ECLASS, OP_END,
    OP_EOD, OP_EODN, OP_EXACT, OP_EXACTI, OP_EXTUNI, OP_FAIL, OP_FALSE, OP_HSPACE, OP_KET,
    OP_KETRMAX, OP_KETRMIN, OP_KETRPOS, OP_MARK, OP_MINPLUS, OP_MINPLUSI, OP_MINQUERY,
    OP_MINQUERYI, OP_MINSTAR, OP_MINSTARI, OP_MINUPTO, OP_MINUPTOI, OP_NCLASS, OP_NOT, OP_NOTEXACT,
    OP_NOTEXACTI, OP_NOTI, OP_NOTMINPLUS, OP_NOTMINPLUSI, OP_NOTMINQUERY, OP_NOTMINQUERYI,
    OP_NOTMINSTAR, OP_NOTMINSTARI, OP_NOTMINUPTO, OP_NOTMINUPTOI, OP_NOTPLUS, OP_NOTPLUSI,
    OP_NOTPOSPLUS, OP_NOTPOSPLUSI, OP_NOTPOSQUERY, OP_NOTPOSQUERYI, OP_NOTPOSSTAR, OP_NOTPOSSTARI,
    OP_NOTPOSUPTO, OP_NOTPOSUPTOI, OP_NOTPROP, OP_NOTQUERY, OP_NOTQUERYI, OP_NOTSTAR, OP_NOTSTARI,
    OP_NOTUPTO, OP_NOTUPTOI, OP_NOT_DIGIT, OP_NOT_HSPACE, OP_NOT_UCP_WORD_BOUNDARY, OP_NOT_VSPACE,
    OP_NOT_WHITESPACE, OP_NOT_WORDCHAR, OP_NOT_WORD_BOUNDARY, OP_ONCE, OP_PLUS, OP_PLUSI,
    OP_POSPLUS, OP_POSPLUSI, OP_POSQUERY, OP_POSQUERYI, OP_POSSTAR, OP_POSSTARI, OP_POSUPTO,
    OP_POSUPTOI, OP_PROP, OP_PRUNE, OP_PRUNE_ARG, OP_QUERY, OP_QUERYI, OP_RECURSE, OP_REF, OP_REFI,
    OP_REVERSE, OP_RREF, OP_SBRA, OP_SBRAPOS, OP_SCBRA, OP_SCBRAPOS, OP_SCOND, OP_SCRIPT_RUN,
    OP_SET_SOM, OP_SKIP, OP_SKIPZERO, OP_SKIP_ARG, OP_SOD, OP_SOM, OP_STAR, OP_STARI,
    OP_TABLE_LENGTH, OP_THEN, OP_THEN_ARG, OP_TRUE, OP_TYPEEXACT, OP_TYPEMINPLUS, OP_TYPEMINQUERY,
    OP_TYPEMINSTAR, OP_TYPEMINUPTO, OP_TYPEPLUS, OP_TYPEPOSPLUS, OP_TYPEPOSQUERY, OP_TYPEPOSSTAR,
    OP_TYPEPOSUPTO, OP_TYPEQUERY, OP_TYPESTAR, OP_TYPEUPTO, OP_UCP_WORD_BOUNDARY, OP_UPTO,
    OP_UPTOI, OP_VREVERSE, OP_VSPACE, OP_WHITESPACE, OP_WORDCHAR, OP_WORD_BOUNDARY, OP_XCLASS,
};
pub use self::pcre2_intmodedep_h::IMM2_SIZE;
pub use self::stddef_h::{size_t, NULL, NULL_0};
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
pub unsafe extern "C" fn _pcre2_find_bracket_8(
    mut code: PCRE2_SPTR8,
    mut utf: BOOL,
    mut number: ::core::ffi::c_int,
) -> PCRE2_SPTR8 {
    loop {
        let mut c: PCRE2_UCHAR8 = *code;
        if c as ::core::ffi::c_int == OP_END as ::core::ffi::c_int {
            return ::core::ptr::null::<PCRE2_UCHAR8>();
        }
        if c as ::core::ffi::c_int == OP_XCLASS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_ECLASS as ::core::ffi::c_int
        {
            code = code.offset(
                ((*code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *code.offset((1 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as isize)
                        as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
        } else if c as ::core::ffi::c_int == OP_CALLOUT_STR as ::core::ffi::c_int {
            code = code.offset(
                ((*code.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                    << 8 as ::core::ffi::c_int
                    | *code.offset(
                        (1 as ::core::ffi::c_int
                            + 2 as ::core::ffi::c_int * 2 as ::core::ffi::c_int
                            + 1 as ::core::ffi::c_int) as isize,
                    ) as ::core::ffi::c_int) as ::core::ffi::c_uint as isize,
            );
        } else if c as ::core::ffi::c_int == OP_REVERSE as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_VREVERSE as ::core::ffi::c_int
        {
            if number < 0 as ::core::ffi::c_int {
                return code;
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
        } else if c as ::core::ffi::c_int == OP_CBRA as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_SCBRA as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_CBRAPOS as ::core::ffi::c_int
            || c as ::core::ffi::c_int == OP_SCBRAPOS as ::core::ffi::c_int
        {
            let mut n: ::core::ffi::c_int = ((*code
                .offset((1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as isize)
                as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int
                | *code.offset(
                    (1 as ::core::ffi::c_int + 2 as ::core::ffi::c_int + 1 as ::core::ffi::c_int)
                        as isize,
                ) as ::core::ffi::c_int)
                as ::core::ffi::c_uint
                as ::core::ffi::c_int;
            if n == number {
                return code;
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
        } else {
            match c as ::core::ffi::c_int {
                85 | 86 | 87 | 88 | 89 | 90 | 94 | 95 | 96 => {
                    if *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        == OP_PROP as ::core::ffi::c_int
                        || *code.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int
                    {
                        code = code.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
                91 | 92 | 93 | 97 => {
                    if *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                        as ::core::ffi::c_int
                        == OP_PROP as ::core::ffi::c_int
                        || *code.offset((1 as ::core::ffi::c_int + IMM2_SIZE) as isize)
                            as ::core::ffi::c_int
                            == OP_NOTPROP as ::core::ffi::c_int
                    {
                        code = code.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
                156 | 164 | 158 | 160 | 162 => {
                    code = code.offset(*code.offset(1 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int as isize);
                }
                _ => {}
            }
            code = code.offset(
                *(&raw const _pcre2_OP_lengths_8 as *const uint8_t).offset(c as isize)
                    as ::core::ffi::c_int as isize,
            );
            if utf != 0 {
                match c as ::core::ffi::c_int {
                    29 | 30 | 31 | 32 | 41 | 54 | 67 | 80 | 39 | 52 | 65 | 78 | 40 | 53 | 66
                    | 79 | 45 | 58 | 71 | 84 | 33 | 46 | 59 | 72 | 34 | 47 | 60 | 73 | 42 | 55
                    | 68 | 81 | 35 | 48 | 61 | 74 | 36 | 49 | 62 | 75 | 43 | 56 | 69 | 82 | 37
                    | 50 | 63 | 76 | 38 | 51 | 64 | 77 | 44 | 57 | 70 | 83 => {
                        if *code.offset(-(1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_int
                            >= 0xc0 as ::core::ffi::c_int
                        {
                            code = code.offset(
                                *(&raw const _pcre2_utf8_table4 as *const uint8_t).offset(
                                    (*code.offset(-(1 as ::core::ffi::c_int) as isize)
                                        as ::core::ffi::c_uint
                                        & 0x3f as ::core::ffi::c_uint)
                                        as isize,
                                ) as ::core::ffi::c_int as isize,
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
