extern "C" {
    pub type _IO_wide_data;
    pub type _IO_codecvt;
    pub type _IO_marker;
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
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn ceil(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
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
pub type __compar_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct diy_fp_t {
    pub f: uint64_t,
    pub e: ::core::ffi::c_int,
}
pub type uint64_t = __uint64_t;
pub type uint32_t = __uint32_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<
    ::core::ffi::c_void,
>();
pub const _IO_EOF_SEEN: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const _IO_ERR_SEEN: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
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
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn js_fmtexp(
    mut p: *mut ::core::ffi::c_char,
    mut e: ::core::ffi::c_int,
) {
    let mut se: [::core::ffi::c_char; 9] = [0; 9];
    let mut i: ::core::ffi::c_int = 0;
    let fresh6 = p;
    p = p.offset(1);
    *fresh6 = 'e' as i32 as ::core::ffi::c_char;
    if e < 0 as ::core::ffi::c_int {
        let fresh7 = p;
        p = p.offset(1);
        *fresh7 = '-' as i32 as ::core::ffi::c_char;
        e = -e;
    } else {
        let fresh8 = p;
        p = p.offset(1);
        *fresh8 = '+' as i32 as ::core::ffi::c_char;
    }
    i = 0 as ::core::ffi::c_int;
    while e != 0 {
        let fresh9 = i;
        i = i + 1;
        se[fresh9 as usize] = (e % 10 as ::core::ffi::c_int + '0' as i32)
            as ::core::ffi::c_char;
        e /= 10 as ::core::ffi::c_int;
    }
    while i < 1 as ::core::ffi::c_int {
        let fresh10 = i;
        i = i + 1;
        se[fresh10 as usize] = '0' as i32 as ::core::ffi::c_char;
    }
    while i > 0 as ::core::ffi::c_int {
        i -= 1;
        let fresh11 = p;
        p = p.offset(1);
        *fresh11 = se[i as usize];
    }
    let fresh12 = p;
    p = p.offset(1);
    *fresh12 = '\0' as i32 as ::core::ffi::c_char;
}
pub const DIY_SIGNIFICAND_SIZE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const D_1_LOG2_10: ::core::ffi::c_double = 0.30102999566398114f64;
static mut powers_ten: [uint64_t; 687] = [
    0xbf29dcaba82fdeae as ::core::ffi::c_ulong,
    0xeef453d6923bd65a as ::core::ffi::c_ulong,
    0x9558b4661b6565f8 as ::core::ffi::c_ulong,
    0xbaaee17fa23ebf76 as ::core::ffi::c_ulong,
    0xe95a99df8ace6f54 as ::core::ffi::c_ulong,
    0x91d8a02bb6c10594 as ::core::ffi::c_ulong,
    0xb64ec836a47146fa as ::core::ffi::c_ulong,
    0xe3e27a444d8d98b8 as ::core::ffi::c_ulong,
    0x8e6d8c6ab0787f73 as ::core::ffi::c_ulong,
    0xb208ef855c969f50 as ::core::ffi::c_ulong,
    0xde8b2b66b3bc4724 as ::core::ffi::c_ulong,
    0x8b16fb203055ac76 as ::core::ffi::c_ulong,
    0xaddcb9e83c6b1794 as ::core::ffi::c_ulong,
    0xd953e8624b85dd79 as ::core::ffi::c_ulong,
    0x87d4713d6f33aa6c as ::core::ffi::c_ulong,
    0xa9c98d8ccb009506 as ::core::ffi::c_ulong,
    0xd43bf0effdc0ba48 as ::core::ffi::c_ulong,
    0x84a57695fe98746d as ::core::ffi::c_ulong,
    0xa5ced43b7e3e9188 as ::core::ffi::c_ulong,
    0xcf42894a5dce35ea as ::core::ffi::c_ulong,
    0x818995ce7aa0e1b2 as ::core::ffi::c_ulong,
    0xa1ebfb4219491a1f as ::core::ffi::c_ulong,
    0xca66fa129f9b60a7 as ::core::ffi::c_ulong,
    0xfd00b897478238d1 as ::core::ffi::c_ulong,
    0x9e20735e8cb16382 as ::core::ffi::c_ulong,
    0xc5a890362fddbc63 as ::core::ffi::c_ulong,
    0xf712b443bbd52b7c as ::core::ffi::c_ulong,
    0x9a6bb0aa55653b2d as ::core::ffi::c_ulong,
    0xc1069cd4eabe89f9 as ::core::ffi::c_ulong,
    0xf148440a256e2c77 as ::core::ffi::c_ulong,
    0x96cd2a865764dbca as ::core::ffi::c_ulong,
    0xbc807527ed3e12bd as ::core::ffi::c_ulong,
    0xeba09271e88d976c as ::core::ffi::c_ulong,
    0x93445b8731587ea3 as ::core::ffi::c_ulong,
    0xb8157268fdae9e4c as ::core::ffi::c_ulong,
    0xe61acf033d1a45df as ::core::ffi::c_ulong,
    0x8fd0c16206306bac as ::core::ffi::c_ulong,
    0xb3c4f1ba87bc8697 as ::core::ffi::c_ulong,
    0xe0b62e2929aba83c as ::core::ffi::c_ulong,
    0x8c71dcd9ba0b4926 as ::core::ffi::c_ulong,
    0xaf8e5410288e1b6f as ::core::ffi::c_ulong,
    0xdb71e91432b1a24b as ::core::ffi::c_ulong,
    0x892731ac9faf056f as ::core::ffi::c_ulong,
    0xab70fe17c79ac6ca as ::core::ffi::c_ulong,
    0xd64d3d9db981787d as ::core::ffi::c_ulong,
    0x85f0468293f0eb4e as ::core::ffi::c_ulong,
    0xa76c582338ed2622 as ::core::ffi::c_ulong,
    0xd1476e2c07286faa as ::core::ffi::c_ulong,
    0x82cca4db847945ca as ::core::ffi::c_ulong,
    0xa37fce126597973d as ::core::ffi::c_ulong,
    0xcc5fc196fefd7d0c as ::core::ffi::c_ulong,
    0xff77b1fcbebcdc4f as ::core::ffi::c_ulong,
    0x9faacf3df73609b1 as ::core::ffi::c_ulong,
    0xc795830d75038c1e as ::core::ffi::c_ulong,
    0xf97ae3d0d2446f25 as ::core::ffi::c_ulong,
    0x9becce62836ac577 as ::core::ffi::c_ulong,
    0xc2e801fb244576d5 as ::core::ffi::c_ulong,
    0xf3a20279ed56d48a as ::core::ffi::c_ulong,
    0x9845418c345644d7 as ::core::ffi::c_ulong,
    0xbe5691ef416bd60c as ::core::ffi::c_ulong,
    0xedec366b11c6cb8f as ::core::ffi::c_ulong,
    0x94b3a202eb1c3f39 as ::core::ffi::c_ulong,
    0xb9e08a83a5e34f08 as ::core::ffi::c_ulong,
    0xe858ad248f5c22ca as ::core::ffi::c_ulong,
    0x91376c36d99995be as ::core::ffi::c_ulong,
    0xb58547448ffffb2e as ::core::ffi::c_ulong,
    0xe2e69915b3fff9f9 as ::core::ffi::c_ulong,
    0x8dd01fad907ffc3c as ::core::ffi::c_ulong,
    0xb1442798f49ffb4b as ::core::ffi::c_ulong,
    0xdd95317f31c7fa1d as ::core::ffi::c_ulong,
    0x8a7d3eef7f1cfc52 as ::core::ffi::c_ulong,
    0xad1c8eab5ee43b67 as ::core::ffi::c_ulong,
    0xd863b256369d4a41 as ::core::ffi::c_ulong,
    0x873e4f75e2224e68 as ::core::ffi::c_ulong,
    0xa90de3535aaae202 as ::core::ffi::c_ulong,
    0xd3515c2831559a83 as ::core::ffi::c_ulong,
    0x8412d9991ed58092 as ::core::ffi::c_ulong,
    0xa5178fff668ae0b6 as ::core::ffi::c_ulong,
    0xce5d73ff402d98e4 as ::core::ffi::c_ulong,
    0x80fa687f881c7f8e as ::core::ffi::c_ulong,
    0xa139029f6a239f72 as ::core::ffi::c_ulong,
    0xc987434744ac874f as ::core::ffi::c_ulong,
    0xfbe9141915d7a922 as ::core::ffi::c_ulong,
    0x9d71ac8fada6c9b5 as ::core::ffi::c_ulong,
    0xc4ce17b399107c23 as ::core::ffi::c_ulong,
    0xf6019da07f549b2b as ::core::ffi::c_ulong,
    0x99c102844f94e0fb as ::core::ffi::c_ulong,
    0xc0314325637a193a as ::core::ffi::c_ulong,
    0xf03d93eebc589f88 as ::core::ffi::c_ulong,
    0x96267c7535b763b5 as ::core::ffi::c_ulong,
    0xbbb01b9283253ca3 as ::core::ffi::c_ulong,
    0xea9c227723ee8bcb as ::core::ffi::c_ulong,
    0x92a1958a7675175f as ::core::ffi::c_ulong,
    0xb749faed14125d37 as ::core::ffi::c_ulong,
    0xe51c79a85916f485 as ::core::ffi::c_ulong,
    0x8f31cc0937ae58d3 as ::core::ffi::c_ulong,
    0xb2fe3f0b8599ef08 as ::core::ffi::c_ulong,
    0xdfbdcece67006ac9 as ::core::ffi::c_ulong,
    0x8bd6a141006042be as ::core::ffi::c_ulong,
    0xaecc49914078536d as ::core::ffi::c_ulong,
    0xda7f5bf590966849 as ::core::ffi::c_ulong,
    0x888f99797a5e012d as ::core::ffi::c_ulong,
    0xaab37fd7d8f58179 as ::core::ffi::c_ulong,
    0xd5605fcdcf32e1d7 as ::core::ffi::c_ulong,
    0x855c3be0a17fcd26 as ::core::ffi::c_ulong,
    0xa6b34ad8c9dfc070 as ::core::ffi::c_ulong,
    0xd0601d8efc57b08c as ::core::ffi::c_ulong,
    0x823c12795db6ce57 as ::core::ffi::c_ulong,
    0xa2cb1717b52481ed as ::core::ffi::c_ulong,
    0xcb7ddcdda26da269 as ::core::ffi::c_ulong,
    0xfe5d54150b090b03 as ::core::ffi::c_ulong,
    0x9efa548d26e5a6e2 as ::core::ffi::c_ulong,
    0xc6b8e9b0709f109a as ::core::ffi::c_ulong,
    0xf867241c8cc6d4c1 as ::core::ffi::c_ulong,
    0x9b407691d7fc44f8 as ::core::ffi::c_ulong,
    0xc21094364dfb5637 as ::core::ffi::c_ulong,
    0xf294b943e17a2bc4 as ::core::ffi::c_ulong,
    0x979cf3ca6cec5b5b as ::core::ffi::c_ulong,
    0xbd8430bd08277231 as ::core::ffi::c_ulong,
    0xece53cec4a314ebe as ::core::ffi::c_ulong,
    0x940f4613ae5ed137 as ::core::ffi::c_ulong,
    0xb913179899f68584 as ::core::ffi::c_ulong,
    0xe757dd7ec07426e5 as ::core::ffi::c_ulong,
    0x9096ea6f3848984f as ::core::ffi::c_ulong,
    0xb4bca50b065abe63 as ::core::ffi::c_ulong,
    0xe1ebce4dc7f16dfc as ::core::ffi::c_ulong,
    0x8d3360f09cf6e4bd as ::core::ffi::c_ulong,
    0xb080392cc4349ded as ::core::ffi::c_ulong,
    0xdca04777f541c568 as ::core::ffi::c_ulong,
    0x89e42caaf9491b61 as ::core::ffi::c_ulong,
    0xac5d37d5b79b6239 as ::core::ffi::c_ulong,
    0xd77485cb25823ac7 as ::core::ffi::c_ulong,
    0x86a8d39ef77164bd as ::core::ffi::c_ulong,
    0xa8530886b54dbdec as ::core::ffi::c_ulong,
    0xd267caa862a12d67 as ::core::ffi::c_ulong,
    0x8380dea93da4bc60 as ::core::ffi::c_ulong,
    0xa46116538d0deb78 as ::core::ffi::c_ulong,
    0xcd795be870516656 as ::core::ffi::c_ulong,
    0x806bd9714632dff6 as ::core::ffi::c_ulong,
    0xa086cfcd97bf97f4 as ::core::ffi::c_ulong,
    0xc8a883c0fdaf7df0 as ::core::ffi::c_ulong,
    0xfad2a4b13d1b5d6c as ::core::ffi::c_ulong,
    0x9cc3a6eec6311a64 as ::core::ffi::c_ulong,
    0xc3f490aa77bd60fd as ::core::ffi::c_ulong,
    0xf4f1b4d515acb93c as ::core::ffi::c_ulong,
    0x991711052d8bf3c5 as ::core::ffi::c_ulong,
    0xbf5cd54678eef0b7 as ::core::ffi::c_ulong,
    0xef340a98172aace5 as ::core::ffi::c_ulong,
    0x9580869f0e7aac0f as ::core::ffi::c_ulong,
    0xbae0a846d2195713 as ::core::ffi::c_ulong,
    0xe998d258869facd7 as ::core::ffi::c_ulong,
    0x91ff83775423cc06 as ::core::ffi::c_ulong,
    0xb67f6455292cbf08 as ::core::ffi::c_ulong,
    0xe41f3d6a7377eeca as ::core::ffi::c_ulong,
    0x8e938662882af53e as ::core::ffi::c_ulong,
    0xb23867fb2a35b28e as ::core::ffi::c_ulong,
    0xdec681f9f4c31f31 as ::core::ffi::c_ulong,
    0x8b3c113c38f9f37f as ::core::ffi::c_ulong,
    0xae0b158b4738705f as ::core::ffi::c_ulong,
    0xd98ddaee19068c76 as ::core::ffi::c_ulong,
    0x87f8a8d4cfa417ca as ::core::ffi::c_ulong,
    0xa9f6d30a038d1dbc as ::core::ffi::c_ulong,
    0xd47487cc8470652b as ::core::ffi::c_ulong,
    0x84c8d4dfd2c63f3b as ::core::ffi::c_ulong,
    0xa5fb0a17c777cf0a as ::core::ffi::c_ulong,
    0xcf79cc9db955c2cc as ::core::ffi::c_ulong,
    0x81ac1fe293d599c0 as ::core::ffi::c_ulong,
    0xa21727db38cb0030 as ::core::ffi::c_ulong,
    0xca9cf1d206fdc03c as ::core::ffi::c_ulong,
    0xfd442e4688bd304b as ::core::ffi::c_ulong,
    0x9e4a9cec15763e2f as ::core::ffi::c_ulong,
    0xc5dd44271ad3cdba as ::core::ffi::c_ulong,
    0xf7549530e188c129 as ::core::ffi::c_ulong,
    0x9a94dd3e8cf578ba as ::core::ffi::c_ulong,
    0xc13a148e3032d6e8 as ::core::ffi::c_ulong,
    0xf18899b1bc3f8ca2 as ::core::ffi::c_ulong,
    0x96f5600f15a7b7e5 as ::core::ffi::c_ulong,
    0xbcb2b812db11a5de as ::core::ffi::c_ulong,
    0xebdf661791d60f56 as ::core::ffi::c_ulong,
    0x936b9fcebb25c996 as ::core::ffi::c_ulong,
    0xb84687c269ef3bfb as ::core::ffi::c_ulong,
    0xe65829b3046b0afa as ::core::ffi::c_ulong,
    0x8ff71a0fe2c2e6dc as ::core::ffi::c_ulong,
    0xb3f4e093db73a093 as ::core::ffi::c_ulong,
    0xe0f218b8d25088b8 as ::core::ffi::c_ulong,
    0x8c974f7383725573 as ::core::ffi::c_ulong,
    0xafbd2350644eead0 as ::core::ffi::c_ulong,
    0xdbac6c247d62a584 as ::core::ffi::c_ulong,
    0x894bc396ce5da772 as ::core::ffi::c_ulong,
    0xab9eb47c81f5114f as ::core::ffi::c_ulong,
    0xd686619ba27255a3 as ::core::ffi::c_ulong,
    0x8613fd0145877586 as ::core::ffi::c_ulong,
    0xa798fc4196e952e7 as ::core::ffi::c_ulong,
    0xd17f3b51fca3a7a1 as ::core::ffi::c_ulong,
    0x82ef85133de648c5 as ::core::ffi::c_ulong,
    0xa3ab66580d5fdaf6 as ::core::ffi::c_ulong,
    0xcc963fee10b7d1b3 as ::core::ffi::c_ulong,
    0xffbbcfe994e5c620 as ::core::ffi::c_ulong,
    0x9fd561f1fd0f9bd4 as ::core::ffi::c_ulong,
    0xc7caba6e7c5382c9 as ::core::ffi::c_ulong,
    0xf9bd690a1b68637b as ::core::ffi::c_ulong,
    0x9c1661a651213e2d as ::core::ffi::c_ulong,
    0xc31bfa0fe5698db8 as ::core::ffi::c_ulong,
    0xf3e2f893dec3f126 as ::core::ffi::c_ulong,
    0x986ddb5c6b3a76b8 as ::core::ffi::c_ulong,
    0xbe89523386091466 as ::core::ffi::c_ulong,
    0xee2ba6c0678b597f as ::core::ffi::c_ulong,
    0x94db483840b717f0 as ::core::ffi::c_ulong,
    0xba121a4650e4ddec as ::core::ffi::c_ulong,
    0xe896a0d7e51e1566 as ::core::ffi::c_ulong,
    0x915e2486ef32cd60 as ::core::ffi::c_ulong,
    0xb5b5ada8aaff80b8 as ::core::ffi::c_ulong,
    0xe3231912d5bf60e6 as ::core::ffi::c_ulong,
    0x8df5efabc5979c90 as ::core::ffi::c_ulong,
    0xb1736b96b6fd83b4 as ::core::ffi::c_ulong,
    0xddd0467c64bce4a1 as ::core::ffi::c_ulong,
    0x8aa22c0dbef60ee4 as ::core::ffi::c_ulong,
    0xad4ab7112eb3929e as ::core::ffi::c_ulong,
    0xd89d64d57a607745 as ::core::ffi::c_ulong,
    0x87625f056c7c4a8b as ::core::ffi::c_ulong,
    0xa93af6c6c79b5d2e as ::core::ffi::c_ulong,
    0xd389b47879823479 as ::core::ffi::c_ulong,
    0x843610cb4bf160cc as ::core::ffi::c_ulong,
    0xa54394fe1eedb8ff as ::core::ffi::c_ulong,
    0xce947a3da6a9273e as ::core::ffi::c_ulong,
    0x811ccc668829b887 as ::core::ffi::c_ulong,
    0xa163ff802a3426a9 as ::core::ffi::c_ulong,
    0xc9bcff6034c13053 as ::core::ffi::c_ulong,
    0xfc2c3f3841f17c68 as ::core::ffi::c_ulong,
    0x9d9ba7832936edc1 as ::core::ffi::c_ulong,
    0xc5029163f384a931 as ::core::ffi::c_ulong,
    0xf64335bcf065d37d as ::core::ffi::c_ulong,
    0x99ea0196163fa42e as ::core::ffi::c_ulong,
    0xc06481fb9bcf8d3a as ::core::ffi::c_ulong,
    0xf07da27a82c37088 as ::core::ffi::c_ulong,
    0x964e858c91ba2655 as ::core::ffi::c_ulong,
    0xbbe226efb628afeb as ::core::ffi::c_ulong,
    0xeadab0aba3b2dbe5 as ::core::ffi::c_ulong,
    0x92c8ae6b464fc96f as ::core::ffi::c_ulong,
    0xb77ada0617e3bbcb as ::core::ffi::c_ulong,
    0xe55990879ddcaabe as ::core::ffi::c_ulong,
    0x8f57fa54c2a9eab7 as ::core::ffi::c_ulong,
    0xb32df8e9f3546564 as ::core::ffi::c_ulong,
    0xdff9772470297ebd as ::core::ffi::c_ulong,
    0x8bfbea76c619ef36 as ::core::ffi::c_ulong,
    0xaefae51477a06b04 as ::core::ffi::c_ulong,
    0xdab99e59958885c5 as ::core::ffi::c_ulong,
    0x88b402f7fd75539b as ::core::ffi::c_ulong,
    0xaae103b5fcd2a882 as ::core::ffi::c_ulong,
    0xd59944a37c0752a2 as ::core::ffi::c_ulong,
    0x857fcae62d8493a5 as ::core::ffi::c_ulong,
    0xa6dfbd9fb8e5b88f as ::core::ffi::c_ulong,
    0xd097ad07a71f26b2 as ::core::ffi::c_ulong,
    0x825ecc24c8737830 as ::core::ffi::c_ulong,
    0xa2f67f2dfa90563b as ::core::ffi::c_ulong,
    0xcbb41ef979346bca as ::core::ffi::c_ulong,
    0xfea126b7d78186bd as ::core::ffi::c_ulong,
    0x9f24b832e6b0f436 as ::core::ffi::c_ulong,
    0xc6ede63fa05d3144 as ::core::ffi::c_ulong,
    0xf8a95fcf88747d94 as ::core::ffi::c_ulong,
    0x9b69dbe1b548ce7d as ::core::ffi::c_ulong,
    0xc24452da229b021c as ::core::ffi::c_ulong,
    0xf2d56790ab41c2a3 as ::core::ffi::c_ulong,
    0x97c560ba6b0919a6 as ::core::ffi::c_ulong,
    0xbdb6b8e905cb600f as ::core::ffi::c_ulong,
    0xed246723473e3813 as ::core::ffi::c_ulong,
    0x9436c0760c86e30c as ::core::ffi::c_ulong,
    0xb94470938fa89bcf as ::core::ffi::c_ulong,
    0xe7958cb87392c2c3 as ::core::ffi::c_ulong,
    0x90bd77f3483bb9ba as ::core::ffi::c_ulong,
    0xb4ecd5f01a4aa828 as ::core::ffi::c_ulong,
    0xe2280b6c20dd5232 as ::core::ffi::c_ulong,
    0x8d590723948a535f as ::core::ffi::c_ulong,
    0xb0af48ec79ace837 as ::core::ffi::c_ulong,
    0xdcdb1b2798182245 as ::core::ffi::c_ulong,
    0x8a08f0f8bf0f156b as ::core::ffi::c_ulong,
    0xac8b2d36eed2dac6 as ::core::ffi::c_ulong,
    0xd7adf884aa879177 as ::core::ffi::c_ulong,
    0x86ccbb52ea94baeb as ::core::ffi::c_ulong,
    0xa87fea27a539e9a5 as ::core::ffi::c_ulong,
    0xd29fe4b18e88640f as ::core::ffi::c_ulong,
    0x83a3eeeef9153e89 as ::core::ffi::c_ulong,
    0xa48ceaaab75a8e2b as ::core::ffi::c_ulong,
    0xcdb02555653131b6 as ::core::ffi::c_ulong,
    0x808e17555f3ebf12 as ::core::ffi::c_ulong,
    0xa0b19d2ab70e6ed6 as ::core::ffi::c_ulong,
    0xc8de047564d20a8c as ::core::ffi::c_ulong,
    0xfb158592be068d2f as ::core::ffi::c_ulong,
    0x9ced737bb6c4183d as ::core::ffi::c_ulong,
    0xc428d05aa4751e4d as ::core::ffi::c_ulong,
    0xf53304714d9265e0 as ::core::ffi::c_ulong,
    0x993fe2c6d07b7fac as ::core::ffi::c_ulong,
    0xbf8fdb78849a5f97 as ::core::ffi::c_ulong,
    0xef73d256a5c0f77d as ::core::ffi::c_ulong,
    0x95a8637627989aae as ::core::ffi::c_ulong,
    0xbb127c53b17ec159 as ::core::ffi::c_ulong,
    0xe9d71b689dde71b0 as ::core::ffi::c_ulong,
    0x9226712162ab070e as ::core::ffi::c_ulong,
    0xb6b00d69bb55c8d1 as ::core::ffi::c_ulong,
    0xe45c10c42a2b3b06 as ::core::ffi::c_ulong,
    0x8eb98a7a9a5b04e3 as ::core::ffi::c_ulong,
    0xb267ed1940f1c61c as ::core::ffi::c_ulong,
    0xdf01e85f912e37a3 as ::core::ffi::c_ulong,
    0x8b61313bbabce2c6 as ::core::ffi::c_ulong,
    0xae397d8aa96c1b78 as ::core::ffi::c_ulong,
    0xd9c7dced53c72256 as ::core::ffi::c_ulong,
    0x881cea14545c7575 as ::core::ffi::c_ulong,
    0xaa242499697392d3 as ::core::ffi::c_ulong,
    0xd4ad2dbfc3d07788 as ::core::ffi::c_ulong,
    0x84ec3c97da624ab5 as ::core::ffi::c_ulong,
    0xa6274bbdd0fadd62 as ::core::ffi::c_ulong,
    0xcfb11ead453994ba as ::core::ffi::c_ulong,
    0x81ceb32c4b43fcf5 as ::core::ffi::c_ulong,
    0xa2425ff75e14fc32 as ::core::ffi::c_ulong,
    0xcad2f7f5359a3b3e as ::core::ffi::c_ulong,
    0xfd87b5f28300ca0e as ::core::ffi::c_ulong,
    0x9e74d1b791e07e48 as ::core::ffi::c_ulong,
    0xc612062576589ddb as ::core::ffi::c_ulong,
    0xf79687aed3eec551 as ::core::ffi::c_ulong,
    0x9abe14cd44753b53 as ::core::ffi::c_ulong,
    0xc16d9a0095928a27 as ::core::ffi::c_ulong,
    0xf1c90080baf72cb1 as ::core::ffi::c_ulong,
    0x971da05074da7bef as ::core::ffi::c_ulong,
    0xbce5086492111aeb as ::core::ffi::c_ulong,
    0xec1e4a7db69561a5 as ::core::ffi::c_ulong,
    0x9392ee8e921d5d07 as ::core::ffi::c_ulong,
    0xb877aa3236a4b449 as ::core::ffi::c_ulong,
    0xe69594bec44de15b as ::core::ffi::c_ulong,
    0x901d7cf73ab0acd9 as ::core::ffi::c_ulong,
    0xb424dc35095cd80f as ::core::ffi::c_ulong,
    0xe12e13424bb40e13 as ::core::ffi::c_ulong,
    0x8cbccc096f5088cc as ::core::ffi::c_ulong,
    0xafebff0bcb24aaff as ::core::ffi::c_ulong,
    0xdbe6fecebdedd5bf as ::core::ffi::c_ulong,
    0x89705f4136b4a597 as ::core::ffi::c_ulong,
    0xabcc77118461cefd as ::core::ffi::c_ulong,
    0xd6bf94d5e57a42bc as ::core::ffi::c_ulong,
    0x8637bd05af6c69b6 as ::core::ffi::c_ulong,
    0xa7c5ac471b478423 as ::core::ffi::c_ulong,
    0xd1b71758e219652c as ::core::ffi::c_ulong,
    0x83126e978d4fdf3b as ::core::ffi::c_ulong,
    0xa3d70a3d70a3d70a as ::core::ffi::c_ulong,
    0xcccccccccccccccd as ::core::ffi::c_ulong,
    0x8000000000000000 as ::core::ffi::c_ulong,
    0xa000000000000000 as ::core::ffi::c_ulong,
    0xc800000000000000 as ::core::ffi::c_ulong,
    0xfa00000000000000 as ::core::ffi::c_ulong,
    0x9c40000000000000 as ::core::ffi::c_ulong,
    0xc350000000000000 as ::core::ffi::c_ulong,
    0xf424000000000000 as ::core::ffi::c_ulong,
    0x9896800000000000 as ::core::ffi::c_ulong,
    0xbebc200000000000 as ::core::ffi::c_ulong,
    0xee6b280000000000 as ::core::ffi::c_ulong,
    0x9502f90000000000 as ::core::ffi::c_ulong,
    0xba43b74000000000 as ::core::ffi::c_ulong,
    0xe8d4a51000000000 as ::core::ffi::c_ulong,
    0x9184e72a00000000 as ::core::ffi::c_ulong,
    0xb5e620f480000000 as ::core::ffi::c_ulong,
    0xe35fa931a0000000 as ::core::ffi::c_ulong,
    0x8e1bc9bf04000000 as ::core::ffi::c_ulong,
    0xb1a2bc2ec5000000 as ::core::ffi::c_ulong,
    0xde0b6b3a76400000 as ::core::ffi::c_ulong,
    0x8ac7230489e80000 as ::core::ffi::c_ulong,
    0xad78ebc5ac620000 as ::core::ffi::c_ulong,
    0xd8d726b7177a8000 as ::core::ffi::c_ulong,
    0x878678326eac9000 as ::core::ffi::c_ulong,
    0xa968163f0a57b400 as ::core::ffi::c_ulong,
    0xd3c21bcecceda100 as ::core::ffi::c_ulong,
    0x84595161401484a0 as ::core::ffi::c_ulong,
    0xa56fa5b99019a5c8 as ::core::ffi::c_ulong,
    0xcecb8f27f4200f3a as ::core::ffi::c_ulong,
    0x813f3978f8940984 as ::core::ffi::c_ulong,
    0xa18f07d736b90be5 as ::core::ffi::c_ulong,
    0xc9f2c9cd04674edf as ::core::ffi::c_ulong,
    0xfc6f7c4045812296 as ::core::ffi::c_ulong,
    0x9dc5ada82b70b59e as ::core::ffi::c_ulong,
    0xc5371912364ce305 as ::core::ffi::c_ulong,
    0xf684df56c3e01bc7 as ::core::ffi::c_ulong,
    0x9a130b963a6c115c as ::core::ffi::c_ulong,
    0xc097ce7bc90715b3 as ::core::ffi::c_ulong,
    0xf0bdc21abb48db20 as ::core::ffi::c_ulong,
    0x96769950b50d88f4 as ::core::ffi::c_ulong,
    0xbc143fa4e250eb31 as ::core::ffi::c_ulong,
    0xeb194f8e1ae525fd as ::core::ffi::c_ulong,
    0x92efd1b8d0cf37be as ::core::ffi::c_ulong,
    0xb7abc627050305ae as ::core::ffi::c_ulong,
    0xe596b7b0c643c719 as ::core::ffi::c_ulong,
    0x8f7e32ce7bea5c70 as ::core::ffi::c_ulong,
    0xb35dbf821ae4f38c as ::core::ffi::c_ulong,
    0xe0352f62a19e306f as ::core::ffi::c_ulong,
    0x8c213d9da502de45 as ::core::ffi::c_ulong,
    0xaf298d050e4395d7 as ::core::ffi::c_ulong,
    0xdaf3f04651d47b4c as ::core::ffi::c_ulong,
    0x88d8762bf324cd10 as ::core::ffi::c_ulong,
    0xab0e93b6efee0054 as ::core::ffi::c_ulong,
    0xd5d238a4abe98068 as ::core::ffi::c_ulong,
    0x85a36366eb71f041 as ::core::ffi::c_ulong,
    0xa70c3c40a64e6c52 as ::core::ffi::c_ulong,
    0xd0cf4b50cfe20766 as ::core::ffi::c_ulong,
    0x82818f1281ed44a0 as ::core::ffi::c_ulong,
    0xa321f2d7226895c8 as ::core::ffi::c_ulong,
    0xcbea6f8ceb02bb3a as ::core::ffi::c_ulong,
    0xfee50b7025c36a08 as ::core::ffi::c_ulong,
    0x9f4f2726179a2245 as ::core::ffi::c_ulong,
    0xc722f0ef9d80aad6 as ::core::ffi::c_ulong,
    0xf8ebad2b84e0d58c as ::core::ffi::c_ulong,
    0x9b934c3b330c8577 as ::core::ffi::c_ulong,
    0xc2781f49ffcfa6d5 as ::core::ffi::c_ulong,
    0xf316271c7fc3908b as ::core::ffi::c_ulong,
    0x97edd871cfda3a57 as ::core::ffi::c_ulong,
    0xbde94e8e43d0c8ec as ::core::ffi::c_ulong,
    0xed63a231d4c4fb27 as ::core::ffi::c_ulong,
    0x945e455f24fb1cf9 as ::core::ffi::c_ulong,
    0xb975d6b6ee39e437 as ::core::ffi::c_ulong,
    0xe7d34c64a9c85d44 as ::core::ffi::c_ulong,
    0x90e40fbeea1d3a4b as ::core::ffi::c_ulong,
    0xb51d13aea4a488dd as ::core::ffi::c_ulong,
    0xe264589a4dcdab15 as ::core::ffi::c_ulong,
    0x8d7eb76070a08aed as ::core::ffi::c_ulong,
    0xb0de65388cc8ada8 as ::core::ffi::c_ulong,
    0xdd15fe86affad912 as ::core::ffi::c_ulong,
    0x8a2dbf142dfcc7ab as ::core::ffi::c_ulong,
    0xacb92ed9397bf996 as ::core::ffi::c_ulong,
    0xd7e77a8f87daf7fc as ::core::ffi::c_ulong,
    0x86f0ac99b4e8dafd as ::core::ffi::c_ulong,
    0xa8acd7c0222311bd as ::core::ffi::c_ulong,
    0xd2d80db02aabd62c as ::core::ffi::c_ulong,
    0x83c7088e1aab65db as ::core::ffi::c_ulong,
    0xa4b8cab1a1563f52 as ::core::ffi::c_ulong,
    0xcde6fd5e09abcf27 as ::core::ffi::c_ulong,
    0x80b05e5ac60b6178 as ::core::ffi::c_ulong,
    0xa0dc75f1778e39d6 as ::core::ffi::c_ulong,
    0xc913936dd571c84c as ::core::ffi::c_ulong,
    0xfb5878494ace3a5f as ::core::ffi::c_ulong,
    0x9d174b2dcec0e47b as ::core::ffi::c_ulong,
    0xc45d1df942711d9a as ::core::ffi::c_ulong,
    0xf5746577930d6501 as ::core::ffi::c_ulong,
    0x9968bf6abbe85f20 as ::core::ffi::c_ulong,
    0xbfc2ef456ae276e9 as ::core::ffi::c_ulong,
    0xefb3ab16c59b14a3 as ::core::ffi::c_ulong,
    0x95d04aee3b80ece6 as ::core::ffi::c_ulong,
    0xbb445da9ca61281f as ::core::ffi::c_ulong,
    0xea1575143cf97227 as ::core::ffi::c_ulong,
    0x924d692ca61be758 as ::core::ffi::c_ulong,
    0xb6e0c377cfa2e12e as ::core::ffi::c_ulong,
    0xe498f455c38b997a as ::core::ffi::c_ulong,
    0x8edf98b59a373fec as ::core::ffi::c_ulong,
    0xb2977ee300c50fe7 as ::core::ffi::c_ulong,
    0xdf3d5e9bc0f653e1 as ::core::ffi::c_ulong,
    0x8b865b215899f46d as ::core::ffi::c_ulong,
    0xae67f1e9aec07188 as ::core::ffi::c_ulong,
    0xda01ee641a708dea as ::core::ffi::c_ulong,
    0x884134fe908658b2 as ::core::ffi::c_ulong,
    0xaa51823e34a7eedf as ::core::ffi::c_ulong,
    0xd4e5e2cdc1d1ea96 as ::core::ffi::c_ulong,
    0x850fadc09923329e as ::core::ffi::c_ulong,
    0xa6539930bf6bff46 as ::core::ffi::c_ulong,
    0xcfe87f7cef46ff17 as ::core::ffi::c_ulong,
    0x81f14fae158c5f6e as ::core::ffi::c_ulong,
    0xa26da3999aef774a as ::core::ffi::c_ulong,
    0xcb090c8001ab551c as ::core::ffi::c_ulong,
    0xfdcb4fa002162a63 as ::core::ffi::c_ulong,
    0x9e9f11c4014dda7e as ::core::ffi::c_ulong,
    0xc646d63501a1511e as ::core::ffi::c_ulong,
    0xf7d88bc24209a565 as ::core::ffi::c_ulong,
    0x9ae757596946075f as ::core::ffi::c_ulong,
    0xc1a12d2fc3978937 as ::core::ffi::c_ulong,
    0xf209787bb47d6b85 as ::core::ffi::c_ulong,
    0x9745eb4d50ce6333 as ::core::ffi::c_ulong,
    0xbd176620a501fc00 as ::core::ffi::c_ulong,
    0xec5d3fa8ce427b00 as ::core::ffi::c_ulong,
    0x93ba47c980e98ce0 as ::core::ffi::c_ulong,
    0xb8a8d9bbe123f018 as ::core::ffi::c_ulong,
    0xe6d3102ad96cec1e as ::core::ffi::c_ulong,
    0x9043ea1ac7e41393 as ::core::ffi::c_ulong,
    0xb454e4a179dd1877 as ::core::ffi::c_ulong,
    0xe16a1dc9d8545e95 as ::core::ffi::c_ulong,
    0x8ce2529e2734bb1d as ::core::ffi::c_ulong,
    0xb01ae745b101e9e4 as ::core::ffi::c_ulong,
    0xdc21a1171d42645d as ::core::ffi::c_ulong,
    0x899504ae72497eba as ::core::ffi::c_ulong,
    0xabfa45da0edbde69 as ::core::ffi::c_ulong,
    0xd6f8d7509292d603 as ::core::ffi::c_ulong,
    0x865b86925b9bc5c2 as ::core::ffi::c_ulong,
    0xa7f26836f282b733 as ::core::ffi::c_ulong,
    0xd1ef0244af2364ff as ::core::ffi::c_ulong,
    0x8335616aed761f1f as ::core::ffi::c_ulong,
    0xa402b9c5a8d3a6e7 as ::core::ffi::c_ulong,
    0xcd036837130890a1 as ::core::ffi::c_ulong,
    0x802221226be55a65 as ::core::ffi::c_ulong,
    0xa02aa96b06deb0fe as ::core::ffi::c_ulong,
    0xc83553c5c8965d3d as ::core::ffi::c_ulong,
    0xfa42a8b73abbf48d as ::core::ffi::c_ulong,
    0x9c69a97284b578d8 as ::core::ffi::c_ulong,
    0xc38413cf25e2d70e as ::core::ffi::c_ulong,
    0xf46518c2ef5b8cd1 as ::core::ffi::c_ulong,
    0x98bf2f79d5993803 as ::core::ffi::c_ulong,
    0xbeeefb584aff8604 as ::core::ffi::c_ulong,
    0xeeaaba2e5dbf6785 as ::core::ffi::c_ulong,
    0x952ab45cfa97a0b3 as ::core::ffi::c_ulong,
    0xba756174393d88e0 as ::core::ffi::c_ulong,
    0xe912b9d1478ceb17 as ::core::ffi::c_ulong,
    0x91abb422ccb812ef as ::core::ffi::c_ulong,
    0xb616a12b7fe617aa as ::core::ffi::c_ulong,
    0xe39c49765fdf9d95 as ::core::ffi::c_ulong,
    0x8e41ade9fbebc27d as ::core::ffi::c_ulong,
    0xb1d219647ae6b31c as ::core::ffi::c_ulong,
    0xde469fbd99a05fe3 as ::core::ffi::c_ulong,
    0x8aec23d680043bee as ::core::ffi::c_ulong,
    0xada72ccc20054aea as ::core::ffi::c_ulong,
    0xd910f7ff28069da4 as ::core::ffi::c_ulong,
    0x87aa9aff79042287 as ::core::ffi::c_ulong,
    0xa99541bf57452b28 as ::core::ffi::c_ulong,
    0xd3fa922f2d1675f2 as ::core::ffi::c_ulong,
    0x847c9b5d7c2e09b7 as ::core::ffi::c_ulong,
    0xa59bc234db398c25 as ::core::ffi::c_ulong,
    0xcf02b2c21207ef2f as ::core::ffi::c_ulong,
    0x8161afb94b44f57d as ::core::ffi::c_ulong,
    0xa1ba1ba79e1632dc as ::core::ffi::c_ulong,
    0xca28a291859bbf93 as ::core::ffi::c_ulong,
    0xfcb2cb35e702af78 as ::core::ffi::c_ulong,
    0x9defbf01b061adab as ::core::ffi::c_ulong,
    0xc56baec21c7a1916 as ::core::ffi::c_ulong,
    0xf6c69a72a3989f5c as ::core::ffi::c_ulong,
    0x9a3c2087a63f6399 as ::core::ffi::c_ulong,
    0xc0cb28a98fcf3c80 as ::core::ffi::c_ulong,
    0xf0fdf2d3f3c30b9f as ::core::ffi::c_ulong,
    0x969eb7c47859e744 as ::core::ffi::c_ulong,
    0xbc4665b596706115 as ::core::ffi::c_ulong,
    0xeb57ff22fc0c795a as ::core::ffi::c_ulong,
    0x9316ff75dd87cbd8 as ::core::ffi::c_ulong,
    0xb7dcbf5354e9bece as ::core::ffi::c_ulong,
    0xe5d3ef282a242e82 as ::core::ffi::c_ulong,
    0x8fa475791a569d11 as ::core::ffi::c_ulong,
    0xb38d92d760ec4455 as ::core::ffi::c_ulong,
    0xe070f78d3927556b as ::core::ffi::c_ulong,
    0x8c469ab843b89563 as ::core::ffi::c_ulong,
    0xaf58416654a6babb as ::core::ffi::c_ulong,
    0xdb2e51bfe9d0696a as ::core::ffi::c_ulong,
    0x88fcf317f22241e2 as ::core::ffi::c_ulong,
    0xab3c2fddeeaad25b as ::core::ffi::c_ulong,
    0xd60b3bd56a5586f2 as ::core::ffi::c_ulong,
    0x85c7056562757457 as ::core::ffi::c_ulong,
    0xa738c6bebb12d16d as ::core::ffi::c_ulong,
    0xd106f86e69d785c8 as ::core::ffi::c_ulong,
    0x82a45b450226b39d as ::core::ffi::c_ulong,
    0xa34d721642b06084 as ::core::ffi::c_ulong,
    0xcc20ce9bd35c78a5 as ::core::ffi::c_ulong,
    0xff290242c83396ce as ::core::ffi::c_ulong,
    0x9f79a169bd203e41 as ::core::ffi::c_ulong,
    0xc75809c42c684dd1 as ::core::ffi::c_ulong,
    0xf92e0c3537826146 as ::core::ffi::c_ulong,
    0x9bbcc7a142b17ccc as ::core::ffi::c_ulong,
    0xc2abf989935ddbfe as ::core::ffi::c_ulong,
    0xf356f7ebf83552fe as ::core::ffi::c_ulong,
    0x98165af37b2153df as ::core::ffi::c_ulong,
    0xbe1bf1b059e9a8d6 as ::core::ffi::c_ulong,
    0xeda2ee1c7064130c as ::core::ffi::c_ulong,
    0x9485d4d1c63e8be8 as ::core::ffi::c_ulong,
    0xb9a74a0637ce2ee1 as ::core::ffi::c_ulong,
    0xe8111c87c5c1ba9a as ::core::ffi::c_ulong,
    0x910ab1d4db9914a0 as ::core::ffi::c_ulong,
    0xb54d5e4a127f59c8 as ::core::ffi::c_ulong,
    0xe2a0b5dc971f303a as ::core::ffi::c_ulong,
    0x8da471a9de737e24 as ::core::ffi::c_ulong,
    0xb10d8e1456105dad as ::core::ffi::c_ulong,
    0xdd50f1996b947519 as ::core::ffi::c_ulong,
    0x8a5296ffe33cc930 as ::core::ffi::c_ulong,
    0xace73cbfdc0bfb7b as ::core::ffi::c_ulong,
    0xd8210befd30efa5a as ::core::ffi::c_ulong,
    0x8714a775e3e95c78 as ::core::ffi::c_ulong,
    0xa8d9d1535ce3b396 as ::core::ffi::c_ulong,
    0xd31045a8341ca07c as ::core::ffi::c_ulong,
    0x83ea2b892091e44e as ::core::ffi::c_ulong,
    0xa4e4b66b68b65d61 as ::core::ffi::c_ulong,
    0xce1de40642e3f4b9 as ::core::ffi::c_ulong,
    0x80d2ae83e9ce78f4 as ::core::ffi::c_ulong,
    0xa1075a24e4421731 as ::core::ffi::c_ulong,
    0xc94930ae1d529cfd as ::core::ffi::c_ulong,
    0xfb9b7cd9a4a7443c as ::core::ffi::c_ulong,
    0x9d412e0806e88aa6 as ::core::ffi::c_ulong,
    0xc491798a08a2ad4f as ::core::ffi::c_ulong,
    0xf5b5d7ec8acb58a3 as ::core::ffi::c_ulong,
    0x9991a6f3d6bf1766 as ::core::ffi::c_ulong,
    0xbff610b0cc6edd3f as ::core::ffi::c_ulong,
    0xeff394dcff8a948f as ::core::ffi::c_ulong,
    0x95f83d0a1fb69cd9 as ::core::ffi::c_ulong,
    0xbb764c4ca7a44410 as ::core::ffi::c_ulong,
    0xea53df5fd18d5514 as ::core::ffi::c_ulong,
    0x92746b9be2f8552c as ::core::ffi::c_ulong,
    0xb7118682dbb66a77 as ::core::ffi::c_ulong,
    0xe4d5e82392a40515 as ::core::ffi::c_ulong,
    0x8f05b1163ba6832d as ::core::ffi::c_ulong,
    0xb2c71d5bca9023f8 as ::core::ffi::c_ulong,
    0xdf78e4b2bd342cf7 as ::core::ffi::c_ulong,
    0x8bab8eefb6409c1a as ::core::ffi::c_ulong,
    0xae9672aba3d0c321 as ::core::ffi::c_ulong,
    0xda3c0f568cc4f3e9 as ::core::ffi::c_ulong,
    0x8865899617fb1871 as ::core::ffi::c_ulong,
    0xaa7eebfb9df9de8e as ::core::ffi::c_ulong,
    0xd51ea6fa85785631 as ::core::ffi::c_ulong,
    0x8533285c936b35df as ::core::ffi::c_ulong,
    0xa67ff273b8460357 as ::core::ffi::c_ulong,
    0xd01fef10a657842c as ::core::ffi::c_ulong,
    0x8213f56a67f6b29c as ::core::ffi::c_ulong,
    0xa298f2c501f45f43 as ::core::ffi::c_ulong,
    0xcb3f2f7642717713 as ::core::ffi::c_ulong,
    0xfe0efb53d30dd4d8 as ::core::ffi::c_ulong,
    0x9ec95d1463e8a507 as ::core::ffi::c_ulong,
    0xc67bb4597ce2ce49 as ::core::ffi::c_ulong,
    0xf81aa16fdc1b81db as ::core::ffi::c_ulong,
    0x9b10a4e5e9913129 as ::core::ffi::c_ulong,
    0xc1d4ce1f63f57d73 as ::core::ffi::c_ulong,
    0xf24a01a73cf2dcd0 as ::core::ffi::c_ulong,
    0x976e41088617ca02 as ::core::ffi::c_ulong,
    0xbd49d14aa79dbc82 as ::core::ffi::c_ulong,
    0xec9c459d51852ba3 as ::core::ffi::c_ulong,
    0x93e1ab8252f33b46 as ::core::ffi::c_ulong,
    0xb8da1662e7b00a17 as ::core::ffi::c_ulong,
    0xe7109bfba19c0c9d as ::core::ffi::c_ulong,
    0x906a617d450187e2 as ::core::ffi::c_ulong,
    0xb484f9dc9641e9db as ::core::ffi::c_ulong,
    0xe1a63853bbd26451 as ::core::ffi::c_ulong,
    0x8d07e33455637eb3 as ::core::ffi::c_ulong,
    0xb049dc016abc5e60 as ::core::ffi::c_ulong,
    0xdc5c5301c56b75f7 as ::core::ffi::c_ulong,
    0x89b9b3e11b6329bb as ::core::ffi::c_ulong,
    0xac2820d9623bf429 as ::core::ffi::c_ulong,
    0xd732290fbacaf134 as ::core::ffi::c_ulong,
    0x867f59a9d4bed6c0 as ::core::ffi::c_ulong,
    0xa81f301449ee8c70 as ::core::ffi::c_ulong,
    0xd226fc195c6a2f8c as ::core::ffi::c_ulong,
    0x83585d8fd9c25db8 as ::core::ffi::c_ulong,
    0xa42e74f3d032f526 as ::core::ffi::c_ulong,
    0xcd3a1230c43fb26f as ::core::ffi::c_ulong,
    0x80444b5e7aa7cf85 as ::core::ffi::c_ulong,
    0xa0555e361951c367 as ::core::ffi::c_ulong,
    0xc86ab5c39fa63441 as ::core::ffi::c_ulong,
    0xfa856334878fc151 as ::core::ffi::c_ulong,
    0x9c935e00d4b9d8d2 as ::core::ffi::c_ulong,
    0xc3b8358109e84f07 as ::core::ffi::c_ulong,
    0xf4a642e14c6262c9 as ::core::ffi::c_ulong,
    0x98e7e9cccfbd7dbe as ::core::ffi::c_ulong,
    0xbf21e44003acdd2d as ::core::ffi::c_ulong,
    0xeeea5d5004981478 as ::core::ffi::c_ulong,
    0x95527a5202df0ccb as ::core::ffi::c_ulong,
    0xbaa718e68396cffe as ::core::ffi::c_ulong,
    0xe950df20247c83fd as ::core::ffi::c_ulong,
    0x91d28b7416cdd27e as ::core::ffi::c_ulong,
    0xb6472e511c81471e as ::core::ffi::c_ulong,
    0xe3d8f9e563a198e5 as ::core::ffi::c_ulong,
    0x8e679c2f5e44ff8f as ::core::ffi::c_ulong,
    0xb201833b35d63f73 as ::core::ffi::c_ulong,
    0xde81e40a034bcf50 as ::core::ffi::c_ulong,
    0x8b112e86420f6192 as ::core::ffi::c_ulong,
    0xadd57a27d29339f6 as ::core::ffi::c_ulong,
    0xd94ad8b1c7380874 as ::core::ffi::c_ulong,
    0x87cec76f1c830549 as ::core::ffi::c_ulong,
    0xa9c2794ae3a3c69b as ::core::ffi::c_ulong,
    0xd433179d9c8cb841 as ::core::ffi::c_ulong,
    0x849feec281d7f329 as ::core::ffi::c_ulong,
    0xa5c7ea73224deff3 as ::core::ffi::c_ulong,
    0xcf39e50feae16bf0 as ::core::ffi::c_ulong,
    0x81842f29f2cce376 as ::core::ffi::c_ulong,
    0xa1e53af46f801c53 as ::core::ffi::c_ulong,
    0xca5e89b18b602368 as ::core::ffi::c_ulong,
    0xfcf62c1dee382c42 as ::core::ffi::c_ulong,
    0x9e19db92b4e31ba9 as ::core::ffi::c_ulong,
    0xc5a05277621be294 as ::core::ffi::c_ulong,
    0xf70867153aa2db39 as ::core::ffi::c_ulong,
    0x9a65406d44a5c903 as ::core::ffi::c_ulong,
    0xc0fe908895cf3b44 as ::core::ffi::c_ulong,
    0xf13e34aabb430a15 as ::core::ffi::c_ulong,
    0x96c6e0eab509e64d as ::core::ffi::c_ulong,
    0xbc789925624c5fe1 as ::core::ffi::c_ulong,
    0xeb96bf6ebadf77d9 as ::core::ffi::c_ulong,
    0x933e37a534cbaae8 as ::core::ffi::c_ulong,
    0xb80dc58e81fe95a1 as ::core::ffi::c_ulong,
    0xe61136f2227e3b0a as ::core::ffi::c_ulong,
    0x8fcac257558ee4e6 as ::core::ffi::c_ulong,
    0xb3bd72ed2af29e20 as ::core::ffi::c_ulong,
    0xe0accfa875af45a8 as ::core::ffi::c_ulong,
    0x8c6c01c9498d8b89 as ::core::ffi::c_ulong,
    0xaf87023b9bf0ee6b as ::core::ffi::c_ulong,
    0xdb68c2ca82ed2a06 as ::core::ffi::c_ulong,
    0x892179be91d43a44 as ::core::ffi::c_ulong,
    0xab69d82e364948d4 as ::core::ffi::c_ulong,
];
static mut powers_ten_e: [::core::ffi::c_int; 687] = [
    -(1203 as ::core::ffi::c_int),
    -(1200 as ::core::ffi::c_int),
    -(1196 as ::core::ffi::c_int),
    -(1193 as ::core::ffi::c_int),
    -(1190 as ::core::ffi::c_int),
    -(1186 as ::core::ffi::c_int),
    -(1183 as ::core::ffi::c_int),
    -(1180 as ::core::ffi::c_int),
    -(1176 as ::core::ffi::c_int),
    -(1173 as ::core::ffi::c_int),
    -(1170 as ::core::ffi::c_int),
    -(1166 as ::core::ffi::c_int),
    -(1163 as ::core::ffi::c_int),
    -(1160 as ::core::ffi::c_int),
    -(1156 as ::core::ffi::c_int),
    -(1153 as ::core::ffi::c_int),
    -(1150 as ::core::ffi::c_int),
    -(1146 as ::core::ffi::c_int),
    -(1143 as ::core::ffi::c_int),
    -(1140 as ::core::ffi::c_int),
    -(1136 as ::core::ffi::c_int),
    -(1133 as ::core::ffi::c_int),
    -(1130 as ::core::ffi::c_int),
    -(1127 as ::core::ffi::c_int),
    -(1123 as ::core::ffi::c_int),
    -(1120 as ::core::ffi::c_int),
    -(1117 as ::core::ffi::c_int),
    -(1113 as ::core::ffi::c_int),
    -(1110 as ::core::ffi::c_int),
    -(1107 as ::core::ffi::c_int),
    -(1103 as ::core::ffi::c_int),
    -(1100 as ::core::ffi::c_int),
    -(1097 as ::core::ffi::c_int),
    -(1093 as ::core::ffi::c_int),
    -(1090 as ::core::ffi::c_int),
    -(1087 as ::core::ffi::c_int),
    -(1083 as ::core::ffi::c_int),
    -(1080 as ::core::ffi::c_int),
    -(1077 as ::core::ffi::c_int),
    -(1073 as ::core::ffi::c_int),
    -(1070 as ::core::ffi::c_int),
    -(1067 as ::core::ffi::c_int),
    -(1063 as ::core::ffi::c_int),
    -(1060 as ::core::ffi::c_int),
    -(1057 as ::core::ffi::c_int),
    -(1053 as ::core::ffi::c_int),
    -(1050 as ::core::ffi::c_int),
    -(1047 as ::core::ffi::c_int),
    -(1043 as ::core::ffi::c_int),
    -(1040 as ::core::ffi::c_int),
    -(1037 as ::core::ffi::c_int),
    -(1034 as ::core::ffi::c_int),
    -(1030 as ::core::ffi::c_int),
    -(1027 as ::core::ffi::c_int),
    -(1024 as ::core::ffi::c_int),
    -(1020 as ::core::ffi::c_int),
    -(1017 as ::core::ffi::c_int),
    -(1014 as ::core::ffi::c_int),
    -(1010 as ::core::ffi::c_int),
    -(1007 as ::core::ffi::c_int),
    -(1004 as ::core::ffi::c_int),
    -(1000 as ::core::ffi::c_int),
    -(997 as ::core::ffi::c_int),
    -(994 as ::core::ffi::c_int),
    -(990 as ::core::ffi::c_int),
    -(987 as ::core::ffi::c_int),
    -(984 as ::core::ffi::c_int),
    -(980 as ::core::ffi::c_int),
    -(977 as ::core::ffi::c_int),
    -(974 as ::core::ffi::c_int),
    -(970 as ::core::ffi::c_int),
    -(967 as ::core::ffi::c_int),
    -(964 as ::core::ffi::c_int),
    -(960 as ::core::ffi::c_int),
    -(957 as ::core::ffi::c_int),
    -(954 as ::core::ffi::c_int),
    -(950 as ::core::ffi::c_int),
    -(947 as ::core::ffi::c_int),
    -(944 as ::core::ffi::c_int),
    -(940 as ::core::ffi::c_int),
    -(937 as ::core::ffi::c_int),
    -(934 as ::core::ffi::c_int),
    -(931 as ::core::ffi::c_int),
    -(927 as ::core::ffi::c_int),
    -(924 as ::core::ffi::c_int),
    -(921 as ::core::ffi::c_int),
    -(917 as ::core::ffi::c_int),
    -(914 as ::core::ffi::c_int),
    -(911 as ::core::ffi::c_int),
    -(907 as ::core::ffi::c_int),
    -(904 as ::core::ffi::c_int),
    -(901 as ::core::ffi::c_int),
    -(897 as ::core::ffi::c_int),
    -(894 as ::core::ffi::c_int),
    -(891 as ::core::ffi::c_int),
    -(887 as ::core::ffi::c_int),
    -(884 as ::core::ffi::c_int),
    -(881 as ::core::ffi::c_int),
    -(877 as ::core::ffi::c_int),
    -(874 as ::core::ffi::c_int),
    -(871 as ::core::ffi::c_int),
    -(867 as ::core::ffi::c_int),
    -(864 as ::core::ffi::c_int),
    -(861 as ::core::ffi::c_int),
    -(857 as ::core::ffi::c_int),
    -(854 as ::core::ffi::c_int),
    -(851 as ::core::ffi::c_int),
    -(847 as ::core::ffi::c_int),
    -(844 as ::core::ffi::c_int),
    -(841 as ::core::ffi::c_int),
    -(838 as ::core::ffi::c_int),
    -(834 as ::core::ffi::c_int),
    -(831 as ::core::ffi::c_int),
    -(828 as ::core::ffi::c_int),
    -(824 as ::core::ffi::c_int),
    -(821 as ::core::ffi::c_int),
    -(818 as ::core::ffi::c_int),
    -(814 as ::core::ffi::c_int),
    -(811 as ::core::ffi::c_int),
    -(808 as ::core::ffi::c_int),
    -(804 as ::core::ffi::c_int),
    -(801 as ::core::ffi::c_int),
    -(798 as ::core::ffi::c_int),
    -(794 as ::core::ffi::c_int),
    -(791 as ::core::ffi::c_int),
    -(788 as ::core::ffi::c_int),
    -(784 as ::core::ffi::c_int),
    -(781 as ::core::ffi::c_int),
    -(778 as ::core::ffi::c_int),
    -(774 as ::core::ffi::c_int),
    -(771 as ::core::ffi::c_int),
    -(768 as ::core::ffi::c_int),
    -(764 as ::core::ffi::c_int),
    -(761 as ::core::ffi::c_int),
    -(758 as ::core::ffi::c_int),
    -(754 as ::core::ffi::c_int),
    -(751 as ::core::ffi::c_int),
    -(748 as ::core::ffi::c_int),
    -(744 as ::core::ffi::c_int),
    -(741 as ::core::ffi::c_int),
    -(738 as ::core::ffi::c_int),
    -(735 as ::core::ffi::c_int),
    -(731 as ::core::ffi::c_int),
    -(728 as ::core::ffi::c_int),
    -(725 as ::core::ffi::c_int),
    -(721 as ::core::ffi::c_int),
    -(718 as ::core::ffi::c_int),
    -(715 as ::core::ffi::c_int),
    -(711 as ::core::ffi::c_int),
    -(708 as ::core::ffi::c_int),
    -(705 as ::core::ffi::c_int),
    -(701 as ::core::ffi::c_int),
    -(698 as ::core::ffi::c_int),
    -(695 as ::core::ffi::c_int),
    -(691 as ::core::ffi::c_int),
    -(688 as ::core::ffi::c_int),
    -(685 as ::core::ffi::c_int),
    -(681 as ::core::ffi::c_int),
    -(678 as ::core::ffi::c_int),
    -(675 as ::core::ffi::c_int),
    -(671 as ::core::ffi::c_int),
    -(668 as ::core::ffi::c_int),
    -(665 as ::core::ffi::c_int),
    -(661 as ::core::ffi::c_int),
    -(658 as ::core::ffi::c_int),
    -(655 as ::core::ffi::c_int),
    -(651 as ::core::ffi::c_int),
    -(648 as ::core::ffi::c_int),
    -(645 as ::core::ffi::c_int),
    -(642 as ::core::ffi::c_int),
    -(638 as ::core::ffi::c_int),
    -(635 as ::core::ffi::c_int),
    -(632 as ::core::ffi::c_int),
    -(628 as ::core::ffi::c_int),
    -(625 as ::core::ffi::c_int),
    -(622 as ::core::ffi::c_int),
    -(618 as ::core::ffi::c_int),
    -(615 as ::core::ffi::c_int),
    -(612 as ::core::ffi::c_int),
    -(608 as ::core::ffi::c_int),
    -(605 as ::core::ffi::c_int),
    -(602 as ::core::ffi::c_int),
    -(598 as ::core::ffi::c_int),
    -(595 as ::core::ffi::c_int),
    -(592 as ::core::ffi::c_int),
    -(588 as ::core::ffi::c_int),
    -(585 as ::core::ffi::c_int),
    -(582 as ::core::ffi::c_int),
    -(578 as ::core::ffi::c_int),
    -(575 as ::core::ffi::c_int),
    -(572 as ::core::ffi::c_int),
    -(568 as ::core::ffi::c_int),
    -(565 as ::core::ffi::c_int),
    -(562 as ::core::ffi::c_int),
    -(558 as ::core::ffi::c_int),
    -(555 as ::core::ffi::c_int),
    -(552 as ::core::ffi::c_int),
    -(549 as ::core::ffi::c_int),
    -(545 as ::core::ffi::c_int),
    -(542 as ::core::ffi::c_int),
    -(539 as ::core::ffi::c_int),
    -(535 as ::core::ffi::c_int),
    -(532 as ::core::ffi::c_int),
    -(529 as ::core::ffi::c_int),
    -(525 as ::core::ffi::c_int),
    -(522 as ::core::ffi::c_int),
    -(519 as ::core::ffi::c_int),
    -(515 as ::core::ffi::c_int),
    -(512 as ::core::ffi::c_int),
    -(509 as ::core::ffi::c_int),
    -(505 as ::core::ffi::c_int),
    -(502 as ::core::ffi::c_int),
    -(499 as ::core::ffi::c_int),
    -(495 as ::core::ffi::c_int),
    -(492 as ::core::ffi::c_int),
    -(489 as ::core::ffi::c_int),
    -(485 as ::core::ffi::c_int),
    -(482 as ::core::ffi::c_int),
    -(479 as ::core::ffi::c_int),
    -(475 as ::core::ffi::c_int),
    -(472 as ::core::ffi::c_int),
    -(469 as ::core::ffi::c_int),
    -(465 as ::core::ffi::c_int),
    -(462 as ::core::ffi::c_int),
    -(459 as ::core::ffi::c_int),
    -(455 as ::core::ffi::c_int),
    -(452 as ::core::ffi::c_int),
    -(449 as ::core::ffi::c_int),
    -(446 as ::core::ffi::c_int),
    -(442 as ::core::ffi::c_int),
    -(439 as ::core::ffi::c_int),
    -(436 as ::core::ffi::c_int),
    -(432 as ::core::ffi::c_int),
    -(429 as ::core::ffi::c_int),
    -(426 as ::core::ffi::c_int),
    -(422 as ::core::ffi::c_int),
    -(419 as ::core::ffi::c_int),
    -(416 as ::core::ffi::c_int),
    -(412 as ::core::ffi::c_int),
    -(409 as ::core::ffi::c_int),
    -(406 as ::core::ffi::c_int),
    -(402 as ::core::ffi::c_int),
    -(399 as ::core::ffi::c_int),
    -(396 as ::core::ffi::c_int),
    -(392 as ::core::ffi::c_int),
    -(389 as ::core::ffi::c_int),
    -(386 as ::core::ffi::c_int),
    -(382 as ::core::ffi::c_int),
    -(379 as ::core::ffi::c_int),
    -(376 as ::core::ffi::c_int),
    -(372 as ::core::ffi::c_int),
    -(369 as ::core::ffi::c_int),
    -(366 as ::core::ffi::c_int),
    -(362 as ::core::ffi::c_int),
    -(359 as ::core::ffi::c_int),
    -(356 as ::core::ffi::c_int),
    -(353 as ::core::ffi::c_int),
    -(349 as ::core::ffi::c_int),
    -(346 as ::core::ffi::c_int),
    -(343 as ::core::ffi::c_int),
    -(339 as ::core::ffi::c_int),
    -(336 as ::core::ffi::c_int),
    -(333 as ::core::ffi::c_int),
    -(329 as ::core::ffi::c_int),
    -(326 as ::core::ffi::c_int),
    -(323 as ::core::ffi::c_int),
    -(319 as ::core::ffi::c_int),
    -(316 as ::core::ffi::c_int),
    -(313 as ::core::ffi::c_int),
    -(309 as ::core::ffi::c_int),
    -(306 as ::core::ffi::c_int),
    -(303 as ::core::ffi::c_int),
    -(299 as ::core::ffi::c_int),
    -(296 as ::core::ffi::c_int),
    -(293 as ::core::ffi::c_int),
    -(289 as ::core::ffi::c_int),
    -(286 as ::core::ffi::c_int),
    -(283 as ::core::ffi::c_int),
    -(279 as ::core::ffi::c_int),
    -(276 as ::core::ffi::c_int),
    -(273 as ::core::ffi::c_int),
    -(269 as ::core::ffi::c_int),
    -(266 as ::core::ffi::c_int),
    -(263 as ::core::ffi::c_int),
    -(259 as ::core::ffi::c_int),
    -(256 as ::core::ffi::c_int),
    -(253 as ::core::ffi::c_int),
    -(250 as ::core::ffi::c_int),
    -(246 as ::core::ffi::c_int),
    -(243 as ::core::ffi::c_int),
    -(240 as ::core::ffi::c_int),
    -(236 as ::core::ffi::c_int),
    -(233 as ::core::ffi::c_int),
    -(230 as ::core::ffi::c_int),
    -(226 as ::core::ffi::c_int),
    -(223 as ::core::ffi::c_int),
    -(220 as ::core::ffi::c_int),
    -(216 as ::core::ffi::c_int),
    -(213 as ::core::ffi::c_int),
    -(210 as ::core::ffi::c_int),
    -(206 as ::core::ffi::c_int),
    -(203 as ::core::ffi::c_int),
    -(200 as ::core::ffi::c_int),
    -(196 as ::core::ffi::c_int),
    -(193 as ::core::ffi::c_int),
    -(190 as ::core::ffi::c_int),
    -(186 as ::core::ffi::c_int),
    -(183 as ::core::ffi::c_int),
    -(180 as ::core::ffi::c_int),
    -(176 as ::core::ffi::c_int),
    -(173 as ::core::ffi::c_int),
    -(170 as ::core::ffi::c_int),
    -(166 as ::core::ffi::c_int),
    -(163 as ::core::ffi::c_int),
    -(160 as ::core::ffi::c_int),
    -(157 as ::core::ffi::c_int),
    -(153 as ::core::ffi::c_int),
    -(150 as ::core::ffi::c_int),
    -(147 as ::core::ffi::c_int),
    -(143 as ::core::ffi::c_int),
    -(140 as ::core::ffi::c_int),
    -(137 as ::core::ffi::c_int),
    -(133 as ::core::ffi::c_int),
    -(130 as ::core::ffi::c_int),
    -(127 as ::core::ffi::c_int),
    -(123 as ::core::ffi::c_int),
    -(120 as ::core::ffi::c_int),
    -(117 as ::core::ffi::c_int),
    -(113 as ::core::ffi::c_int),
    -(110 as ::core::ffi::c_int),
    -(107 as ::core::ffi::c_int),
    -(103 as ::core::ffi::c_int),
    -(100 as ::core::ffi::c_int),
    -(97 as ::core::ffi::c_int),
    -(93 as ::core::ffi::c_int),
    -(90 as ::core::ffi::c_int),
    -(87 as ::core::ffi::c_int),
    -(83 as ::core::ffi::c_int),
    -(80 as ::core::ffi::c_int),
    -(77 as ::core::ffi::c_int),
    -(73 as ::core::ffi::c_int),
    -(70 as ::core::ffi::c_int),
    -(67 as ::core::ffi::c_int),
    -(63 as ::core::ffi::c_int),
    -(60 as ::core::ffi::c_int),
    -(57 as ::core::ffi::c_int),
    -(54 as ::core::ffi::c_int),
    -(50 as ::core::ffi::c_int),
    -(47 as ::core::ffi::c_int),
    -(44 as ::core::ffi::c_int),
    -(40 as ::core::ffi::c_int),
    -(37 as ::core::ffi::c_int),
    -(34 as ::core::ffi::c_int),
    -(30 as ::core::ffi::c_int),
    -(27 as ::core::ffi::c_int),
    -(24 as ::core::ffi::c_int),
    -(20 as ::core::ffi::c_int),
    -(17 as ::core::ffi::c_int),
    -(14 as ::core::ffi::c_int),
    -(10 as ::core::ffi::c_int),
    -(7 as ::core::ffi::c_int),
    -(4 as ::core::ffi::c_int),
    0 as ::core::ffi::c_int,
    3 as ::core::ffi::c_int,
    6 as ::core::ffi::c_int,
    10 as ::core::ffi::c_int,
    13 as ::core::ffi::c_int,
    16 as ::core::ffi::c_int,
    20 as ::core::ffi::c_int,
    23 as ::core::ffi::c_int,
    26 as ::core::ffi::c_int,
    30 as ::core::ffi::c_int,
    33 as ::core::ffi::c_int,
    36 as ::core::ffi::c_int,
    39 as ::core::ffi::c_int,
    43 as ::core::ffi::c_int,
    46 as ::core::ffi::c_int,
    49 as ::core::ffi::c_int,
    53 as ::core::ffi::c_int,
    56 as ::core::ffi::c_int,
    59 as ::core::ffi::c_int,
    63 as ::core::ffi::c_int,
    66 as ::core::ffi::c_int,
    69 as ::core::ffi::c_int,
    73 as ::core::ffi::c_int,
    76 as ::core::ffi::c_int,
    79 as ::core::ffi::c_int,
    83 as ::core::ffi::c_int,
    86 as ::core::ffi::c_int,
    89 as ::core::ffi::c_int,
    93 as ::core::ffi::c_int,
    96 as ::core::ffi::c_int,
    99 as ::core::ffi::c_int,
    103 as ::core::ffi::c_int,
    106 as ::core::ffi::c_int,
    109 as ::core::ffi::c_int,
    113 as ::core::ffi::c_int,
    116 as ::core::ffi::c_int,
    119 as ::core::ffi::c_int,
    123 as ::core::ffi::c_int,
    126 as ::core::ffi::c_int,
    129 as ::core::ffi::c_int,
    132 as ::core::ffi::c_int,
    136 as ::core::ffi::c_int,
    139 as ::core::ffi::c_int,
    142 as ::core::ffi::c_int,
    146 as ::core::ffi::c_int,
    149 as ::core::ffi::c_int,
    152 as ::core::ffi::c_int,
    156 as ::core::ffi::c_int,
    159 as ::core::ffi::c_int,
    162 as ::core::ffi::c_int,
    166 as ::core::ffi::c_int,
    169 as ::core::ffi::c_int,
    172 as ::core::ffi::c_int,
    176 as ::core::ffi::c_int,
    179 as ::core::ffi::c_int,
    182 as ::core::ffi::c_int,
    186 as ::core::ffi::c_int,
    189 as ::core::ffi::c_int,
    192 as ::core::ffi::c_int,
    196 as ::core::ffi::c_int,
    199 as ::core::ffi::c_int,
    202 as ::core::ffi::c_int,
    206 as ::core::ffi::c_int,
    209 as ::core::ffi::c_int,
    212 as ::core::ffi::c_int,
    216 as ::core::ffi::c_int,
    219 as ::core::ffi::c_int,
    222 as ::core::ffi::c_int,
    226 as ::core::ffi::c_int,
    229 as ::core::ffi::c_int,
    232 as ::core::ffi::c_int,
    235 as ::core::ffi::c_int,
    239 as ::core::ffi::c_int,
    242 as ::core::ffi::c_int,
    245 as ::core::ffi::c_int,
    249 as ::core::ffi::c_int,
    252 as ::core::ffi::c_int,
    255 as ::core::ffi::c_int,
    259 as ::core::ffi::c_int,
    262 as ::core::ffi::c_int,
    265 as ::core::ffi::c_int,
    269 as ::core::ffi::c_int,
    272 as ::core::ffi::c_int,
    275 as ::core::ffi::c_int,
    279 as ::core::ffi::c_int,
    282 as ::core::ffi::c_int,
    285 as ::core::ffi::c_int,
    289 as ::core::ffi::c_int,
    292 as ::core::ffi::c_int,
    295 as ::core::ffi::c_int,
    299 as ::core::ffi::c_int,
    302 as ::core::ffi::c_int,
    305 as ::core::ffi::c_int,
    309 as ::core::ffi::c_int,
    312 as ::core::ffi::c_int,
    315 as ::core::ffi::c_int,
    319 as ::core::ffi::c_int,
    322 as ::core::ffi::c_int,
    325 as ::core::ffi::c_int,
    328 as ::core::ffi::c_int,
    332 as ::core::ffi::c_int,
    335 as ::core::ffi::c_int,
    338 as ::core::ffi::c_int,
    342 as ::core::ffi::c_int,
    345 as ::core::ffi::c_int,
    348 as ::core::ffi::c_int,
    352 as ::core::ffi::c_int,
    355 as ::core::ffi::c_int,
    358 as ::core::ffi::c_int,
    362 as ::core::ffi::c_int,
    365 as ::core::ffi::c_int,
    368 as ::core::ffi::c_int,
    372 as ::core::ffi::c_int,
    375 as ::core::ffi::c_int,
    378 as ::core::ffi::c_int,
    382 as ::core::ffi::c_int,
    385 as ::core::ffi::c_int,
    388 as ::core::ffi::c_int,
    392 as ::core::ffi::c_int,
    395 as ::core::ffi::c_int,
    398 as ::core::ffi::c_int,
    402 as ::core::ffi::c_int,
    405 as ::core::ffi::c_int,
    408 as ::core::ffi::c_int,
    412 as ::core::ffi::c_int,
    415 as ::core::ffi::c_int,
    418 as ::core::ffi::c_int,
    422 as ::core::ffi::c_int,
    425 as ::core::ffi::c_int,
    428 as ::core::ffi::c_int,
    431 as ::core::ffi::c_int,
    435 as ::core::ffi::c_int,
    438 as ::core::ffi::c_int,
    441 as ::core::ffi::c_int,
    445 as ::core::ffi::c_int,
    448 as ::core::ffi::c_int,
    451 as ::core::ffi::c_int,
    455 as ::core::ffi::c_int,
    458 as ::core::ffi::c_int,
    461 as ::core::ffi::c_int,
    465 as ::core::ffi::c_int,
    468 as ::core::ffi::c_int,
    471 as ::core::ffi::c_int,
    475 as ::core::ffi::c_int,
    478 as ::core::ffi::c_int,
    481 as ::core::ffi::c_int,
    485 as ::core::ffi::c_int,
    488 as ::core::ffi::c_int,
    491 as ::core::ffi::c_int,
    495 as ::core::ffi::c_int,
    498 as ::core::ffi::c_int,
    501 as ::core::ffi::c_int,
    505 as ::core::ffi::c_int,
    508 as ::core::ffi::c_int,
    511 as ::core::ffi::c_int,
    515 as ::core::ffi::c_int,
    518 as ::core::ffi::c_int,
    521 as ::core::ffi::c_int,
    524 as ::core::ffi::c_int,
    528 as ::core::ffi::c_int,
    531 as ::core::ffi::c_int,
    534 as ::core::ffi::c_int,
    538 as ::core::ffi::c_int,
    541 as ::core::ffi::c_int,
    544 as ::core::ffi::c_int,
    548 as ::core::ffi::c_int,
    551 as ::core::ffi::c_int,
    554 as ::core::ffi::c_int,
    558 as ::core::ffi::c_int,
    561 as ::core::ffi::c_int,
    564 as ::core::ffi::c_int,
    568 as ::core::ffi::c_int,
    571 as ::core::ffi::c_int,
    574 as ::core::ffi::c_int,
    578 as ::core::ffi::c_int,
    581 as ::core::ffi::c_int,
    584 as ::core::ffi::c_int,
    588 as ::core::ffi::c_int,
    591 as ::core::ffi::c_int,
    594 as ::core::ffi::c_int,
    598 as ::core::ffi::c_int,
    601 as ::core::ffi::c_int,
    604 as ::core::ffi::c_int,
    608 as ::core::ffi::c_int,
    611 as ::core::ffi::c_int,
    614 as ::core::ffi::c_int,
    617 as ::core::ffi::c_int,
    621 as ::core::ffi::c_int,
    624 as ::core::ffi::c_int,
    627 as ::core::ffi::c_int,
    631 as ::core::ffi::c_int,
    634 as ::core::ffi::c_int,
    637 as ::core::ffi::c_int,
    641 as ::core::ffi::c_int,
    644 as ::core::ffi::c_int,
    647 as ::core::ffi::c_int,
    651 as ::core::ffi::c_int,
    654 as ::core::ffi::c_int,
    657 as ::core::ffi::c_int,
    661 as ::core::ffi::c_int,
    664 as ::core::ffi::c_int,
    667 as ::core::ffi::c_int,
    671 as ::core::ffi::c_int,
    674 as ::core::ffi::c_int,
    677 as ::core::ffi::c_int,
    681 as ::core::ffi::c_int,
    684 as ::core::ffi::c_int,
    687 as ::core::ffi::c_int,
    691 as ::core::ffi::c_int,
    694 as ::core::ffi::c_int,
    697 as ::core::ffi::c_int,
    701 as ::core::ffi::c_int,
    704 as ::core::ffi::c_int,
    707 as ::core::ffi::c_int,
    711 as ::core::ffi::c_int,
    714 as ::core::ffi::c_int,
    717 as ::core::ffi::c_int,
    720 as ::core::ffi::c_int,
    724 as ::core::ffi::c_int,
    727 as ::core::ffi::c_int,
    730 as ::core::ffi::c_int,
    734 as ::core::ffi::c_int,
    737 as ::core::ffi::c_int,
    740 as ::core::ffi::c_int,
    744 as ::core::ffi::c_int,
    747 as ::core::ffi::c_int,
    750 as ::core::ffi::c_int,
    754 as ::core::ffi::c_int,
    757 as ::core::ffi::c_int,
    760 as ::core::ffi::c_int,
    764 as ::core::ffi::c_int,
    767 as ::core::ffi::c_int,
    770 as ::core::ffi::c_int,
    774 as ::core::ffi::c_int,
    777 as ::core::ffi::c_int,
    780 as ::core::ffi::c_int,
    784 as ::core::ffi::c_int,
    787 as ::core::ffi::c_int,
    790 as ::core::ffi::c_int,
    794 as ::core::ffi::c_int,
    797 as ::core::ffi::c_int,
    800 as ::core::ffi::c_int,
    804 as ::core::ffi::c_int,
    807 as ::core::ffi::c_int,
    810 as ::core::ffi::c_int,
    813 as ::core::ffi::c_int,
    817 as ::core::ffi::c_int,
    820 as ::core::ffi::c_int,
    823 as ::core::ffi::c_int,
    827 as ::core::ffi::c_int,
    830 as ::core::ffi::c_int,
    833 as ::core::ffi::c_int,
    837 as ::core::ffi::c_int,
    840 as ::core::ffi::c_int,
    843 as ::core::ffi::c_int,
    847 as ::core::ffi::c_int,
    850 as ::core::ffi::c_int,
    853 as ::core::ffi::c_int,
    857 as ::core::ffi::c_int,
    860 as ::core::ffi::c_int,
    863 as ::core::ffi::c_int,
    867 as ::core::ffi::c_int,
    870 as ::core::ffi::c_int,
    873 as ::core::ffi::c_int,
    877 as ::core::ffi::c_int,
    880 as ::core::ffi::c_int,
    883 as ::core::ffi::c_int,
    887 as ::core::ffi::c_int,
    890 as ::core::ffi::c_int,
    893 as ::core::ffi::c_int,
    897 as ::core::ffi::c_int,
    900 as ::core::ffi::c_int,
    903 as ::core::ffi::c_int,
    907 as ::core::ffi::c_int,
    910 as ::core::ffi::c_int,
    913 as ::core::ffi::c_int,
    916 as ::core::ffi::c_int,
    920 as ::core::ffi::c_int,
    923 as ::core::ffi::c_int,
    926 as ::core::ffi::c_int,
    930 as ::core::ffi::c_int,
    933 as ::core::ffi::c_int,
    936 as ::core::ffi::c_int,
    940 as ::core::ffi::c_int,
    943 as ::core::ffi::c_int,
    946 as ::core::ffi::c_int,
    950 as ::core::ffi::c_int,
    953 as ::core::ffi::c_int,
    956 as ::core::ffi::c_int,
    960 as ::core::ffi::c_int,
    963 as ::core::ffi::c_int,
    966 as ::core::ffi::c_int,
    970 as ::core::ffi::c_int,
    973 as ::core::ffi::c_int,
    976 as ::core::ffi::c_int,
    980 as ::core::ffi::c_int,
    983 as ::core::ffi::c_int,
    986 as ::core::ffi::c_int,
    990 as ::core::ffi::c_int,
    993 as ::core::ffi::c_int,
    996 as ::core::ffi::c_int,
    1000 as ::core::ffi::c_int,
    1003 as ::core::ffi::c_int,
    1006 as ::core::ffi::c_int,
    1009 as ::core::ffi::c_int,
    1013 as ::core::ffi::c_int,
    1016 as ::core::ffi::c_int,
    1019 as ::core::ffi::c_int,
    1023 as ::core::ffi::c_int,
    1026 as ::core::ffi::c_int,
    1029 as ::core::ffi::c_int,
    1033 as ::core::ffi::c_int,
    1036 as ::core::ffi::c_int,
    1039 as ::core::ffi::c_int,
    1043 as ::core::ffi::c_int,
    1046 as ::core::ffi::c_int,
    1049 as ::core::ffi::c_int,
    1053 as ::core::ffi::c_int,
    1056 as ::core::ffi::c_int,
    1059 as ::core::ffi::c_int,
    1063 as ::core::ffi::c_int,
    1066 as ::core::ffi::c_int,
    1069 as ::core::ffi::c_int,
    1073 as ::core::ffi::c_int,
    1076 as ::core::ffi::c_int,
];
unsafe extern "C" fn cached_power(mut k: ::core::ffi::c_int) -> diy_fp_t {
    let mut res: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut index: ::core::ffi::c_int = 343 as ::core::ffi::c_int + k;
    res.f = powers_ten[index as usize];
    res.e = powers_ten_e[index as usize];
    return res;
}
unsafe extern "C" fn k_comp(
    mut e: ::core::ffi::c_int,
    mut alpha: ::core::ffi::c_int,
    mut gamma: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    return ceil(
        (alpha - e + 63 as ::core::ffi::c_int) as ::core::ffi::c_double * D_1_LOG2_10,
    ) as ::core::ffi::c_int;
}
unsafe extern "C" fn minus(mut x: diy_fp_t, mut y: diy_fp_t) -> diy_fp_t {
    let mut r: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    r.f = x.f.wrapping_sub(y.f);
    r.e = x.e;
    return r;
}
unsafe extern "C" fn multiply(mut x: diy_fp_t, mut y: diy_fp_t) -> diy_fp_t {
    let mut a: uint64_t = 0;
    let mut b: uint64_t = 0;
    let mut c: uint64_t = 0;
    let mut d: uint64_t = 0;
    let mut ac: uint64_t = 0;
    let mut bc: uint64_t = 0;
    let mut ad: uint64_t = 0;
    let mut bd: uint64_t = 0;
    let mut tmp: uint64_t = 0;
    let mut r: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut M32: uint64_t = 0xffffffff as uint64_t;
    a = x.f >> 32 as ::core::ffi::c_int;
    b = x.f & M32;
    c = y.f >> 32 as ::core::ffi::c_int;
    d = y.f & M32;
    ac = a.wrapping_mul(c);
    bc = b.wrapping_mul(c);
    ad = a.wrapping_mul(d);
    bd = b.wrapping_mul(d);
    tmp = (bd >> 32 as ::core::ffi::c_int).wrapping_add(ad & M32).wrapping_add(bc & M32);
    tmp = (tmp as ::core::ffi::c_ulong)
        .wrapping_add(
            ((1 as ::core::ffi::c_uint) << 31 as ::core::ffi::c_int)
                as ::core::ffi::c_ulong,
        ) as uint64_t as uint64_t;
    r.f = ac
        .wrapping_add(ad >> 32 as ::core::ffi::c_int)
        .wrapping_add(bc >> 32 as ::core::ffi::c_int)
        .wrapping_add(tmp >> 32 as ::core::ffi::c_int);
    r.e = x.e + y.e + 64 as ::core::ffi::c_int;
    return r;
}
unsafe extern "C" fn double_to_uint64(mut d: ::core::ffi::c_double) -> uint64_t {
    let mut n: uint64_t = 0;
    memcpy(
        &raw mut n as *mut ::core::ffi::c_void,
        &raw mut d as *const ::core::ffi::c_void,
        8 as size_t,
    );
    return n;
}
pub const DP_SIGNIFICAND_SIZE: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const DP_EXPONENT_BIAS: ::core::ffi::c_int = 0x3ff as ::core::ffi::c_int
    + DP_SIGNIFICAND_SIZE;
pub const DP_MIN_EXPONENT: ::core::ffi::c_int = -DP_EXPONENT_BIAS;
pub const DP_EXPONENT_MASK: ::core::ffi::c_long = 0x7ff0000000000000
    as ::core::ffi::c_long;
pub const DP_SIGNIFICAND_MASK: ::core::ffi::c_long = 0xfffffffffffff
    as ::core::ffi::c_long;
pub const DP_HIDDEN_BIT: ::core::ffi::c_long = 0x10000000000000 as ::core::ffi::c_long;
unsafe extern "C" fn double2diy_fp(mut d: ::core::ffi::c_double) -> diy_fp_t {
    let mut d64: uint64_t = double_to_uint64(d);
    let mut biased_e: ::core::ffi::c_int = ((d64 & DP_EXPONENT_MASK as uint64_t)
        >> DP_SIGNIFICAND_SIZE) as ::core::ffi::c_int;
    let mut significand: uint64_t = d64 & DP_SIGNIFICAND_MASK as uint64_t;
    let mut res: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    if biased_e != 0 as ::core::ffi::c_int {
        res.f = significand.wrapping_add(DP_HIDDEN_BIT as uint64_t);
        res.e = biased_e - DP_EXPONENT_BIAS;
    } else {
        res.f = significand;
        res.e = DP_MIN_EXPONENT + 1 as ::core::ffi::c_int;
    }
    return res;
}
unsafe extern "C" fn normalize_boundary(mut in_0: diy_fp_t) -> diy_fp_t {
    let mut res: diy_fp_t = in_0;
    while res.f & (DP_HIDDEN_BIT << 1 as ::core::ffi::c_int) as uint64_t == 0 {
        res.f <<= 1 as ::core::ffi::c_int;
        res.e -= 1;
    }
    res.f <<= DIY_SIGNIFICAND_SIZE - DP_SIGNIFICAND_SIZE - 2 as ::core::ffi::c_int;
    res.e = res.e
        - (DIY_SIGNIFICAND_SIZE - DP_SIGNIFICAND_SIZE - 2 as ::core::ffi::c_int);
    return res;
}
unsafe extern "C" fn normalized_boundaries(
    mut d: ::core::ffi::c_double,
    mut out_m_minus: *mut diy_fp_t,
    mut out_m_plus: *mut diy_fp_t,
) {
    let mut v: diy_fp_t = double2diy_fp(d);
    let mut pl: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut mi: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut significand_is_zero: ::core::ffi::c_int = (v.f == DP_HIDDEN_BIT as uint64_t)
        as ::core::ffi::c_int;
    pl.f = (v.f << 1 as ::core::ffi::c_int).wrapping_add(1 as uint64_t);
    pl.e = v.e - 1 as ::core::ffi::c_int;
    pl = normalize_boundary(pl);
    if significand_is_zero != 0 {
        mi.f = (v.f << 2 as ::core::ffi::c_int).wrapping_sub(1 as uint64_t);
        mi.e = v.e - 2 as ::core::ffi::c_int;
    } else {
        mi.f = (v.f << 1 as ::core::ffi::c_int).wrapping_sub(1 as uint64_t);
        mi.e = v.e - 1 as ::core::ffi::c_int;
    }
    mi.f <<= mi.e - pl.e;
    mi.e = pl.e;
    *out_m_plus = pl;
    *out_m_minus = mi;
}
pub const TEN2: ::core::ffi::c_int = 100 as ::core::ffi::c_int;
unsafe extern "C" fn digit_gen(
    mut Mp: diy_fp_t,
    mut delta: diy_fp_t,
    mut buffer: *mut ::core::ffi::c_char,
    mut len: *mut ::core::ffi::c_int,
    mut K: *mut ::core::ffi::c_int,
) {
    let mut div: uint32_t = 0;
    let mut p1: uint32_t = 0;
    let mut p2: uint64_t = 0;
    let mut d: ::core::ffi::c_int = 0;
    let mut kappa: ::core::ffi::c_int = 0;
    let mut one: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    one.f = (1 as ::core::ffi::c_int as uint64_t) << -Mp.e;
    one.e = Mp.e;
    p1 = (Mp.f >> -one.e) as uint32_t;
    p2 = Mp.f & one.f.wrapping_sub(1 as uint64_t);
    *len = 0 as ::core::ffi::c_int;
    kappa = 3 as ::core::ffi::c_int;
    div = TEN2 as uint32_t;
    while kappa > 0 as ::core::ffi::c_int {
        d = p1.wrapping_div(div) as ::core::ffi::c_int;
        if d != 0 || *len != 0 {
            let fresh13 = *len;
            *len = *len + 1;
            *buffer.offset(fresh13 as isize) = ('0' as i32 + d) as ::core::ffi::c_char;
        }
        p1 = (p1 as ::core::ffi::c_uint).wrapping_rem(div as ::core::ffi::c_uint)
            as uint32_t as uint32_t;
        kappa -= 1;
        div = (div as ::core::ffi::c_uint).wrapping_div(10 as ::core::ffi::c_uint)
            as uint32_t as uint32_t;
        if ((p1 as uint64_t) << -one.e).wrapping_add(p2) <= delta.f {
            *K += kappa;
            return;
        }
    }
    loop {
        p2 = (p2 as ::core::ffi::c_ulong).wrapping_mul(10 as ::core::ffi::c_ulong)
            as uint64_t as uint64_t;
        d = (p2 >> -one.e) as ::core::ffi::c_int;
        if d != 0 || *len != 0 {
            let fresh14 = *len;
            *len = *len + 1;
            *buffer.offset(fresh14 as isize) = ('0' as i32 + d) as ::core::ffi::c_char;
        }
        p2 = (p2 as ::core::ffi::c_ulong
            & one.f.wrapping_sub(1 as uint64_t) as ::core::ffi::c_ulong) as uint64_t;
        kappa -= 1;
        delta.f = (delta.f as ::core::ffi::c_ulong)
            .wrapping_mul(10 as ::core::ffi::c_ulong) as uint64_t as uint64_t;
        if !(p2 > delta.f) {
            break;
        }
    }
    *K += kappa;
}
#[no_mangle]
pub unsafe extern "C" fn js_grisu2(
    mut v: ::core::ffi::c_double,
    mut buffer: *mut ::core::ffi::c_char,
    mut K: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut length: ::core::ffi::c_int = 0;
    let mut mk: ::core::ffi::c_int = 0;
    let mut w_m: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut w_p: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut c_mk: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut Wp: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut Wm: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut delta: diy_fp_t = diy_fp_t { f: 0, e: 0 };
    let mut q: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
    let mut alpha: ::core::ffi::c_int = -(59 as ::core::ffi::c_int);
    let mut gamma: ::core::ffi::c_int = -(56 as ::core::ffi::c_int);
    normalized_boundaries(v, &raw mut w_m, &raw mut w_p);
    mk = k_comp(w_p.e + q, alpha, gamma);
    c_mk = cached_power(mk);
    Wp = multiply(w_p, c_mk);
    Wm = multiply(w_m, c_mk);
    Wm.f = Wm.f.wrapping_add(1);
    Wp.f = Wp.f.wrapping_sub(1);
    delta = minus(Wp, Wm);
    *K = -mk;
    digit_gen(Wp, delta, buffer, &raw mut length, K);
    return length;
}
static mut maxExponent: ::core::ffi::c_int = 511 as ::core::ffi::c_int;
static mut powersOf10: [::core::ffi::c_double; 9] = [
    10.0f64,
    100.0f64,
    1.0e4f64,
    1.0e8f64,
    1.0e16f64,
    1.0e32f64,
    1.0e64f64,
    1.0e128f64,
    1.0e256f64,
];
#[no_mangle]
pub unsafe extern "C" fn js_strtod(
    mut string: *const ::core::ffi::c_char,
    mut endPtr: *mut *mut ::core::ffi::c_char,
) -> ::core::ffi::c_double {
    let mut sign: ::core::ffi::c_int = 0;
    let mut expSign: ::core::ffi::c_int = FALSE;
    let mut fraction: ::core::ffi::c_double = 0.;
    let mut dblExp: ::core::ffi::c_double = 0.;
    let mut d: *mut ::core::ffi::c_double = ::core::ptr::null_mut::<
        ::core::ffi::c_double,
    >();
    let mut p: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut c: ::core::ffi::c_int = 0;
    let mut exp: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut fracExp: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut mantSize: ::core::ffi::c_int = 0;
    let mut decPt: ::core::ffi::c_int = 0;
    let mut pExp: *const ::core::ffi::c_char = ::core::ptr::null::<
        ::core::ffi::c_char,
    >();
    p = string;
    while *p as ::core::ffi::c_int == ' ' as i32
        || *p as ::core::ffi::c_int == '\t' as i32
        || *p as ::core::ffi::c_int == '\n' as i32
        || *p as ::core::ffi::c_int == '\r' as i32
    {
        p = p.offset(1 as ::core::ffi::c_int as isize);
    }
    if *p as ::core::ffi::c_int == '-' as i32 {
        sign = TRUE;
        p = p.offset(1 as ::core::ffi::c_int as isize);
    } else {
        if *p as ::core::ffi::c_int == '+' as i32 {
            p = p.offset(1 as ::core::ffi::c_int as isize);
        }
        sign = FALSE;
    }
    decPt = -(1 as ::core::ffi::c_int);
    mantSize = 0 as ::core::ffi::c_int;
    loop {
        c = *p as ::core::ffi::c_int;
        if !(c >= '0' as i32 && c <= '9' as i32) {
            if c != '.' as i32 || decPt >= 0 as ::core::ffi::c_int {
                break;
            }
            decPt = mantSize;
        }
        p = p.offset(1 as ::core::ffi::c_int as isize);
        mantSize += 1 as ::core::ffi::c_int;
    }
    pExp = p;
    p = p.offset(-(mantSize as isize));
    if decPt < 0 as ::core::ffi::c_int {
        decPt = mantSize;
    } else {
        mantSize -= 1 as ::core::ffi::c_int;
    }
    if mantSize > 18 as ::core::ffi::c_int {
        fracExp = decPt - 18 as ::core::ffi::c_int;
        mantSize = 18 as ::core::ffi::c_int;
    } else {
        fracExp = decPt - mantSize;
    }
    if mantSize == 0 as ::core::ffi::c_int {
        fraction = 0.0f64;
        p = string;
    } else {
        let mut frac1: ::core::ffi::c_int = 0;
        let mut frac2: ::core::ffi::c_int = 0;
        frac1 = 0 as ::core::ffi::c_int;
        while mantSize > 9 as ::core::ffi::c_int {
            c = *p as ::core::ffi::c_int;
            p = p.offset(1 as ::core::ffi::c_int as isize);
            if c == '.' as i32 {
                c = *p as ::core::ffi::c_int;
                p = p.offset(1 as ::core::ffi::c_int as isize);
            }
            frac1 = 10 as ::core::ffi::c_int * frac1 + (c - '0' as i32);
            mantSize -= 1 as ::core::ffi::c_int;
        }
        frac2 = 0 as ::core::ffi::c_int;
        while mantSize > 0 as ::core::ffi::c_int {
            c = *p as ::core::ffi::c_int;
            p = p.offset(1 as ::core::ffi::c_int as isize);
            if c == '.' as i32 {
                c = *p as ::core::ffi::c_int;
                p = p.offset(1 as ::core::ffi::c_int as isize);
            }
            frac2 = 10 as ::core::ffi::c_int * frac2 + (c - '0' as i32);
            mantSize -= 1 as ::core::ffi::c_int;
        }
        fraction = 1.0e9f64 * frac1 as ::core::ffi::c_double
            + frac2 as ::core::ffi::c_double;
        p = pExp;
        if *p as ::core::ffi::c_int == 'E' as i32
            || *p as ::core::ffi::c_int == 'e' as i32
        {
            p = p.offset(1 as ::core::ffi::c_int as isize);
            if *p as ::core::ffi::c_int == '-' as i32 {
                expSign = TRUE;
                p = p.offset(1 as ::core::ffi::c_int as isize);
            } else {
                if *p as ::core::ffi::c_int == '+' as i32 {
                    p = p.offset(1 as ::core::ffi::c_int as isize);
                }
                expSign = FALSE;
            }
            while *p as ::core::ffi::c_int >= '0' as i32
                && *p as ::core::ffi::c_int <= '9' as i32
                && exp < INT_MAX / 100 as ::core::ffi::c_int
            {
                exp = exp * 10 as ::core::ffi::c_int
                    + (*p as ::core::ffi::c_int - '0' as i32);
                p = p.offset(1 as ::core::ffi::c_int as isize);
            }
            while *p as ::core::ffi::c_int >= '0' as i32
                && *p as ::core::ffi::c_int <= '9' as i32
            {
                p = p.offset(1 as ::core::ffi::c_int as isize);
            }
        }
        if expSign != 0 {
            exp = fracExp - exp;
        } else {
            exp = fracExp + exp;
        }
        if exp < -maxExponent {
            exp = maxExponent;
            expSign = TRUE;
            *__errno_location() = ERANGE;
        } else if exp > maxExponent {
            exp = maxExponent;
            expSign = FALSE;
            *__errno_location() = ERANGE;
        } else if exp < 0 as ::core::ffi::c_int {
            expSign = TRUE;
            exp = -exp;
        } else {
            expSign = FALSE;
        }
        dblExp = 1.0f64;
        d = &raw mut powersOf10 as *mut ::core::ffi::c_double;
        while exp != 0 as ::core::ffi::c_int {
            if exp & 0o1 as ::core::ffi::c_int != 0 {
                dblExp *= *d;
            }
            exp >>= 1 as ::core::ffi::c_int;
            d = d.offset(1 as ::core::ffi::c_int as isize);
        }
        if expSign != 0 {
            fraction /= dblExp;
        } else {
            fraction *= dblExp;
        }
    }
    if !endPtr.is_null() {
        *endPtr = p as *mut ::core::ffi::c_char;
    }
    if sign != 0 {
        return -fraction;
    }
    return fraction;
}
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const ERANGE: ::core::ffi::c_int = 34 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
