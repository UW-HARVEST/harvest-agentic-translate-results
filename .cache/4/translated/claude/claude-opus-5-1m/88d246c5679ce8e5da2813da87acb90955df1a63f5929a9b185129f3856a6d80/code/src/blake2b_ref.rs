//! Translation of `crypto_generichash/blake2b/ref/blake2b-ref.c`.
//!
//! Exports (after the `private/quirks.h` renaming):
//!   * `_sodium_blake2b`
//!   * `_sodium_blake2b_final`
//!   * `_sodium_blake2b_init`
//!   * `_sodium_blake2b_init_key`
//!   * `_sodium_blake2b_init_key_salt_personal`
//!   * `_sodium_blake2b_init_param`
//!   * `_sodium_blake2b_init_salt_personal`
//!   * `_sodium_blake2b_pick_best_implementation`
//!   * `_sodium_blake2b_salt_personal`
//!   * `_sodium_blake2b_update`
//!
//! The reference build defines none of `HAVE_AVX2INTRIN_H` / `HAVE_EMMINTRIN_H`
//! / `HAVE_TMMINTRIN_H` / `HAVE_SMMINTRIN_H` / `HAVE_TI_MODE`, so the SIMD
//! dispatch blocks in `blake2b_pick_best_implementation()` are preprocessed
//! away entirely and the 128-bit counter path is not taken.

use crate::common::*;
use core::ffi::{c_int, c_void};

/* enum blake2b_constant (blake2.h) */
const BLAKE2B_BLOCKBYTES: usize = 128;
const BLAKE2B_OUTBYTES: usize = 64;
const BLAKE2B_KEYBYTES: usize = 64;
const BLAKE2B_SALTBYTES: usize = 16;
const BLAKE2B_PERSONALBYTES: usize = 16;

/* `typedef struct blake2b_param_` from blake2.h -- declared inside
 * `#pragma pack(push, 1)`.  sizeof == 64, _Alignof == 1. */
#[repr(C, packed)]
pub struct blake2b_param {
    pub digest_length: u8,                    /*  1 */
    pub key_length: u8,                       /*  2 */
    pub fanout: u8,                           /*  3 */
    pub depth: u8,                            /*  4 */
    pub leaf_length: [u8; 4],                 /*  8 */
    pub node_offset: [u8; 8],                 /* 16 */
    pub node_depth: u8,                       /* 17 */
    pub inner_length: u8,                     /* 18 */
    pub reserved: [u8; 14],                   /* 32 */
    pub salt: [u8; BLAKE2B_SALTBYTES],        /* 48 */
    pub personal: [u8; BLAKE2B_PERSONALBYTES], /* 64 */
}

/* `typedef struct blake2b_state` from blake2.h -- also inside
 * `#pragma pack(push, 1)`.  sizeof == 361, _Alignof == 1, field offsets
 * h=0 t=64 f=80 buf=96 buflen=352 last_node=360. */
#[repr(C, packed)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 2 * 128],
    pub buflen: usize,
    pub last_node: u8,
}

/* CRYPTO_ALIGN(64) wrapper used for the `blake2b_state S[1]` locals. */
#[repr(C, align(64))]
struct AlignedState(blake2b_state);

/* typedef int (*blake2b_compress_fn)(blake2b_state *S,
 *                                    const uint8_t block[BLAKE2B_BLOCKBYTES]); */
type blake2b_compress_fn = unsafe extern "C" fn(*mut blake2b_state, *const u8) -> c_int;

extern "C" {
    /* blake2b-compress-ref.c */
    fn _sodium_blake2b_compress_ref(S: *mut blake2b_state, block: *const u8) -> c_int;
    /* sodium/core.c -- __attribute__((noreturn)) */
    fn sodium_misuse() -> !;
    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
}

/* static blake2b_compress_fn blake2b_compress = blake2b_compress_ref; */
static mut blake2b_compress: blake2b_compress_fn = _sodium_blake2b_compress_ref;

static blake2b_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

/* LCOV_EXCL_START */
#[inline(always)]
unsafe fn blake2b_set_lastnode(S: *mut blake2b_state) -> c_int {
    (*S).f[1] = !0u64; /* -1 */
    0
}
/* LCOV_EXCL_STOP */

#[inline(always)]
unsafe fn blake2b_is_lastblock(S: *const blake2b_state) -> c_int {
    ((*S).f[0] != 0) as c_int
}

#[inline(always)]
unsafe fn blake2b_set_lastblock(S: *mut blake2b_state) -> c_int {
    if (*S).last_node != 0 {
        blake2b_set_lastnode(S); /* LCOV_EXCL_LINE */
    }
    (*S).f[0] = !0u64; /* -1 */
    0
}

#[inline(always)]
unsafe fn blake2b_increment_counter(S: *mut blake2b_state, inc: u64) -> c_int {
    /* !HAVE_TI_MODE */
    let t0 = (*S).t[0].wrapping_add(inc);
    (*S).t[0] = t0;
    (*S).t[1] = (*S).t[1].wrapping_add((t0 < inc) as u64);
    0
}

/* Parameter-related functions */
#[inline(always)]
unsafe fn blake2b_param_set_salt(P: *mut blake2b_param, salt: *const u8) -> c_int {
    memcpy(
        core::ptr::addr_of_mut!((*P).salt) as *mut u8,
        salt,
        BLAKE2B_SALTBYTES,
    );
    0
}

#[inline(always)]
unsafe fn blake2b_param_set_personal(P: *mut blake2b_param, personal: *const u8) -> c_int {
    memcpy(
        core::ptr::addr_of_mut!((*P).personal) as *mut u8,
        personal,
        BLAKE2B_PERSONALBYTES,
    );
    0
}

#[inline(always)]
unsafe fn blake2b_init0(S: *mut blake2b_state) -> c_int {
    let mut i: usize = 0;

    while i < 8 {
        (*S).h[i] = blake2b_IV[i];
        i += 1;
    }
    /* zero everything between .t and .last_node */
    const T_OFF: usize = core::mem::offset_of!(blake2b_state, t);
    const LN_OFF: usize = core::mem::offset_of!(blake2b_state, last_node);
    memset(
        (S as *mut u8).add(T_OFF),
        0,
        LN_OFF + core::mem::size_of::<u8>() - T_OFF,
    );
    0
}

/* init xors IV with input parameter block */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_param(
    S: *mut blake2b_state,
    P: *const blake2b_param,
) -> c_int {
    let mut i: usize;
    let p: *const u8;

    /* COMPILER_ASSERT(sizeof *P == 64); */
    const _: () = assert!(core::mem::size_of::<blake2b_param>() == 64);
    blake2b_init0(S);
    p = P as *const u8;

    /* IV XOR ParamBlock */
    i = 0;
    while i < 8 {
        (*S).h[i] ^= load64_le(p.add(core::mem::size_of::<u64>() * i));
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    memset(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    memset(
        core::ptr::addr_of_mut!(P.salt) as *mut u8,
        0,
        BLAKE2B_SALTBYTES,
    );
    memset(
        core::ptr::addr_of_mut!(P.personal) as *mut u8,
        0,
        BLAKE2B_PERSONALBYTES,
    );
    _sodium_blake2b_init_param(S, &P as *const blake2b_param)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = 0;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    memset(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    if !salt.is_null() {
        blake2b_param_set_salt(&mut P as *mut blake2b_param, salt as *const u8);
    } else {
        memset(
            core::ptr::addr_of_mut!(P.salt) as *mut u8,
            0,
            BLAKE2B_SALTBYTES,
        );
    }
    if !personal.is_null() {
        blake2b_param_set_personal(&mut P as *mut blake2b_param, personal as *const u8);
    } else {
        memset(
            core::ptr::addr_of_mut!(P.personal) as *mut u8,
            0,
            BLAKE2B_PERSONALBYTES,
        );
    }
    _sodium_blake2b_init_param(S, &P as *const blake2b_param)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    memset(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    memset(
        core::ptr::addr_of_mut!(P.salt) as *mut u8,
        0,
        BLAKE2B_SALTBYTES,
    );
    memset(
        core::ptr::addr_of_mut!(P.personal) as *mut u8,
        0,
        BLAKE2B_PERSONALBYTES,
    );

    if _sodium_blake2b_init_param(S, &P as *const blake2b_param) < 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block: [u8; BLAKE2B_BLOCKBYTES] = [0; BLAKE2B_BLOCKBYTES];
        memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize); /* key and keylen cannot be 0 */
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
        /* Burn the key from stack */
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut P: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    P.digest_length = outlen;
    P.key_length = keylen;
    P.fanout = 1;
    P.depth = 1;
    store32_le(core::ptr::addr_of_mut!(P.leaf_length) as *mut u8, 0);
    store64_le(core::ptr::addr_of_mut!(P.node_offset) as *mut u8, 0);
    P.node_depth = 0;
    P.inner_length = 0;
    memset(core::ptr::addr_of_mut!(P.reserved) as *mut u8, 0, 14);
    if !salt.is_null() {
        blake2b_param_set_salt(&mut P as *mut blake2b_param, salt as *const u8);
    } else {
        memset(
            core::ptr::addr_of_mut!(P.salt) as *mut u8,
            0,
            BLAKE2B_SALTBYTES,
        );
    }
    if !personal.is_null() {
        blake2b_param_set_personal(&mut P as *mut blake2b_param, personal as *const u8);
    } else {
        memset(
            core::ptr::addr_of_mut!(P.personal) as *mut u8,
            0,
            BLAKE2B_PERSONALBYTES,
        );
    }

    if _sodium_blake2b_init_param(S, &P as *const blake2b_param) < 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    {
        let mut block: [u8; BLAKE2B_BLOCKBYTES] = [0; BLAKE2B_BLOCKBYTES];
        memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize); /* key and keylen cannot be 0 */
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
        /* Burn the key from stack */
    }
    0
}

/* inlen now in bytes */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_update(
    S: *mut blake2b_state,
    mut in_: *const u8,
    mut inlen: u64,
) -> c_int {
    while inlen > 0 {
        let left: usize = (*S).buflen;
        let fill: usize = 2 * BLAKE2B_BLOCKBYTES - left;

        if inlen > fill as u64 {
            memcpy(
                (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left),
                in_,
                fill,
            ); /* Fill buffer */
            (*S).buflen = (*S).buflen.wrapping_add(fill);
            blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
            let compress = blake2b_compress;
            compress(S, core::ptr::addr_of!((*S).buf) as *const u8); /* Compress */
            memcpy(
                core::ptr::addr_of_mut!((*S).buf) as *mut u8,
                (core::ptr::addr_of!((*S).buf) as *const u8).add(BLAKE2B_BLOCKBYTES),
                BLAKE2B_BLOCKBYTES,
            ); /* Shift buffer left */
            (*S).buflen = (*S).buflen.wrapping_sub(BLAKE2B_BLOCKBYTES);
            in_ = in_.add(fill);
            inlen -= fill as u64;
        } else
        /* inlen <= fill */
        {
            memcpy(
                (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add(left),
                in_,
                inlen as usize,
            );
            (*S).buflen = (*S).buflen.wrapping_add(inlen as usize); /* Be lazy, do not compress */
            in_ = in_.add(inlen as usize);
            inlen -= inlen;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_final(
    S: *mut blake2b_state,
    out: *mut u8,
    outlen: u8,
) -> c_int {
    let mut buffer: [u8; BLAKE2B_OUTBYTES] = [0; BLAKE2B_OUTBYTES];

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if blake2b_is_lastblock(S as *const blake2b_state) != 0 {
        return -1;
    }
    if (*S).buflen > BLAKE2B_BLOCKBYTES {
        blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
        let compress = blake2b_compress;
        compress(S, core::ptr::addr_of!((*S).buf) as *const u8);
        (*S).buflen = (*S).buflen.wrapping_sub(BLAKE2B_BLOCKBYTES);
        assert!((*S).buflen <= BLAKE2B_BLOCKBYTES);
        memcpy(
            core::ptr::addr_of_mut!((*S).buf) as *mut u8,
            (core::ptr::addr_of!((*S).buf) as *const u8).add(BLAKE2B_BLOCKBYTES),
            (*S).buflen,
        );
    }

    blake2b_increment_counter(S, (*S).buflen as u64);
    blake2b_set_lastblock(S);
    memset(
        (core::ptr::addr_of_mut!((*S).buf) as *mut u8).add((*S).buflen),
        0,
        2 * BLAKE2B_BLOCKBYTES - (*S).buflen,
    ); /* Padding */
    let compress = blake2b_compress;
    compress(S, core::ptr::addr_of!((*S).buf) as *const u8);

    /* COMPILER_ASSERT(sizeof buffer == 64U); */
    const _: () = assert!(BLAKE2B_OUTBYTES == 64);
    store64_le(buffer.as_mut_ptr().add(8 * 0), (*S).h[0]);
    store64_le(buffer.as_mut_ptr().add(8 * 1), (*S).h[1]);
    store64_le(buffer.as_mut_ptr().add(8 * 2), (*S).h[2]);
    store64_le(buffer.as_mut_ptr().add(8 * 3), (*S).h[3]);
    store64_le(buffer.as_mut_ptr().add(8 * 4), (*S).h[4]);
    store64_le(buffer.as_mut_ptr().add(8 * 5), (*S).h[5]);
    store64_le(buffer.as_mut_ptr().add(8 * 6), (*S).h[6]);
    store64_le(buffer.as_mut_ptr().add(8 * 7), (*S).h[7]);
    memcpy(out, buffer.as_ptr(), outlen as usize); /* outlen <= BLAKE2B_OUTBYTES (64) */

    sodium_memzero(
        core::ptr::addr_of_mut!((*S).h) as *mut c_void,
        core::mem::size_of::<[u64; 8]>(),
    );
    sodium_memzero(
        core::ptr::addr_of_mut!((*S).buf) as *mut c_void,
        core::mem::size_of::<[u8; 2 * 128]>(),
    );

    0
}

/* inlen, at least, should be uint64_t. Others can be size_t. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
) -> c_int {
    let mut s: AlignedState = core::mem::zeroed();
    let S: *mut blake2b_state = core::ptr::addr_of_mut!(s.0);

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key(S, outlen, key, keylen) < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else {
        if _sodium_blake2b_init(S, outlen) < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }

    _sodium_blake2b_update(S, in_ as *const u8, inlen);
    _sodium_blake2b_final(S, out, outlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_salt_personal(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut s: AlignedState = core::mem::zeroed();
    let S: *mut blake2b_state = core::ptr::addr_of_mut!(s.0);

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if out.is_null() {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal) < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    } else {
        if _sodium_blake2b_init_salt_personal(S, outlen, salt, personal) < 0 {
            sodium_misuse(); /* LCOV_EXCL_LINE */
        }
    }

    _sodium_blake2b_update(S, in_ as *const u8, inlen);
    _sodium_blake2b_final(S, out, outlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_pick_best_implementation() -> c_int {
    /* LCOV_EXCL_START */
    /* No HAVE_AVX2INTRIN_H / HAVE_EMMINTRIN_H / HAVE_TMMINTRIN_H /
     * HAVE_SMMINTRIN_H in the reference build: all SIMD blocks vanish. */
    blake2b_compress = _sodium_blake2b_compress_ref;

    0
    /* LCOV_EXCL_STOP */
}
