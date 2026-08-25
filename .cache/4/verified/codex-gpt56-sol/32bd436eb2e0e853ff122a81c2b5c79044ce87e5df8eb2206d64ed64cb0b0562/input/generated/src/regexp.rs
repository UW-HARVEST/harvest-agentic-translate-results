extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn strtoll(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    fn realloc(
        __ptr: *mut ::core::ffi::c_void,
        __size: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: ::core::ffi::VaList,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn __uflow(_: *mut FILE) -> ::core::ffi::c_int;
    fn __overflow(_: *mut FILE, _: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strchr(
        __s: *const ::core::ffi::c_char,
        __c: ::core::ffi::c_int,
    ) -> *mut ::core::ffi::c_char;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn longjmp(__env: *mut __jmp_buf_tag, __val: ::core::ffi::c_int) -> !;
    fn jsU_chartorune(
        rune: *mut Rune,
        str: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn jsU_isalpharune(c: Rune) -> ::core::ffi::c_int;
    fn jsU_toupperrune(c: Rune) -> Rune;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: ::core::ffi::c_uint,
    pub fp_offset: ::core::ffi::c_uint,
    pub overflow_arg_area: *mut ::core::ffi::c_void,
    pub reg_save_area: *mut ::core::ffi::c_void,
}
pub type size_t = usize;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sigset_t {
    pub __val: [::core::ffi::c_ulong; 16],
}
pub type __compar_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
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
pub type FILE = _IO_FILE;
pub type __jmp_buf = [::core::ffi::c_long; 8];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __jmp_buf_tag {
    pub __jmpbuf: __jmp_buf,
    pub __mask_was_saved: ::core::ffi::c_int,
    pub __saved_mask: __sigset_t,
}
pub type jmp_buf = [__jmp_buf_tag; 1];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Reprog {
    pub start: *mut Reinst,
    pub end: *mut Reinst,
    pub cclass: *mut Reclass,
    pub flags: ::core::ffi::c_int,
    pub nsub: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Reclass {
    pub end: *mut Rune,
    pub spans: [Rune; 64],
}
pub type Rune = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Reinst {
    pub opcode: ::core::ffi::c_uchar,
    pub n: ::core::ffi::c_uchar,
    pub c: Rune,
    pub cc: *mut Reclass,
    pub x: *mut Reinst,
    pub y: *mut Reinst,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Resub {
    pub nsub: ::core::ffi::c_int,
    pub sub: [C2RustUnnamed; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub sp: *const ::core::ffi::c_char,
    pub ep: *const ::core::ffi::c_char,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cstate {
    pub prog: *mut Reprog,
    pub pstart: *mut Renode,
    pub pend: *mut Renode,
    pub source: *const ::core::ffi::c_char,
    pub ncclass: ::core::ffi::c_int,
    pub nsub: ::core::ffi::c_int,
    pub sub: [*mut Renode; 16],
    pub lookahead: ::core::ffi::c_int,
    pub yychar: Rune,
    pub yycc: *mut Reclass,
    pub yymin: ::core::ffi::c_int,
    pub yymax: ::core::ffi::c_int,
    pub error: *const ::core::ffi::c_char,
    pub kaboom: jmp_buf,
    pub cclass: [Reclass; 128],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Renode {
    pub type_0: ::core::ffi::c_uchar,
    pub ng: ::core::ffi::c_uchar,
    pub m: ::core::ffi::c_uchar,
    pub n: ::core::ffi::c_uchar,
    pub c: Rune,
    pub cc: ::core::ffi::c_int,
    pub x: *mut Renode,
    pub y: *mut Renode,
}
pub const I_END: C2RustUnnamed_3 = 0;
pub const I_RPAR: C2RustUnnamed_3 = 16;
pub const I_REF: C2RustUnnamed_3 = 10;
pub const P_REF: C2RustUnnamed_2 = 14;
pub const I_NCCLASS: C2RustUnnamed_3 = 9;
pub const P_NCCLASS: C2RustUnnamed_2 = 13;
pub const I_CCLASS: C2RustUnnamed_3 = 8;
pub const P_CCLASS: C2RustUnnamed_2 = 12;
pub const REG_ICASE: C2RustUnnamed_0 = 1;
pub const I_CHAR: C2RustUnnamed_3 = 7;
pub const P_CHAR: C2RustUnnamed_2 = 11;
pub const I_ANY: C2RustUnnamed_3 = 6;
pub const P_ANY: C2RustUnnamed_2 = 10;
pub const I_NLA: C2RustUnnamed_3 = 4;
pub const P_NLA: C2RustUnnamed_2 = 9;
pub const I_PLA: C2RustUnnamed_3 = 3;
pub const P_PLA: C2RustUnnamed_2 = 8;
pub const I_LPAR: C2RustUnnamed_3 = 15;
pub const P_PAR: C2RustUnnamed_2 = 7;
pub const I_NWORD: C2RustUnnamed_3 = 14;
pub const P_NWORD: C2RustUnnamed_2 = 6;
pub const I_WORD: C2RustUnnamed_3 = 13;
pub const P_WORD: C2RustUnnamed_2 = 5;
pub const I_EOL: C2RustUnnamed_3 = 12;
pub const P_EOL: C2RustUnnamed_2 = 4;
pub const I_BOL: C2RustUnnamed_3 = 11;
pub const P_BOL: C2RustUnnamed_2 = 3;
pub const I_SPLIT: C2RustUnnamed_3 = 2;
pub const I_JUMP: C2RustUnnamed_3 = 1;
pub const P_REP: C2RustUnnamed_2 = 2;
pub const P_ALT: C2RustUnnamed_2 = 1;
pub const P_CAT: C2RustUnnamed_2 = 0;
pub const I_ANYNL: C2RustUnnamed_3 = 5;
pub const L_CHAR: C2RustUnnamed_1 = 256;
pub const L_NLA: C2RustUnnamed_1 = 261;
pub const L_PLA: C2RustUnnamed_1 = 260;
pub const L_NC: C2RustUnnamed_1 = 259;
pub const L_CCLASS: C2RustUnnamed_1 = 257;
pub const L_NCCLASS: C2RustUnnamed_1 = 258;
pub const L_COUNT: C2RustUnnamed_1 = 265;
pub const L_REF: C2RustUnnamed_1 = 264;
pub const L_NWORD: C2RustUnnamed_1 = 263;
pub const L_WORD: C2RustUnnamed_1 = 262;
pub const REG_NEWLINE: C2RustUnnamed_0 = 2;
pub const REG_NOTBOL: C2RustUnnamed_0 = 4;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub type C2RustUnnamed_2 = ::core::ffi::c_uint;
pub type C2RustUnnamed_3 = ::core::ffi::c_uint;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
#[inline]
unsafe extern "C" fn atoi(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn atol(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_long {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn atoll(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_longlong {
    return strtoll(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn __bswap_16(mut __bsx: __uint16_t) -> __uint16_t {
    return (__bsx as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
        & 0xff as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            << 8 as ::core::ffi::c_int) as __uint16_t;
}
#[inline]
unsafe extern "C" fn __bswap_32(mut __bsx: __uint32_t) -> __uint32_t {
    return (__bsx & 0xff000000 as __uint32_t) >> 24 as ::core::ffi::c_int
        | (__bsx & 0xff0000 as __uint32_t) >> 8 as ::core::ffi::c_int
        | (__bsx & 0xff00 as __uint32_t) << 8 as ::core::ffi::c_int
        | (__bsx & 0xff as __uint32_t) << 24 as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn __bswap_64(mut __bsx: __uint64_t) -> __uint64_t {
    return ((__bsx as ::core::ffi::c_ulonglong
        & 0xff00000000000000 as ::core::ffi::c_ulonglong) >> 56 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong
            & 0xff000000000000 as ::core::ffi::c_ulonglong) >> 40 as ::core::ffi::c_int
        | (__bsx as ::core::ffi::c_ulonglong
            & 0xff0000000000 as ::core::ffi::c_ulonglong) >> 24 as ::core::ffi::c_int
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
#[inline]
unsafe extern "C" fn __uint16_identity(mut __x: __uint16_t) -> __uint16_t {
    return __x;
}
#[inline]
unsafe extern "C" fn __uint32_identity(mut __x: __uint32_t) -> __uint32_t {
    return __x;
}
#[inline]
unsafe extern "C" fn __uint64_identity(mut __x: __uint64_t) -> __uint64_t {
    return __x;
}
#[inline]
unsafe extern "C" fn bsearch(
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
        __p = (__base as *const ::core::ffi::c_char)
            .offset(__idx.wrapping_mul(__size) as isize) as *const ::core::ffi::c_void;
        __comparison = Some(__compar.expect("non-null function pointer"))
            .expect("non-null function pointer")(__key, __p);
        if __comparison < 0 as ::core::ffi::c_int {
            __u = __idx;
        } else if __comparison > 0 as ::core::ffi::c_int {
            __l = __idx.wrapping_add(1 as size_t);
        } else {
            return __p as *mut ::core::ffi::c_void
        }
    }
    return NULL;
}
#[inline]
unsafe extern "C" fn atof(
    mut __nptr: *const ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    return strtod(__nptr, NULL as *mut *mut ::core::ffi::c_char);
}
pub const _IO_EOF_SEEN: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const _IO_ERR_SEEN: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const EOF: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
#[inline]
unsafe extern "C" fn vprintf(
    mut __fmt: *const ::core::ffi::c_char,
    mut __arg: ::core::ffi::VaList,
) -> ::core::ffi::c_int {
    return vfprintf(stdout, __fmt, __arg.as_va_list());
}
#[inline]
unsafe extern "C" fn getchar() -> ::core::ffi::c_int {
    return getc(stdin);
}
#[inline]
unsafe extern "C" fn fgetc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh2 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh2 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getc_unlocked(mut __fp: *mut FILE) -> ::core::ffi::c_int {
    return if ((*__fp)._IO_read_ptr >= (*__fp)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(__fp)
    } else {
        let fresh0 = (*__fp)._IO_read_ptr;
        (*__fp)._IO_read_ptr = (*__fp)._IO_read_ptr.offset(1);
        *(fresh0 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn getchar_unlocked() -> ::core::ffi::c_int {
    return if ((*stdin)._IO_read_ptr >= (*stdin)._IO_read_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
    {
        __uflow(stdin)
    } else {
        let fresh1 = (*stdin)._IO_read_ptr;
        (*stdin)._IO_read_ptr = (*stdin)._IO_read_ptr.offset(1);
        *(fresh1 as *mut ::core::ffi::c_uchar) as ::core::ffi::c_int
    };
}
#[inline]
unsafe extern "C" fn putchar(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return putc(__c, stdout);
}
#[inline]
unsafe extern "C" fn fputc_unlocked(
    mut __c: ::core::ffi::c_int,
    mut __stream: *mut FILE,
) -> ::core::ffi::c_int {
    return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end)
        as ::core::ffi::c_int as ::core::ffi::c_long != 0
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
unsafe extern "C" fn putc_unlocked(
    mut __c: ::core::ffi::c_int,
    mut __stream: *mut FILE,
) -> ::core::ffi::c_int {
    return if ((*__stream)._IO_write_ptr >= (*__stream)._IO_write_end)
        as ::core::ffi::c_int as ::core::ffi::c_long != 0
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
unsafe extern "C" fn putchar_unlocked(
    mut __c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return if ((*stdout)._IO_write_ptr >= (*stdout)._IO_write_end) as ::core::ffi::c_int
        as ::core::ffi::c_long != 0
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
unsafe extern "C" fn feof_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
    return ((*__stream)._flags & _IO_EOF_SEEN != 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn ferror_unlocked(mut __stream: *mut FILE) -> ::core::ffi::c_int {
    return ((*__stream)._flags & _IO_ERR_SEEN != 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
pub const REG_MAXSUB: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const REPINF: ::core::ffi::c_int = 255 as ::core::ffi::c_int;
pub const REG_MAXPROG: ::core::ffi::c_int = (32 as ::core::ffi::c_int)
    << 10 as ::core::ffi::c_int;
pub const REG_MAXREC: ::core::ffi::c_int = 4096 as ::core::ffi::c_int;
pub const REG_MAXCLASS: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
unsafe extern "C" fn die(mut g: *mut cstate, mut message: *const ::core::ffi::c_char) {
    (*g).error = message;
    longjmp(&raw mut (*g).kaboom as *mut __jmp_buf_tag, 1 as ::core::ffi::c_int);
}
unsafe extern "C" fn canon(mut c: Rune) -> ::core::ffi::c_int {
    let mut u: Rune = jsU_toupperrune(c);
    if c >= 128 as ::core::ffi::c_int && u < 128 as ::core::ffi::c_int {
        return c as ::core::ffi::c_int;
    }
    return u as ::core::ffi::c_int;
}
unsafe extern "C" fn hex(
    mut g: *mut cstate,
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if c >= '0' as i32 && c <= '9' as i32 {
        return c - '0' as i32;
    }
    if c >= 'a' as i32 && c <= 'f' as i32 {
        return c - 'a' as i32 + 0xa as ::core::ffi::c_int;
    }
    if c >= 'A' as i32 && c <= 'F' as i32 {
        return c - 'A' as i32 + 0xa as ::core::ffi::c_int;
    }
    die(g, b"invalid escape sequence\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn dec(
    mut g: *mut cstate,
    mut c: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if c >= '0' as i32 && c <= '9' as i32 {
        return c - '0' as i32;
    }
    die(g, b"invalid quantifier\0" as *const u8 as *const ::core::ffi::c_char);
    return 0 as ::core::ffi::c_int;
}
pub const ESCAPES: [::core::ffi::c_char; 34] = unsafe {
    ::core::mem::transmute::<
        [u8; 34],
        [::core::ffi::c_char; 34],
    >(*b"BbDdSsWw^$\\.*+?()[]{}|-0123456789\0")
};
unsafe extern "C" fn isunicodeletter(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c >= 'a' as i32 && c <= 'z' as i32 || c >= 'A' as i32 && c <= 'Z' as i32
        || jsU_isalpharune(c as Rune) != 0) as ::core::ffi::c_int;
}
unsafe extern "C" fn nextrune(mut g: *mut cstate) -> ::core::ffi::c_int {
    if *(*g).source == 0 {
        (*g).yychar = EOF as Rune;
        return 0 as ::core::ffi::c_int;
    }
    (*g).source = (*g)
        .source
        .offset(jsU_chartorune(&raw mut (*g).yychar, (*g).source) as isize);
    if (*g).yychar == '\\' as i32 {
        if *(*g).source == 0 {
            die(
                g,
                b"unterminated escape sequence\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        (*g).source = (*g)
            .source
            .offset(jsU_chartorune(&raw mut (*g).yychar, (*g).source) as isize);
        match (*g).yychar {
            102 => {
                (*g).yychar = '\u{c}' as i32 as Rune;
                return 0 as ::core::ffi::c_int;
            }
            110 => {
                (*g).yychar = '\n' as i32 as Rune;
                return 0 as ::core::ffi::c_int;
            }
            114 => {
                (*g).yychar = '\r' as i32 as Rune;
                return 0 as ::core::ffi::c_int;
            }
            116 => {
                (*g).yychar = '\t' as i32 as Rune;
                return 0 as ::core::ffi::c_int;
            }
            118 => {
                (*g).yychar = '\u{b}' as i32 as Rune;
                return 0 as ::core::ffi::c_int;
            }
            99 => {
                if *(*g).source.offset(0 as ::core::ffi::c_int as isize) == 0 {
                    die(
                        g,
                        b"unterminated escape sequence\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
                let fresh11 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = (*fresh11 as ::core::ffi::c_int & 31 as ::core::ffi::c_int)
                    as Rune;
                return 0 as ::core::ffi::c_int;
            }
            120 => {
                if *(*g).source.offset(0 as ::core::ffi::c_int as isize) == 0
                    || *(*g).source.offset(1 as ::core::ffi::c_int as isize) == 0
                {
                    die(
                        g,
                        b"unterminated escape sequence\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
                let fresh12 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = (hex(g, *fresh12 as ::core::ffi::c_int)
                    << 4 as ::core::ffi::c_int) as Rune;
                let fresh13 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar += hex(g, *fresh13 as ::core::ffi::c_int);
                if (*g).yychar == 0 as ::core::ffi::c_int {
                    (*g).yychar = '0' as i32 as Rune;
                    return 1 as ::core::ffi::c_int;
                }
                return 1 as ::core::ffi::c_int;
            }
            117 => {
                if *(*g).source.offset(0 as ::core::ffi::c_int as isize) == 0
                    || *(*g).source.offset(1 as ::core::ffi::c_int as isize) == 0
                    || *(*g).source.offset(2 as ::core::ffi::c_int as isize) == 0
                    || *(*g).source.offset(3 as ::core::ffi::c_int as isize) == 0
                {
                    die(
                        g,
                        b"unterminated escape sequence\0" as *const u8
                            as *const ::core::ffi::c_char,
                    );
                }
                let fresh14 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = (hex(g, *fresh14 as ::core::ffi::c_int)
                    << 12 as ::core::ffi::c_int) as Rune;
                let fresh15 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar
                    += hex(g, *fresh15 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int;
                let fresh16 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar
                    += hex(g, *fresh16 as ::core::ffi::c_int) << 4 as ::core::ffi::c_int;
                let fresh17 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar += hex(g, *fresh17 as ::core::ffi::c_int);
                if (*g).yychar == 0 as ::core::ffi::c_int {
                    (*g).yychar = '0' as i32 as Rune;
                    return 1 as ::core::ffi::c_int;
                }
                return 1 as ::core::ffi::c_int;
            }
            0 => {
                (*g).yychar = '0' as i32 as Rune;
                return 1 as ::core::ffi::c_int;
            }
            _ => {}
        }
        if !strchr(ESCAPES.as_ptr(), (*g).yychar as ::core::ffi::c_int).is_null() {
            return 1 as ::core::ffi::c_int;
        }
        if isunicodeletter((*g).yychar as ::core::ffi::c_int) != 0
            || (*g).yychar == '_' as i32
        {
            die(
                g,
                b"invalid escape character\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        return 0 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn lexcount(mut g: *mut cstate) -> ::core::ffi::c_int {
    let fresh19 = (*g).source;
    (*g).source = (*g).source.offset(1);
    (*g).yychar = *fresh19 as Rune;
    (*g).yymin = dec(g, (*g).yychar as ::core::ffi::c_int);
    let fresh20 = (*g).source;
    (*g).source = (*g).source.offset(1);
    (*g).yychar = *fresh20 as Rune;
    while (*g).yychar != ',' as i32 && (*g).yychar != '}' as i32 {
        (*g).yymin = (*g).yymin * 10 as ::core::ffi::c_int
            + dec(g, (*g).yychar as ::core::ffi::c_int);
        let fresh21 = (*g).source;
        (*g).source = (*g).source.offset(1);
        (*g).yychar = *fresh21 as Rune;
        if (*g).yymin >= REPINF {
            die(g, b"numeric overflow\0" as *const u8 as *const ::core::ffi::c_char);
        }
    }
    if (*g).yychar == ',' as i32 {
        let fresh22 = (*g).source;
        (*g).source = (*g).source.offset(1);
        (*g).yychar = *fresh22 as Rune;
        if (*g).yychar == '}' as i32 {
            (*g).yymax = REPINF;
        } else {
            (*g).yymax = dec(g, (*g).yychar as ::core::ffi::c_int);
            let fresh23 = (*g).source;
            (*g).source = (*g).source.offset(1);
            (*g).yychar = *fresh23 as Rune;
            while (*g).yychar != '}' as i32 {
                (*g).yymax = (*g).yymax * 10 as ::core::ffi::c_int
                    + dec(g, (*g).yychar as ::core::ffi::c_int);
                let fresh24 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = *fresh24 as Rune;
                if (*g).yymax >= REPINF {
                    die(
                        g,
                        b"numeric overflow\0" as *const u8 as *const ::core::ffi::c_char,
                    );
                }
            }
        }
    } else {
        (*g).yymax = (*g).yymin;
    }
    return L_COUNT as ::core::ffi::c_int;
}
unsafe extern "C" fn newcclass(mut g: *mut cstate) {
    if (*g).ncclass >= REG_MAXCLASS {
        die(
            g,
            b"too many character classes\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    let fresh18 = (*g).ncclass;
    (*g).ncclass = (*g).ncclass + 1;
    (*g).yycc = (&raw mut (*g).cclass as *mut Reclass).offset(fresh18 as isize);
    (*(*g).yycc).end = &raw mut (*(*g).yycc).spans as *mut Rune;
}
unsafe extern "C" fn addrange(mut g: *mut cstate, mut a: Rune, mut b: Rune) {
    let mut cc: *mut Reclass = (*g).yycc;
    let mut p: *mut Rune = ::core::ptr::null_mut::<Rune>();
    if a > b {
        die(
            g,
            b"invalid character class range\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    p = &raw mut (*cc).spans as *mut Rune;
    while p < (*cc).end {
        if a >= *p.offset(0 as ::core::ffi::c_int as isize)
            && b <= *p.offset(1 as ::core::ffi::c_int as isize)
        {
            return;
        }
        if a < *p.offset(0 as ::core::ffi::c_int as isize)
            && b >= *p.offset(1 as ::core::ffi::c_int as isize)
        {
            *p.offset(0 as ::core::ffi::c_int as isize) = a;
            *p.offset(1 as ::core::ffi::c_int as isize) = b;
            return;
        }
        if b
            >= *p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                - 1 as ::core::ffi::c_int
            && b <= *p.offset(1 as ::core::ffi::c_int as isize)
            && a < *p.offset(0 as ::core::ffi::c_int as isize)
        {
            *p.offset(0 as ::core::ffi::c_int as isize) = a;
            return;
        }
        if a >= *p.offset(0 as ::core::ffi::c_int as isize)
            && a
                <= *p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    + 1 as ::core::ffi::c_int
            && b > *p.offset(1 as ::core::ffi::c_int as isize)
        {
            *p.offset(1 as ::core::ffi::c_int as isize) = b;
            return;
        }
        p = p.offset(2 as ::core::ffi::c_int as isize);
    }
    if (*cc).end.offset(2 as ::core::ffi::c_int as isize)
        >= (&raw mut (*cc).spans as *mut Rune)
            .offset(
                (::core::mem::size_of::<[Rune; 64]>() as usize)
                    .wrapping_div(::core::mem::size_of::<Rune>() as usize)
                    as ::core::ffi::c_int as isize,
            )
    {
        die(
            g,
            b"too many character class ranges\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    let fresh9 = (*cc).end;
    (*cc).end = (*cc).end.offset(1);
    *fresh9 = a;
    let fresh10 = (*cc).end;
    (*cc).end = (*cc).end.offset(1);
    *fresh10 = b;
}
unsafe extern "C" fn addranges_d(mut g: *mut cstate) {
    addrange(g, '0' as i32, '9' as i32);
}
unsafe extern "C" fn addranges_D(mut g: *mut cstate) {
    addrange(g, 0 as Rune, '0' as i32 - 1 as Rune);
    addrange(g, '9' as i32 + 1 as Rune, 0xffff as Rune);
}
unsafe extern "C" fn addranges_s(mut g: *mut cstate) {
    addrange(g, 0x9 as Rune, 0xd as Rune);
    addrange(g, 0x20 as Rune, 0x20 as Rune);
    addrange(g, 0xa0 as Rune, 0xa0 as Rune);
    addrange(g, 0x2028 as Rune, 0x2029 as Rune);
    addrange(g, 0xfeff as Rune, 0xfeff as Rune);
}
unsafe extern "C" fn addranges_S(mut g: *mut cstate) {
    addrange(g, 0 as Rune, 0x9 as Rune - 1 as Rune);
    addrange(g, 0xd as Rune + 1 as Rune, 0x20 as Rune - 1 as Rune);
    addrange(g, 0x20 as Rune + 1 as Rune, 0xa0 as Rune - 1 as Rune);
    addrange(g, 0xa0 as Rune + 1 as Rune, 0x2028 as Rune - 1 as Rune);
    addrange(g, 0x2029 as Rune + 1 as Rune, 0xfeff as Rune - 1 as Rune);
    addrange(g, 0xfeff as Rune + 1 as Rune, 0xffff as Rune);
}
unsafe extern "C" fn addranges_w(mut g: *mut cstate) {
    addrange(g, '0' as i32, '9' as i32);
    addrange(g, 'A' as i32, 'Z' as i32);
    addrange(g, '_' as i32, '_' as i32);
    addrange(g, 'a' as i32, 'z' as i32);
}
unsafe extern "C" fn addranges_W(mut g: *mut cstate) {
    addrange(g, 0 as Rune, '0' as i32 - 1 as Rune);
    addrange(g, '9' as i32 + 1 as Rune, 'A' as i32 - 1 as Rune);
    addrange(g, 'Z' as i32 + 1 as Rune, '_' as i32 - 1 as Rune);
    addrange(g, '_' as i32 + 1 as Rune, 'a' as i32 - 1 as Rune);
    addrange(g, 'z' as i32 + 1 as Rune, 0xffff as Rune);
}
unsafe extern "C" fn lexclass(mut g: *mut cstate) -> ::core::ffi::c_int {
    let mut type_0: ::core::ffi::c_int = L_CCLASS as ::core::ffi::c_int;
    let mut quoted: ::core::ffi::c_int = 0;
    let mut havesave: ::core::ffi::c_int = 0;
    let mut havedash: ::core::ffi::c_int = 0;
    let mut save: Rune = 0 as Rune;
    newcclass(g);
    quoted = nextrune(g);
    if quoted == 0 && (*g).yychar == '^' as i32 {
        type_0 = L_NCCLASS as ::core::ffi::c_int;
        quoted = nextrune(g);
    }
    havedash = 0 as ::core::ffi::c_int;
    havesave = havedash;
    loop {
        if (*g).yychar == EOF {
            die(
                g,
                b"unterminated character class\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        if quoted == 0 && (*g).yychar == ']' as i32 {
            break;
        }
        if quoted == 0 && (*g).yychar == '-' as i32 {
            if havesave != 0 {
                if havedash != 0 {
                    addrange(g, save, '-' as i32);
                    havedash = 0 as ::core::ffi::c_int;
                    havesave = havedash;
                } else {
                    havedash = 1 as ::core::ffi::c_int;
                }
            } else {
                save = '-' as i32 as Rune;
                havesave = 1 as ::core::ffi::c_int;
            }
        } else if quoted != 0
            && !strchr(
                    b"DSWdsw\0" as *const u8 as *const ::core::ffi::c_char,
                    (*g).yychar as ::core::ffi::c_int,
                )
                .is_null()
        {
            if havesave != 0 {
                addrange(g, save, save);
                if havedash != 0 {
                    addrange(g, '-' as i32, '-' as i32);
                }
            }
            match (*g).yychar {
                100 => {
                    addranges_d(g);
                }
                115 => {
                    addranges_s(g);
                }
                119 => {
                    addranges_w(g);
                }
                68 => {
                    addranges_D(g);
                }
                83 => {
                    addranges_S(g);
                }
                87 => {
                    addranges_W(g);
                }
                _ => {}
            }
            havedash = 0 as ::core::ffi::c_int;
            havesave = havedash;
        } else {
            if quoted != 0 {
                if (*g).yychar == 'b' as i32 {
                    (*g).yychar = '\u{8}' as i32 as Rune;
                } else if (*g).yychar == '0' as i32 {
                    (*g).yychar = 0 as ::core::ffi::c_int as Rune;
                }
            }
            if havesave != 0 {
                if havedash != 0 {
                    addrange(g, save, (*g).yychar);
                    havedash = 0 as ::core::ffi::c_int;
                    havesave = havedash;
                } else {
                    addrange(g, save, save);
                    save = (*g).yychar;
                }
            } else {
                save = (*g).yychar;
                havesave = 1 as ::core::ffi::c_int;
            }
        }
        quoted = nextrune(g);
    }
    if havesave != 0 {
        addrange(g, save, save);
        if havedash != 0 {
            addrange(g, '-' as i32, '-' as i32);
        }
    }
    return type_0;
}
unsafe extern "C" fn lex(mut g: *mut cstate) -> ::core::ffi::c_int {
    let mut quoted: ::core::ffi::c_int = nextrune(g);
    if quoted != 0 {
        match (*g).yychar {
            98 => return L_WORD as ::core::ffi::c_int,
            66 => return L_NWORD as ::core::ffi::c_int,
            100 => {
                newcclass(g);
                addranges_d(g);
                return L_CCLASS as ::core::ffi::c_int;
            }
            115 => {
                newcclass(g);
                addranges_s(g);
                return L_CCLASS as ::core::ffi::c_int;
            }
            119 => {
                newcclass(g);
                addranges_w(g);
                return L_CCLASS as ::core::ffi::c_int;
            }
            68 => {
                newcclass(g);
                addranges_d(g);
                return L_NCCLASS as ::core::ffi::c_int;
            }
            83 => {
                newcclass(g);
                addranges_s(g);
                return L_NCCLASS as ::core::ffi::c_int;
            }
            87 => {
                newcclass(g);
                addranges_w(g);
                return L_NCCLASS as ::core::ffi::c_int;
            }
            48 => {
                (*g).yychar = 0 as ::core::ffi::c_int as Rune;
                return L_CHAR as ::core::ffi::c_int;
            }
            _ => {}
        }
        if (*g).yychar >= '0' as i32 && (*g).yychar <= '9' as i32 {
            (*g).yychar -= '0' as i32;
            if *(*g).source as ::core::ffi::c_int >= '0' as i32
                && *(*g).source as ::core::ffi::c_int <= '9' as i32
            {
                let fresh8 = (*g).source;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = ((*g).yychar as ::core::ffi::c_int
                    * 10 as ::core::ffi::c_int + *fresh8 as ::core::ffi::c_int
                    - '0' as i32) as Rune;
            }
            return L_REF as ::core::ffi::c_int;
        }
        return L_CHAR as ::core::ffi::c_int;
    }
    match (*g).yychar {
        EOF | 36 | 41 | 42 | 43 | 46 | 63 | 94 | 124 => {
            return (*g).yychar as ::core::ffi::c_int;
        }
        _ => {}
    }
    if (*g).yychar == '{' as i32 {
        return lexcount(g);
    }
    if (*g).yychar == '[' as i32 {
        return lexclass(g);
    }
    if (*g).yychar == '(' as i32 {
        if *(*g).source.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == '?' as i32
        {
            if *(*g).source.offset(1 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int == ':' as i32
            {
                (*g).source = (*g).source.offset(2 as ::core::ffi::c_int as isize);
                return L_NC as ::core::ffi::c_int;
            }
            if *(*g).source.offset(1 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int == '=' as i32
            {
                (*g).source = (*g).source.offset(2 as ::core::ffi::c_int as isize);
                return L_PLA as ::core::ffi::c_int;
            }
            if *(*g).source.offset(1 as ::core::ffi::c_int as isize)
                as ::core::ffi::c_int == '!' as i32
            {
                (*g).source = (*g).source.offset(2 as ::core::ffi::c_int as isize);
                return L_NLA as ::core::ffi::c_int;
            }
        }
        return '(' as i32;
    }
    return L_CHAR as ::core::ffi::c_int;
}
unsafe extern "C" fn newnode(
    mut g: *mut cstate,
    mut type_0: ::core::ffi::c_int,
) -> *mut Renode {
    let fresh25 = (*g).pend;
    (*g).pend = (*g).pend.offset(1);
    let mut node: *mut Renode = fresh25;
    (*node).type_0 = type_0 as ::core::ffi::c_uchar;
    (*node).cc = -(1 as ::core::ffi::c_int);
    (*node).c = 0 as ::core::ffi::c_int as Rune;
    (*node).ng = 0 as ::core::ffi::c_uchar;
    (*node).m = 0 as ::core::ffi::c_uchar;
    (*node).n = 0 as ::core::ffi::c_uchar;
    (*node).y = ::core::ptr::null_mut::<Renode>();
    (*node).x = (*node).y;
    return node;
}
unsafe extern "C" fn empty(mut node: *mut Renode) -> ::core::ffi::c_int {
    if node.is_null() {
        return 1 as ::core::ffi::c_int;
    }
    match (*node).type_0 as ::core::ffi::c_int {
        0 => {
            return (empty((*node).x) != 0 && empty((*node).y) != 0) as ::core::ffi::c_int;
        }
        1 => {
            return (empty((*node).x) != 0 || empty((*node).y) != 0) as ::core::ffi::c_int;
        }
        2 => {
            return (empty((*node).x) != 0
                || (*node).m as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
        }
        7 => return empty((*node).x),
        14 => return empty((*node).x),
        10 | 11 | 12 | 13 => return 0 as ::core::ffi::c_int,
        _ => return 1 as ::core::ffi::c_int,
    };
}
unsafe extern "C" fn newrep(
    mut g: *mut cstate,
    mut atom: *mut Renode,
    mut ng: ::core::ffi::c_int,
    mut min: ::core::ffi::c_int,
    mut max: ::core::ffi::c_int,
) -> *mut Renode {
    let mut rep: *mut Renode = newnode(g, P_REP as ::core::ffi::c_int);
    if max == REPINF && empty(atom) != 0 {
        die(
            g,
            b"infinite loop matching the empty string\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    (*rep).ng = ng as ::core::ffi::c_uchar;
    (*rep).m = min as ::core::ffi::c_uchar;
    (*rep).n = max as ::core::ffi::c_uchar;
    (*rep).x = atom;
    return rep;
}
unsafe extern "C" fn regnext(mut g: *mut cstate) {
    (*g).lookahead = lex(g);
}
unsafe extern "C" fn regaccept(
    mut g: *mut cstate,
    mut t: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if (*g).lookahead == t {
        regnext(g);
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn parseatom(mut g: *mut cstate) -> *mut Renode {
    let mut atom: *mut Renode = ::core::ptr::null_mut::<Renode>();
    if (*g).lookahead == L_CHAR as ::core::ffi::c_int {
        atom = newnode(g, P_CHAR as ::core::ffi::c_int);
        (*atom).c = (*g).yychar;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_CCLASS as ::core::ffi::c_int {
        atom = newnode(g, P_CCLASS as ::core::ffi::c_int);
        (*atom).cc = (*g).yycc.offset_from(&raw mut (*g).cclass as *mut Reclass)
            as ::core::ffi::c_long as ::core::ffi::c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_NCCLASS as ::core::ffi::c_int {
        atom = newnode(g, P_NCCLASS as ::core::ffi::c_int);
        (*atom).cc = (*g).yycc.offset_from(&raw mut (*g).cclass as *mut Reclass)
            as ::core::ffi::c_long as ::core::ffi::c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_REF as ::core::ffi::c_int {
        atom = newnode(g, P_REF as ::core::ffi::c_int);
        if (*g).yychar == 0 as ::core::ffi::c_int || (*g).yychar >= (*g).nsub
            || (*g).sub[(*g).yychar as usize].is_null()
        {
            die(
                g,
                b"invalid back-reference\0" as *const u8 as *const ::core::ffi::c_char,
            );
        }
        (*atom).n = (*g).yychar as ::core::ffi::c_uchar;
        (*atom).x = (*g).sub[(*g).yychar as usize];
        regnext(g);
        return atom;
    }
    if regaccept(g, '.' as i32) != 0 {
        return newnode(g, P_ANY as ::core::ffi::c_int);
    }
    if regaccept(g, '(' as i32) != 0 {
        atom = newnode(g, P_PAR as ::core::ffi::c_int);
        if (*g).nsub == REG_MAXSUB {
            die(g, b"too many captures\0" as *const u8 as *const ::core::ffi::c_char);
        }
        let fresh26 = (*g).nsub;
        (*g).nsub = (*g).nsub + 1;
        (*atom).n = fresh26 as ::core::ffi::c_uchar;
        (*atom).x = parsealt(g);
        (*g).sub[(*atom).n as usize] = atom;
        if regaccept(g, ')' as i32) == 0 {
            die(g, b"unmatched '('\0" as *const u8 as *const ::core::ffi::c_char);
        }
        return atom;
    }
    if regaccept(g, L_NC as ::core::ffi::c_int) != 0 {
        atom = parsealt(g);
        if regaccept(g, ')' as i32) == 0 {
            die(g, b"unmatched '('\0" as *const u8 as *const ::core::ffi::c_char);
        }
        return atom;
    }
    if regaccept(g, L_PLA as ::core::ffi::c_int) != 0 {
        atom = newnode(g, P_PLA as ::core::ffi::c_int);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as i32) == 0 {
            die(g, b"unmatched '('\0" as *const u8 as *const ::core::ffi::c_char);
        }
        return atom;
    }
    if regaccept(g, L_NLA as ::core::ffi::c_int) != 0 {
        atom = newnode(g, P_NLA as ::core::ffi::c_int);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as i32) == 0 {
            die(g, b"unmatched '('\0" as *const u8 as *const ::core::ffi::c_char);
        }
        return atom;
    }
    die(g, b"syntax error\0" as *const u8 as *const ::core::ffi::c_char);
    return ::core::ptr::null_mut::<Renode>();
}
unsafe extern "C" fn parserep(mut g: *mut cstate) -> *mut Renode {
    let mut atom: *mut Renode = ::core::ptr::null_mut::<Renode>();
    if regaccept(g, '^' as i32) != 0 {
        return newnode(g, P_BOL as ::core::ffi::c_int);
    }
    if regaccept(g, '$' as i32) != 0 {
        return newnode(g, P_EOL as ::core::ffi::c_int);
    }
    if regaccept(g, L_WORD as ::core::ffi::c_int) != 0 {
        return newnode(g, P_WORD as ::core::ffi::c_int);
    }
    if regaccept(g, L_NWORD as ::core::ffi::c_int) != 0 {
        return newnode(g, P_NWORD as ::core::ffi::c_int);
    }
    atom = parseatom(g);
    if (*g).lookahead == L_COUNT as ::core::ffi::c_int {
        let mut min: ::core::ffi::c_int = (*g).yymin;
        let mut max: ::core::ffi::c_int = (*g).yymax;
        regnext(g);
        if max < min {
            die(g, b"invalid quantifier\0" as *const u8 as *const ::core::ffi::c_char);
        }
        return newrep(g, atom, regaccept(g, '?' as i32), min, max);
    }
    if regaccept(g, '*' as i32) != 0 {
        return newrep(
            g,
            atom,
            regaccept(g, '?' as i32),
            0 as ::core::ffi::c_int,
            REPINF,
        );
    }
    if regaccept(g, '+' as i32) != 0 {
        return newrep(
            g,
            atom,
            regaccept(g, '?' as i32),
            1 as ::core::ffi::c_int,
            REPINF,
        );
    }
    if regaccept(g, '?' as i32) != 0 {
        return newrep(
            g,
            atom,
            regaccept(g, '?' as i32),
            0 as ::core::ffi::c_int,
            1 as ::core::ffi::c_int,
        );
    }
    return atom;
}
unsafe extern "C" fn parsecat(mut g: *mut cstate) -> *mut Renode {
    let mut cat: *mut Renode = ::core::ptr::null_mut::<Renode>();
    let mut head: *mut Renode = ::core::ptr::null_mut::<Renode>();
    let mut tail: *mut *mut Renode = ::core::ptr::null_mut::<*mut Renode>();
    if (*g).lookahead != EOF && (*g).lookahead != '|' as i32
        && (*g).lookahead != ')' as i32
    {
        head = parserep(g);
        tail = &raw mut head;
        while (*g).lookahead != EOF && (*g).lookahead != '|' as i32
            && (*g).lookahead != ')' as i32
        {
            cat = newnode(g, P_CAT as ::core::ffi::c_int);
            (*cat).x = *tail;
            (*cat).y = parserep(g);
            *tail = cat;
            tail = &raw mut (*cat).y;
        }
        return head;
    }
    return ::core::ptr::null_mut::<Renode>();
}
unsafe extern "C" fn parsealt(mut g: *mut cstate) -> *mut Renode {
    let mut alt: *mut Renode = ::core::ptr::null_mut::<Renode>();
    let mut x: *mut Renode = ::core::ptr::null_mut::<Renode>();
    alt = parsecat(g);
    while regaccept(g, '|' as i32) != 0 {
        x = alt;
        alt = newnode(g, P_ALT as ::core::ffi::c_int);
        (*alt).x = x;
        (*alt).y = parsecat(g);
    }
    return alt;
}
unsafe extern "C" fn count(
    mut g: *mut cstate,
    mut node: *mut Renode,
    mut depth: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut min: ::core::ffi::c_int = 0;
    let mut max: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    if node.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    depth += 1;
    if depth > REG_MAXREC {
        die(g, b"stack overflow\0" as *const u8 as *const ::core::ffi::c_char);
    }
    match (*node).type_0 as ::core::ffi::c_int {
        0 => return count(g, (*node).x, depth) + count(g, (*node).y, depth),
        1 => {
            return count(g, (*node).x, depth) + count(g, (*node).y, depth)
                + 2 as ::core::ffi::c_int;
        }
        2 => {
            min = (*node).m as ::core::ffi::c_int;
            max = (*node).n as ::core::ffi::c_int;
            if min == max {
                n = count(g, (*node).x, depth) * min;
            } else if max < REPINF {
                n = count(g, (*node).x, depth) * max + (max - min);
            } else {
                n = count(g, (*node).x, depth) * (min + 1 as ::core::ffi::c_int)
                    + 2 as ::core::ffi::c_int;
            }
            if n < 0 as ::core::ffi::c_int || n > REG_MAXPROG {
                die(
                    g,
                    b"program too large\0" as *const u8 as *const ::core::ffi::c_char,
                );
            }
            return n;
        }
        7 => return count(g, (*node).x, depth) + 2 as ::core::ffi::c_int,
        8 => return count(g, (*node).x, depth) + 2 as ::core::ffi::c_int,
        9 => return count(g, (*node).x, depth) + 2 as ::core::ffi::c_int,
        _ => return 1 as ::core::ffi::c_int,
    };
}
unsafe extern "C" fn regemit(
    mut prog: *mut Reprog,
    mut opcode: ::core::ffi::c_int,
) -> *mut Reinst {
    let fresh7 = (*prog).end;
    (*prog).end = (*prog).end.offset(1);
    let mut inst: *mut Reinst = fresh7;
    (*inst).opcode = opcode as ::core::ffi::c_uchar;
    (*inst).n = 0 as ::core::ffi::c_uchar;
    (*inst).c = 0 as ::core::ffi::c_int as Rune;
    (*inst).cc = ::core::ptr::null_mut::<Reclass>();
    (*inst).y = ::core::ptr::null_mut::<Reinst>();
    (*inst).x = (*inst).y;
    return inst;
}
unsafe extern "C" fn compile(mut prog: *mut Reprog, mut node: *mut Renode) {
    let mut current_block: u64;
    let mut inst: *mut Reinst = ::core::ptr::null_mut::<Reinst>();
    let mut split: *mut Reinst = ::core::ptr::null_mut::<Reinst>();
    let mut jump: *mut Reinst = ::core::ptr::null_mut::<Reinst>();
    let mut i: ::core::ffi::c_int = 0;
    loop {
        if node.is_null() {
            return;
        }
        match (*node).type_0 as ::core::ffi::c_int {
            0 => {
                compile(prog, (*node).x);
                node = (*node).y;
            }
            1 => {
                split = regemit(prog, I_SPLIT as ::core::ffi::c_int);
                compile(prog, (*node).x);
                jump = regemit(prog, I_JUMP as ::core::ffi::c_int);
                compile(prog, (*node).y);
                (*split).x = split.offset(1 as ::core::ffi::c_int as isize);
                (*split).y = jump.offset(1 as ::core::ffi::c_int as isize);
                (*jump).x = (*prog).end;
                current_block = 13853033528615664019;
                break;
            }
            2 => {
                inst = ::core::ptr::null_mut::<Reinst>();
                i = 0 as ::core::ffi::c_int;
                while i < (*node).m as ::core::ffi::c_int {
                    inst = (*prog).end;
                    compile(prog, (*node).x);
                    i += 1;
                }
                if (*node).m as ::core::ffi::c_int == (*node).n as ::core::ffi::c_int {
                    current_block = 13853033528615664019;
                    break;
                } else {
                    current_block = 8831408221741692167;
                    break;
                }
            }
            3 => {
                regemit(prog, I_BOL as ::core::ffi::c_int);
                current_block = 13853033528615664019;
                break;
            }
            4 => {
                regemit(prog, I_EOL as ::core::ffi::c_int);
                current_block = 13853033528615664019;
                break;
            }
            5 => {
                regemit(prog, I_WORD as ::core::ffi::c_int);
                current_block = 13853033528615664019;
                break;
            }
            6 => {
                regemit(prog, I_NWORD as ::core::ffi::c_int);
                current_block = 13853033528615664019;
                break;
            }
            7 => {
                inst = regemit(prog, I_LPAR as ::core::ffi::c_int);
                (*inst).n = (*node).n;
                compile(prog, (*node).x);
                inst = regemit(prog, I_RPAR as ::core::ffi::c_int);
                (*inst).n = (*node).n;
                current_block = 13853033528615664019;
                break;
            }
            8 => {
                split = regemit(prog, I_PLA as ::core::ffi::c_int);
                compile(prog, (*node).x);
                regemit(prog, I_END as ::core::ffi::c_int);
                (*split).x = split.offset(1 as ::core::ffi::c_int as isize);
                (*split).y = (*prog).end;
                current_block = 13853033528615664019;
                break;
            }
            9 => {
                split = regemit(prog, I_NLA as ::core::ffi::c_int);
                compile(prog, (*node).x);
                regemit(prog, I_END as ::core::ffi::c_int);
                (*split).x = split.offset(1 as ::core::ffi::c_int as isize);
                (*split).y = (*prog).end;
                current_block = 13853033528615664019;
                break;
            }
            10 => {
                regemit(prog, I_ANY as ::core::ffi::c_int);
                current_block = 13853033528615664019;
                break;
            }
            11 => {
                inst = regemit(prog, I_CHAR as ::core::ffi::c_int);
                (*inst).c = (if (*prog).flags & REG_ICASE as ::core::ffi::c_int != 0 {
                    canon((*node).c)
                } else {
                    (*node).c as ::core::ffi::c_int
                }) as Rune;
                current_block = 13853033528615664019;
                break;
            }
            12 => {
                inst = regemit(prog, I_CCLASS as ::core::ffi::c_int);
                (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                current_block = 13853033528615664019;
                break;
            }
            13 => {
                inst = regemit(prog, I_NCCLASS as ::core::ffi::c_int);
                (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                current_block = 13853033528615664019;
                break;
            }
            14 => {
                inst = regemit(prog, I_REF as ::core::ffi::c_int);
                (*inst).n = (*node).n;
                current_block = 13853033528615664019;
                break;
            }
            _ => {
                current_block = 13853033528615664019;
                break;
            }
        }
    }
    match current_block {
        8831408221741692167 => {
            if ((*node).n as ::core::ffi::c_int) < REPINF {
                i = (*node).m as ::core::ffi::c_int;
                while i < (*node).n as ::core::ffi::c_int {
                    split = regemit(prog, I_SPLIT as ::core::ffi::c_int);
                    compile(prog, (*node).x);
                    if (*node).ng != 0 {
                        (*split).y = split.offset(1 as ::core::ffi::c_int as isize);
                        (*split).x = (*prog).end;
                    } else {
                        (*split).x = split.offset(1 as ::core::ffi::c_int as isize);
                        (*split).y = (*prog).end;
                    }
                    i += 1;
                }
            } else if (*node).m as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                split = regemit(prog, I_SPLIT as ::core::ffi::c_int);
                compile(prog, (*node).x);
                jump = regemit(prog, I_JUMP as ::core::ffi::c_int);
                if (*node).ng != 0 {
                    (*split).y = split.offset(1 as ::core::ffi::c_int as isize);
                    (*split).x = (*prog).end;
                } else {
                    (*split).x = split.offset(1 as ::core::ffi::c_int as isize);
                    (*split).y = (*prog).end;
                }
                (*jump).x = split;
            } else {
                split = regemit(prog, I_SPLIT as ::core::ffi::c_int);
                if (*node).ng != 0 {
                    (*split).y = inst;
                    (*split).x = (*prog).end;
                } else {
                    (*split).x = inst;
                    (*split).y = (*prog).end;
                }
            }
        }
        _ => {}
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_regcompx(
    mut alloc: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut ::core::ffi::c_void,
            ::core::ffi::c_int,
        ) -> *mut ::core::ffi::c_void,
    >,
    mut ctx: *mut ::core::ffi::c_void,
    mut pattern: *const ::core::ffi::c_char,
    mut cflags: ::core::ffi::c_int,
    mut errorp: *mut *const ::core::ffi::c_char,
) -> *mut Reprog {
    let mut g: cstate = cstate {
        prog: ::core::ptr::null_mut::<Reprog>(),
        pstart: ::core::ptr::null_mut::<Renode>(),
        pend: ::core::ptr::null_mut::<Renode>(),
        source: ::core::ptr::null::<::core::ffi::c_char>(),
        ncclass: 0,
        nsub: 0,
        sub: [::core::ptr::null_mut::<Renode>(); 16],
        lookahead: 0,
        yychar: 0,
        yycc: ::core::ptr::null_mut::<Reclass>(),
        yymin: 0,
        yymax: 0,
        error: ::core::ptr::null::<::core::ffi::c_char>(),
        kaboom: [__jmp_buf_tag {
            __jmpbuf: [0; 8],
            __mask_was_saved: 0,
            __saved_mask: __sigset_t { __val: [0; 16] },
        }; 1],
        cclass: [Reclass {
            end: ::core::ptr::null_mut::<Rune>(),
            spans: [0; 64],
        }; 128],
    };
    let mut node: *mut Renode = ::core::ptr::null_mut::<Renode>();
    let mut split: *mut Reinst = ::core::ptr::null_mut::<Reinst>();
    let mut jump: *mut Reinst = ::core::ptr::null_mut::<Reinst>();
    let mut i: ::core::ffi::c_int = 0;
    let mut n: ::core::ffi::c_int = 0;
    g.pstart = ::core::ptr::null_mut::<Renode>();
    g.prog = ::core::ptr::null_mut::<Reprog>();
    if _setjmp(&raw mut g.kaboom as *mut __jmp_buf_tag) != 0 {
        if !errorp.is_null() {
            *errorp = g.error;
        }
        alloc
            .expect(
                "non-null function pointer",
            )(ctx, g.pstart as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
        if !g.prog.is_null() {
            alloc
                .expect(
                    "non-null function pointer",
                )(
                ctx,
                (*g.prog).cclass as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
            );
            alloc
                .expect(
                    "non-null function pointer",
                )(
                ctx,
                (*g.prog).start as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
            );
            alloc
                .expect(
                    "non-null function pointer",
                )(ctx, g.prog as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
        }
        return ::core::ptr::null_mut::<Reprog>();
    }
    g.prog = alloc
        .expect(
            "non-null function pointer",
        )(ctx, NULL_0, ::core::mem::size_of::<Reprog>() as ::core::ffi::c_int)
        as *mut Reprog;
    if g.prog.is_null() {
        die(
            &raw mut g,
            b"cannot allocate regular expression\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    (*g.prog).start = ::core::ptr::null_mut::<Reinst>();
    (*g.prog).cclass = ::core::ptr::null_mut::<Reclass>();
    n = strlen(pattern).wrapping_mul(2 as size_t) as ::core::ffi::c_int;
    if n > REG_MAXPROG {
        die(
            &raw mut g,
            b"program too large\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    if n > 0 as ::core::ffi::c_int {
        g.pend = alloc
            .expect(
                "non-null function pointer",
            )(
            ctx,
            NULL_0,
            (::core::mem::size_of::<Renode>() as usize).wrapping_mul(n as usize)
                as ::core::ffi::c_int,
        ) as *mut Renode;
        g.pstart = g.pend;
        if g.pstart.is_null() {
            die(
                &raw mut g,
                b"cannot allocate regular expression parse list\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
    }
    g.source = pattern;
    g.ncclass = 0 as ::core::ffi::c_int;
    g.nsub = 1 as ::core::ffi::c_int;
    i = 0 as ::core::ffi::c_int;
    while i < REG_MAXSUB {
        g.sub[i as usize] = ::core::ptr::null_mut::<Renode>();
        i += 1;
    }
    (*g.prog).flags = cflags;
    regnext(&raw mut g);
    node = parsealt(&raw mut g);
    if g.lookahead == ')' as i32 {
        die(&raw mut g, b"unmatched ')'\0" as *const u8 as *const ::core::ffi::c_char);
    }
    if g.lookahead != EOF {
        die(&raw mut g, b"syntax error\0" as *const u8 as *const ::core::ffi::c_char);
    }
    n = 6 as ::core::ffi::c_int + count(&raw mut g, node, 0 as ::core::ffi::c_int);
    if n < 0 as ::core::ffi::c_int || n > REG_MAXPROG {
        die(
            &raw mut g,
            b"program too large\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    (*g.prog).nsub = g.nsub;
    (*g.prog).end = alloc
        .expect(
            "non-null function pointer",
        )(
        ctx,
        NULL_0,
        (n as usize).wrapping_mul(::core::mem::size_of::<Reinst>() as usize)
            as ::core::ffi::c_int,
    ) as *mut Reinst;
    (*g.prog).start = (*g.prog).end;
    if (*g.prog).start.is_null() {
        die(
            &raw mut g,
            b"cannot allocate regular expression instruction list\0" as *const u8
                as *const ::core::ffi::c_char,
        );
    }
    if g.ncclass > 0 as ::core::ffi::c_int {
        (*g.prog).cclass = alloc
            .expect(
                "non-null function pointer",
            )(
            ctx,
            NULL_0,
            (g.ncclass as usize).wrapping_mul(::core::mem::size_of::<Reclass>() as usize)
                as ::core::ffi::c_int,
        ) as *mut Reclass;
        if (*g.prog).cclass.is_null() {
            die(
                &raw mut g,
                b"cannot allocate regular expression character class list\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
        }
        memcpy(
            (*g.prog).cclass as *mut ::core::ffi::c_void,
            &raw mut g.cclass as *mut Reclass as *const ::core::ffi::c_void,
            (g.ncclass as size_t)
                .wrapping_mul(::core::mem::size_of::<Reclass>() as size_t),
        );
        i = 0 as ::core::ffi::c_int;
        while i < g.ncclass {
            let ref mut fresh6 = (*(*g.prog).cclass.offset(i as isize)).end;
            *fresh6 = (&raw mut (*(*g.prog).cclass.offset(i as isize)).spans
                as *mut Rune)
                .offset(
                    g
                        .cclass[i as usize]
                        .end
                        .offset_from(
                            &raw mut (*(&raw mut g.cclass as *mut Reclass)
                                .offset(i as isize))
                                .spans as *mut Rune,
                        ) as ::core::ffi::c_long as isize,
                );
            i += 1;
        }
    }
    split = regemit(g.prog, I_SPLIT as ::core::ffi::c_int);
    (*split).x = split.offset(3 as ::core::ffi::c_int as isize);
    (*split).y = split.offset(1 as ::core::ffi::c_int as isize);
    regemit(g.prog, I_ANYNL as ::core::ffi::c_int);
    jump = regemit(g.prog, I_JUMP as ::core::ffi::c_int);
    (*jump).x = split;
    regemit(g.prog, I_LPAR as ::core::ffi::c_int);
    compile(g.prog, node);
    regemit(g.prog, I_RPAR as ::core::ffi::c_int);
    regemit(g.prog, I_END as ::core::ffi::c_int);
    alloc
        .expect(
            "non-null function pointer",
        )(ctx, g.pstart as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
    if !errorp.is_null() {
        *errorp = ::core::ptr::null::<::core::ffi::c_char>();
    }
    return g.prog;
}
#[no_mangle]
pub unsafe extern "C" fn js_regfreex(
    mut alloc: Option<
        unsafe extern "C" fn(
            *mut ::core::ffi::c_void,
            *mut ::core::ffi::c_void,
            ::core::ffi::c_int,
        ) -> *mut ::core::ffi::c_void,
    >,
    mut ctx: *mut ::core::ffi::c_void,
    mut prog: *mut Reprog,
) {
    if !prog.is_null() {
        if !(*prog).cclass.is_null() {
            alloc
                .expect(
                    "non-null function pointer",
                )(
                ctx,
                (*prog).cclass as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
            );
        }
        alloc
            .expect(
                "non-null function pointer",
            )(ctx, (*prog).start as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
        alloc
            .expect(
                "non-null function pointer",
            )(ctx, prog as *mut ::core::ffi::c_void, 0 as ::core::ffi::c_int);
    }
}
unsafe extern "C" fn default_alloc(
    mut ctx: *mut ::core::ffi::c_void,
    mut p: *mut ::core::ffi::c_void,
    mut n: ::core::ffi::c_int,
) -> *mut ::core::ffi::c_void {
    if n == 0 as ::core::ffi::c_int {
        free(p);
        return NULL_0;
    }
    return realloc(p, n as size_t);
}
#[no_mangle]
pub unsafe extern "C" fn js_regcomp(
    mut pattern: *const ::core::ffi::c_char,
    mut cflags: ::core::ffi::c_int,
    mut errorp: *mut *const ::core::ffi::c_char,
) -> *mut Reprog {
    return js_regcompx(
        Some(
            default_alloc
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *mut ::core::ffi::c_void,
                    ::core::ffi::c_int,
                ) -> *mut ::core::ffi::c_void,
        ),
        NULL_0,
        pattern,
        cflags,
        errorp,
    );
}
#[no_mangle]
pub unsafe extern "C" fn js_regfree(mut prog: *mut Reprog) {
    js_regfreex(
        Some(
            default_alloc
                as unsafe extern "C" fn(
                    *mut ::core::ffi::c_void,
                    *mut ::core::ffi::c_void,
                    ::core::ffi::c_int,
                ) -> *mut ::core::ffi::c_void,
        ),
        NULL_0,
        prog,
    );
}
unsafe extern "C" fn isnewline(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c == 0xa as ::core::ffi::c_int || c == 0xd as ::core::ffi::c_int
        || c == 0x2028 as ::core::ffi::c_int || c == 0x2029 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn iswordchar(mut c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return (c == '_' as i32 || c >= 'a' as i32 && c <= 'z' as i32
        || c >= 'A' as i32 && c <= 'Z' as i32 || c >= '0' as i32 && c <= '9' as i32)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn incclass(mut cc: *mut Reclass, mut c: Rune) -> ::core::ffi::c_int {
    let mut p: *mut Rune = ::core::ptr::null_mut::<Rune>();
    p = &raw mut (*cc).spans as *mut Rune;
    while p < (*cc).end {
        if *p.offset(0 as ::core::ffi::c_int as isize) <= c
            && c <= *p.offset(1 as ::core::ffi::c_int as isize)
        {
            return 1 as ::core::ffi::c_int;
        }
        p = p.offset(2 as ::core::ffi::c_int as isize);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn incclasscanon(
    mut cc: *mut Reclass,
    mut c: Rune,
) -> ::core::ffi::c_int {
    let mut p: *mut Rune = ::core::ptr::null_mut::<Rune>();
    let mut r: Rune = 0;
    p = &raw mut (*cc).spans as *mut Rune;
    while p < (*cc).end {
        r = *p.offset(0 as ::core::ffi::c_int as isize);
        while r <= *p.offset(1 as ::core::ffi::c_int as isize) {
            if c == canon(r) {
                return 1 as ::core::ffi::c_int;
            }
            r += 1;
        }
        p = p.offset(2 as ::core::ffi::c_int as isize);
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn strncmpcanon(
    mut a: *const ::core::ffi::c_char,
    mut b: *const ::core::ffi::c_char,
    mut n: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut ra: Rune = 0;
    let mut rb: Rune = 0;
    let mut c: ::core::ffi::c_int = 0;
    loop {
        let fresh27 = n;
        n = n - 1;
        if !(fresh27 != 0) {
            break;
        }
        if *a == 0 {
            return -(1 as ::core::ffi::c_int);
        }
        if *b == 0 {
            return 1 as ::core::ffi::c_int;
        }
        a = a.offset(jsU_chartorune(&raw mut ra, a) as isize);
        b = b.offset(jsU_chartorune(&raw mut rb, b) as isize);
        c = canon(ra) - canon(rb);
        if c != 0 {
            return c;
        }
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn match_0(
    mut pc: *mut Reinst,
    mut sp: *const ::core::ffi::c_char,
    mut bol: *const ::core::ffi::c_char,
    mut flags: ::core::ffi::c_int,
    mut out: *mut Resub,
    mut depth: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut scratch: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    let mut result: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut c: Rune = 0;
    if depth > REG_MAXREC {
        return -(1 as ::core::ffi::c_int);
    }
    loop {
        let mut current_block_97: u64;
        match (*pc).opcode as ::core::ffi::c_int {
            0 => return 0 as ::core::ffi::c_int,
            1 => {
                pc = (*pc).x;
            }
            2 => {
                scratch = *out;
                result = match_0(
                    (*pc).x,
                    sp,
                    bol,
                    flags,
                    &raw mut scratch,
                    depth + 1 as ::core::ffi::c_int,
                );
                if result == -(1 as ::core::ffi::c_int) {
                    return -(1 as ::core::ffi::c_int);
                }
                if result == 0 as ::core::ffi::c_int {
                    *out = scratch;
                    return 0 as ::core::ffi::c_int;
                }
                pc = (*pc).y;
            }
            3 => {
                result = match_0(
                    (*pc).x,
                    sp,
                    bol,
                    flags,
                    out,
                    depth + 1 as ::core::ffi::c_int,
                );
                if result == -(1 as ::core::ffi::c_int) {
                    return -(1 as ::core::ffi::c_int);
                }
                if result == 1 as ::core::ffi::c_int {
                    return 1 as ::core::ffi::c_int;
                }
                pc = (*pc).y;
            }
            4 => {
                scratch = *out;
                result = match_0(
                    (*pc).x,
                    sp,
                    bol,
                    flags,
                    &raw mut scratch,
                    depth + 1 as ::core::ffi::c_int,
                );
                if result == -(1 as ::core::ffi::c_int) {
                    return -(1 as ::core::ffi::c_int);
                }
                if result == 0 as ::core::ffi::c_int {
                    return 1 as ::core::ffi::c_int;
                }
                pc = (*pc).y;
            }
            5 => {
                if *sp == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            6 => {
                if *sp == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                if isnewline(c as ::core::ffi::c_int) != 0 {
                    return 1 as ::core::ffi::c_int;
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            7 => {
                if *sp == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                if flags & REG_ICASE as ::core::ffi::c_int != 0 {
                    c = canon(c) as Rune;
                }
                if c != (*pc).c {
                    return 1 as ::core::ffi::c_int;
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            8 => {
                if *sp == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                if flags & REG_ICASE as ::core::ffi::c_int != 0 {
                    if incclasscanon((*pc).cc, canon(c) as Rune) == 0 {
                        return 1 as ::core::ffi::c_int;
                    }
                } else if incclass((*pc).cc, c) == 0 {
                    return 1 as ::core::ffi::c_int
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            9 => {
                if *sp == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                if flags & REG_ICASE as ::core::ffi::c_int != 0 {
                    if incclasscanon((*pc).cc, canon(c) as Rune) != 0 {
                        return 1 as ::core::ffi::c_int;
                    }
                } else if incclass((*pc).cc, c) != 0 {
                    return 1 as ::core::ffi::c_int
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            10 => {
                i = (*out)
                    .sub[(*pc).n as usize]
                    .ep
                    .offset_from((*out).sub[(*pc).n as usize].sp) as ::core::ffi::c_long
                    as ::core::ffi::c_int;
                if flags & REG_ICASE as ::core::ffi::c_int != 0 {
                    if strncmpcanon(sp, (*out).sub[(*pc).n as usize].sp, i) != 0 {
                        return 1 as ::core::ffi::c_int;
                    }
                } else if strncmp(sp, (*out).sub[(*pc).n as usize].sp, i as size_t) != 0
                {
                    return 1 as ::core::ffi::c_int
                }
                if i > 0 as ::core::ffi::c_int {
                    sp = sp.offset(i as isize);
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            11 => {
                if sp == bol && flags & REG_NOTBOL as ::core::ffi::c_int == 0 {
                    pc = pc.offset(1 as ::core::ffi::c_int as isize);
                } else {
                    if flags & REG_NEWLINE as ::core::ffi::c_int != 0 {
                        if sp > bol
                            && isnewline(
                                *sp.offset(-(1 as ::core::ffi::c_int) as isize)
                                    as ::core::ffi::c_int,
                            ) != 0
                        {
                            pc = pc.offset(1 as ::core::ffi::c_int as isize);
                            current_block_97 = 6471821049853688503;
                        } else {
                            current_block_97 = 15462640364611497761;
                        }
                    } else {
                        current_block_97 = 15462640364611497761;
                    }
                    match current_block_97 {
                        6471821049853688503 => {}
                        _ => return 1 as ::core::ffi::c_int,
                    }
                }
            }
            12 => {
                if *sp as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                    pc = pc.offset(1 as ::core::ffi::c_int as isize);
                } else {
                    if flags & REG_NEWLINE as ::core::ffi::c_int != 0 {
                        if isnewline(*sp as ::core::ffi::c_int) != 0 {
                            pc = pc.offset(1 as ::core::ffi::c_int as isize);
                            current_block_97 = 6471821049853688503;
                        } else {
                            current_block_97 = 5793491756164225964;
                        }
                    } else {
                        current_block_97 = 5793491756164225964;
                    }
                    match current_block_97 {
                        6471821049853688503 => {}
                        _ => return 1 as ::core::ffi::c_int,
                    }
                }
            }
            13 => {
                i = (sp > bol
                    && iswordchar(
                        *sp.offset(-(1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int,
                    ) != 0) as ::core::ffi::c_int;
                i
                    ^= iswordchar(
                        *sp.offset(0 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int,
                    );
                if i == 0 {
                    return 1 as ::core::ffi::c_int;
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            14 => {
                i = (sp > bol
                    && iswordchar(
                        *sp.offset(-(1 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int,
                    ) != 0) as ::core::ffi::c_int;
                i
                    ^= iswordchar(
                        *sp.offset(0 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int,
                    );
                if i != 0 {
                    return 1 as ::core::ffi::c_int;
                }
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            15 => {
                (*out).sub[(*pc).n as usize].sp = sp;
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            16 => {
                (*out).sub[(*pc).n as usize].ep = sp;
                pc = pc.offset(1 as ::core::ffi::c_int as isize);
            }
            _ => return 1 as ::core::ffi::c_int,
        }
    };
}
#[no_mangle]
pub unsafe extern "C" fn js_regexec(
    mut prog: *mut Reprog,
    mut sp: *const ::core::ffi::c_char,
    mut sub: *mut Resub,
    mut eflags: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut scratch: Resub = Resub {
        nsub: 0,
        sub: [C2RustUnnamed {
            sp: ::core::ptr::null::<::core::ffi::c_char>(),
            ep: ::core::ptr::null::<::core::ffi::c_char>(),
        }; 16],
    };
    let mut i: ::core::ffi::c_int = 0;
    if sub.is_null() {
        sub = &raw mut scratch;
    }
    (*sub).nsub = (*prog).nsub;
    i = 0 as ::core::ffi::c_int;
    while i < REG_MAXSUB {
        (*sub).sub[i as usize].ep = ::core::ptr::null::<::core::ffi::c_char>();
        (*sub).sub[i as usize].sp = (*sub).sub[i as usize].ep;
        i += 1;
    }
    return match_0(
        (*prog).start,
        sp,
        sp,
        (*prog).flags | eflags,
        sub,
        0 as ::core::ffi::c_int,
    );
}
