extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memset(
        __s: *mut libc::c_void,
        __c: libc::c_int,
        __n: size_t,
    ) -> *mut libc::c_void;
    fn memcmp(
        __s1: *const libc::c_void,
        __s2: *const libc::c_void,
        __n: size_t,
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn crypto_sign_keypair(
        pk: *mut libc::c_uchar,
        sk: *mut libc::c_uchar,
    ) -> libc::c_int;
    fn crypto_sign(
        sm: *mut libc::c_uchar,
        smlen: *mut libc::c_ulonglong,
        m: *const libc::c_uchar,
        mlen: libc::c_ulonglong,
        sk: *const libc::c_uchar,
    ) -> libc::c_int;
    fn crypto_sign_open(
        m: *mut libc::c_uchar,
        mlen: *mut libc::c_ulonglong,
        sm: *const libc::c_uchar,
        smlen: libc::c_ulonglong,
        pk: *const libc::c_uchar,
    ) -> libc::c_int;
    fn randombytes_init(
        entropy_input: *mut libc::c_uchar,
        personalization_string: *mut libc::c_uchar,
    );
    fn randombytes(
        x: *mut libc::c_uchar,
        xlen: libc::c_ulonglong,
    ) -> libc::c_int;
    fn SPX_tweak_constants(ctx: *mut spx_ctx);
    fn SPX_haraka_S_inc_init(s_inc: *mut uint8_t);
    fn SPX_haraka_S_inc_absorb(
        s_inc: *mut uint8_t,
        m: *const uint8_t,
        mlen: size_t,
        ctx: *const spx_ctx,
    );
    fn SPX_haraka_S_inc_finalize(s_inc: *mut uint8_t);
    fn SPX_haraka_S_inc_squeeze(
        out: *mut uint8_t,
        outlen: size_t,
        s_inc: *mut uint8_t,
        ctx: *const spx_ctx,
    );
}
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub type uint64_t = __uint64_t;
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct spx_ctx {
    pub pub_seed: [uint8_t; 16],
    pub sk_seed: [uint8_t; 16],
    pub tweaked512_rc64: [[uint64_t; 8]; 10],
    pub tweaked256_rc32: [[uint32_t; 8]; 10],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct kat_tr_ctx {
    pub inner: spx_ctx,
    pub s: [uint8_t; 65],
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const CRYPTO_ALGNAME: [libc::c_char; 9] =
    unsafe { std::mem::transmute::<[u8; 9], [libc::c_char; 9]>(*b"SPHINCS+\0") };
pub const CRYPTO_SECRETKEYBYTES: libc::c_int = SPX_SK_BYTES;
pub const CRYPTO_PUBLICKEYBYTES: libc::c_int = SPX_PK_BYTES;
pub const CRYPTO_BYTES: libc::c_int = SPX_BYTES;
pub const BASE_MLEN: libc::c_int = 33 as libc::c_int;
pub const LOOP_COUNT: libc::c_int = 7 as libc::c_int;
pub const KAT_SUCCESS: libc::c_int = 0 as libc::c_int;
pub const KAT_OVERFLOW: libc::c_int = -(1 as libc::c_int);
pub const KAT_CRYPTO_FAILURE: libc::c_int = -(2 as libc::c_int);
#[inline]
unsafe extern "C" fn kat_tr_init(mut ctx: *mut kat_tr_ctx) {
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < SPX_N as size_t {
        (*ctx).inner.pub_seed[i as usize] = 0 as uint8_t;
        (*ctx).inner.sk_seed[i as usize] = 0 as uint8_t;
        i = i.wrapping_add(1);
    }
    SPX_tweak_constants(&raw mut (*ctx).inner);
    SPX_haraka_S_inc_init(&raw mut (*ctx).s as *mut uint8_t);
    static mut tag: [uint8_t; 25] = unsafe {
        std::mem::transmute::<[u8; 25], [uint8_t; 25]>(*b"KAT-TRANSCRIPT-v1-HARAKA\0")
    };
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw const tag as *const uint8_t,
        (std::mem::size_of::<[uint8_t; 25]>() as size_t).wrapping_sub(1 as size_t),
        &raw mut (*ctx).inner,
    );
    let sep: uint8_t = 0 as uint8_t;
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw const sep,
        1 as size_t,
        &raw mut (*ctx).inner,
    );
}
#[inline]
unsafe extern "C" fn kat_tr_absorb_label(
    mut ctx: *mut kat_tr_ctx,
    mut label: *const libc::c_char,
) {
    let mut p: *const uint8_t = label as *const uint8_t;
    let mut n: size_t = 0 as size_t;
    while *p.offset(n as isize) != 0 {
        n = n.wrapping_add(1);
    }
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        p,
        n,
        &raw mut (*ctx).inner,
    );
    let sep: uint8_t = 0 as uint8_t;
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw const sep,
        1 as size_t,
        &raw mut (*ctx).inner,
    );
}
#[inline]
unsafe extern "C" fn kat_tr_absorb_u64(mut ctx: *mut kat_tr_ctx, mut x: libc::c_ulonglong) {
    let mut le: [uint8_t; 8] = [0; 8];
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 8 as size_t {
        le[i as usize] =
            (x >> (8 as size_t).wrapping_mul(i) & 0xff as libc::c_ulonglong) as uint8_t;
        i = i.wrapping_add(1);
    }
    let mut lenle: [uint8_t; 8] = [0; 8];
    let mut L: libc::c_ulonglong = 8 as libc::c_ulonglong;
    i = 0 as size_t;
    while i < 8 as size_t {
        lenle[i as usize] =
            (L >> (8 as size_t).wrapping_mul(i) & 0xff as libc::c_ulonglong) as uint8_t;
        i = i.wrapping_add(1);
    }
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw mut lenle as *mut uint8_t,
        8 as size_t,
        &raw mut (*ctx).inner,
    );
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw mut le as *mut uint8_t,
        8 as size_t,
        &raw mut (*ctx).inner,
    );
}
#[inline]
unsafe extern "C" fn kat_tr_absorb_bytes(
    mut ctx: *mut kat_tr_ctx,
    mut buf: *const uint8_t,
    mut len: size_t,
) {
    let mut lenle: [uint8_t; 8] = [0; 8];
    let mut L: libc::c_ulonglong = len as libc::c_ulonglong;
    let mut i: size_t = 0;
    i = 0 as size_t;
    while i < 8 as size_t {
        lenle[i as usize] =
            (L >> (8 as size_t).wrapping_mul(i) & 0xff as libc::c_ulonglong) as uint8_t;
        i = i.wrapping_add(1);
    }
    SPX_haraka_S_inc_absorb(
        &raw mut (*ctx).s as *mut uint8_t,
        &raw mut lenle as *mut uint8_t,
        8 as size_t,
        &raw mut (*ctx).inner,
    );
    if len != 0 {
        SPX_haraka_S_inc_absorb(
            &raw mut (*ctx).s as *mut uint8_t,
            buf,
            len,
            &raw mut (*ctx).inner,
        );
    }
}
#[inline]
unsafe extern "C" fn kat_tr_final(mut ctx: *mut kat_tr_ctx, mut out32: *mut uint8_t) {
    SPX_haraka_S_inc_finalize(&raw mut (*ctx).s as *mut uint8_t);
    SPX_haraka_S_inc_squeeze(
        out32 as *mut uint8_t,
        32 as size_t,
        &raw mut (*ctx).s as *mut uint8_t,
        &raw mut (*ctx).inner,
    );
}
unsafe fn main_0() -> libc::c_int {
    static mut m: [libc::c_uchar; 231] = [0; 231];
    static mut sm: [libc::c_uchar; 8087] = [0; 8087];
    static mut m1: [libc::c_uchar; 8087] = [0; 8087];
    static mut pk: [libc::c_uchar; 32] = [0; 32];
    static mut sk: [libc::c_uchar; 64] = [0; 64];
    static mut seed: [libc::c_uchar; 48] = [0; 48];
    static mut entropy_input: [libc::c_uchar; 48] = [0; 48];
    static mut msg: [libc::c_uchar; 231] = [0; 231];
    let mut mlen: libc::c_ulonglong = 0;
    let mut smlen: libc::c_ulonglong = 0;
    let mut mlen1: libc::c_ulonglong = 0;
    let mut ret: libc::c_int = 0;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < 48 as libc::c_int {
        entropy_input[i as usize] = i as libc::c_uchar;
        i += 1;
    }
    randombytes_init(
        &raw mut entropy_input as *mut libc::c_uchar,
        std::ptr::null_mut::<libc::c_uchar>(),
    );
    let mut tctx: kat_tr_ctx = kat_tr_ctx {
        inner: spx_ctx {
            pub_seed: [0; 16],
            sk_seed: [0; 16],
            tweaked512_rc64: [[0; 8]; 10],
            tweaked256_rc32: [[0; 8]; 10],
        },
        s: [0; 65],
    };
    kat_tr_init(&raw mut tctx);
    kat_tr_absorb_label(
        &raw mut tctx,
        b"CRYPTO_ALGNAME\0" as *const u8 as *const libc::c_char,
    );
    kat_tr_absorb_bytes(
        &raw mut tctx,
        CRYPTO_ALGNAME.as_ptr() as *const uint8_t,
        strlen(CRYPTO_ALGNAME.as_ptr()),
    );
    kat_tr_absorb_label(
        &raw mut tctx,
        b"SKBYTES\0" as *const u8 as *const libc::c_char,
    );
    kat_tr_absorb_u64(
        &raw mut tctx,
        CRYPTO_SECRETKEYBYTES as libc::c_ulonglong,
    );
    kat_tr_absorb_label(
        &raw mut tctx,
        b"PKBYTES\0" as *const u8 as *const libc::c_char,
    );
    kat_tr_absorb_u64(
        &raw mut tctx,
        CRYPTO_PUBLICKEYBYTES as libc::c_ulonglong,
    );
    kat_tr_absorb_label(
        &raw mut tctx,
        b"SIGBYTES\0" as *const u8 as *const libc::c_char,
    );
    kat_tr_absorb_u64(&raw mut tctx, CRYPTO_BYTES as libc::c_ulonglong);
    let mut i_0: libc::c_int = 0 as libc::c_int;
    while i_0 < LOOP_COUNT {
        randombytes(
            &raw mut seed as *mut libc::c_uchar,
            std::mem::size_of::<[libc::c_uchar; 48]>() as libc::c_ulonglong,
        );
        kat_tr_absorb_label(
            &raw mut tctx,
            b"count\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_u64(&raw mut tctx, i_0 as libc::c_ulonglong);
        kat_tr_absorb_label(
            &raw mut tctx,
            b"seed\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_bytes(
            &raw mut tctx,
            &raw mut seed as *mut libc::c_uchar,
            std::mem::size_of::<[libc::c_uchar; 48]>() as size_t,
        );
        mlen = (BASE_MLEN * (i_0 + 1 as libc::c_int)) as libc::c_ulonglong;
        if mlen > (BASE_MLEN * LOOP_COUNT) as libc::c_ulonglong {
            fprintf(
                stderr as *mut FILE,
                b"mlen overflow\n\0" as *const u8 as *const libc::c_char,
            );
            return KAT_OVERFLOW;
        }
        kat_tr_absorb_label(
            &raw mut tctx,
            b"mlen\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_u64(&raw mut tctx, mlen);
        randombytes(&raw mut msg as *mut libc::c_uchar, mlen);
        kat_tr_absorb_label(
            &raw mut tctx,
            b"msg\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_bytes(
            &raw mut tctx,
            &raw mut msg as *mut libc::c_uchar,
            mlen as size_t,
        );
        memset(
            &raw mut m as *mut libc::c_uchar as *mut libc::c_void,
            0 as libc::c_int,
            mlen as size_t,
        );
        memset(
            &raw mut m1 as *mut libc::c_uchar as *mut libc::c_void,
            0 as libc::c_int,
            mlen.wrapping_add(CRYPTO_BYTES as libc::c_ulonglong) as size_t,
        );
        memset(
            &raw mut sm as *mut libc::c_uchar as *mut libc::c_void,
            0 as libc::c_int,
            mlen.wrapping_add(CRYPTO_BYTES as libc::c_ulonglong) as size_t,
        );
        memcpy(
            &raw mut m as *mut libc::c_uchar as *mut libc::c_void,
            &raw mut msg as *mut libc::c_uchar as *const libc::c_void,
            mlen as size_t,
        );
        ret = crypto_sign_keypair(
            &raw mut pk as *mut libc::c_uchar,
            &raw mut sk as *mut libc::c_uchar,
        );
        if ret != 0 {
            fprintf(
                stderr as *mut FILE,
                b"crypto_sign_keypair=%d\n\0" as *const u8 as *const libc::c_char,
                ret,
            );
            return KAT_CRYPTO_FAILURE;
        }
        kat_tr_absorb_label(
            &raw mut tctx,
            b"pk\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_bytes(
            &raw mut tctx,
            &raw mut pk as *mut libc::c_uchar,
            CRYPTO_PUBLICKEYBYTES as size_t,
        );
        kat_tr_absorb_label(
            &raw mut tctx,
            b"sk\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_bytes(
            &raw mut tctx,
            &raw mut sk as *mut libc::c_uchar,
            CRYPTO_SECRETKEYBYTES as size_t,
        );
        ret = crypto_sign(
            &raw mut sm as *mut libc::c_uchar,
            &raw mut smlen,
            &raw mut m as *mut libc::c_uchar,
            mlen,
            &raw mut sk as *mut libc::c_uchar,
        );
        if ret != 0 {
            fprintf(
                stderr as *mut FILE,
                b"crypto_sign=%d\n\0" as *const u8 as *const libc::c_char,
                ret,
            );
            return KAT_CRYPTO_FAILURE;
        }
        kat_tr_absorb_label(
            &raw mut tctx,
            b"smlen\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_u64(&raw mut tctx, smlen);
        kat_tr_absorb_label(
            &raw mut tctx,
            b"sm\0" as *const u8 as *const libc::c_char,
        );
        kat_tr_absorb_bytes(
            &raw mut tctx,
            &raw mut sm as *mut libc::c_uchar,
            smlen as size_t,
        );
        ret = crypto_sign_open(
            &raw mut m1 as *mut libc::c_uchar,
            &raw mut mlen1,
            &raw mut sm as *mut libc::c_uchar,
            smlen,
            &raw mut pk as *mut libc::c_uchar,
        );
        if ret != 0 {
            fprintf(
                stderr as *mut FILE,
                b"crypto_sign_open=%d\n\0" as *const u8 as *const libc::c_char,
                ret,
            );
            return KAT_CRYPTO_FAILURE;
        }
        if mlen1 != mlen {
            fprintf(
                stderr as *mut FILE,
                b"mlen mismatch\n\0" as *const u8 as *const libc::c_char,
            );
            return KAT_CRYPTO_FAILURE;
        }
        if memcmp(
            &raw mut m as *mut libc::c_uchar as *const libc::c_void,
            &raw mut m1 as *mut libc::c_uchar as *const libc::c_void,
            mlen as size_t,
        ) != 0 as libc::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"m mismatch\n\0" as *const u8 as *const libc::c_char,
            );
            return KAT_CRYPTO_FAILURE;
        }
        i_0 += 1;
    }
    let mut digest: [uint8_t; 32] = [
        0 as libc::c_int as uint8_t,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    kat_tr_final(&raw mut tctx, &raw mut digest as *mut uint8_t);
    printf(b"KAT transcript digest = \0" as *const u8 as *const libc::c_char);
    let mut i_1: size_t = 0 as size_t;
    while i_1 < 32 as size_t {
        printf(
            b"%02X\0" as *const u8 as *const libc::c_char,
            digest[i_1 as usize] as libc::c_int,
        );
        i_1 = i_1.wrapping_add(1);
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
    return KAT_SUCCESS;
}
pub const SPX_N: libc::c_int = 16 as libc::c_int;
pub const SPX_FULL_HEIGHT: libc::c_int = 63 as libc::c_int;
pub const SPX_D: libc::c_int = 7 as libc::c_int;
pub const SPX_FORS_HEIGHT: libc::c_int = 12 as libc::c_int;
pub const SPX_FORS_TREES: libc::c_int = 14 as libc::c_int;
pub const SPX_WOTS_LOGW: libc::c_int = 4 as libc::c_int;
pub const SPX_WOTS_LEN1: libc::c_int = 8 as libc::c_int * SPX_N / SPX_WOTS_LOGW;
pub const SPX_WOTS_LEN2: libc::c_int = 3 as libc::c_int;
pub const SPX_WOTS_LEN: libc::c_int = SPX_WOTS_LEN1 + SPX_WOTS_LEN2;
pub const SPX_WOTS_BYTES: libc::c_int = SPX_WOTS_LEN * SPX_N;
pub const SPX_FORS_BYTES: libc::c_int =
    (SPX_FORS_HEIGHT + 1 as libc::c_int) * SPX_FORS_TREES * SPX_N;
pub const SPX_BYTES: libc::c_int =
    SPX_N + SPX_FORS_BYTES + SPX_D * SPX_WOTS_BYTES + SPX_FULL_HEIGHT * SPX_N;
pub const SPX_PK_BYTES: libc::c_int = 2 as libc::c_int * SPX_N;
pub const SPX_SK_BYTES: libc::c_int = 2 as libc::c_int * SPX_N + SPX_PK_BYTES;
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
