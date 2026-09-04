//! Translation of the BLAKE2b reference implementation and the
//! `crypto_generichash_blake2b` front-end.
//!
//! Combines:
//!   - crypto_generichash/blake2b/ref/blake2b-ref.c
//!   - crypto_generichash/blake2b/ref/blake2b-compress-ref.c
//!   - crypto_generichash/blake2b/ref/generichash_blake2b.c
//!   - crypto_generichash/blake2b/generichash_blake2.c
//! plus headers blake2.h and include/sodium/crypto_generichash_blake2b.h.

use core::ffi::{c_int, c_uchar, c_void};
use core::mem::offset_of;

use crate::common::{load64_le, memcpy, memset, rotr64, store32_le, store64_le};
use crate::randombytes::randombytes_buf;
use crate::sodium_core::sodium_misuse;
use crate::sodium_utils::sodium_memzero;

/* ---- enum blake2b_constant ---- */
pub const BLAKE2B_BLOCKBYTES: usize = 128;
pub const BLAKE2B_OUTBYTES: usize = 64;
pub const BLAKE2B_KEYBYTES: usize = 64;
pub const BLAKE2B_SALTBYTES: usize = 16;
pub const BLAKE2B_PERSONALBYTES: usize = 16;

/* ---- public header constants (crypto_generichash_blake2b.h) ---- */
pub const crypto_generichash_blake2b_BYTES_MIN: usize = 16;
pub const crypto_generichash_blake2b_BYTES_MAX: usize = 64;
pub const crypto_generichash_blake2b_BYTES: usize = 32;
pub const crypto_generichash_blake2b_KEYBYTES_MIN: usize = 16;
pub const crypto_generichash_blake2b_KEYBYTES_MAX: usize = 64;
pub const crypto_generichash_blake2b_KEYBYTES: usize = 32;
pub const crypto_generichash_blake2b_SALTBYTES: usize = 16;
pub const crypto_generichash_blake2b_PERSONALBYTES: usize = 16;

/* ---- structs (both under `#pragma pack(push, 1)`) ---- */

#[repr(C, packed)]
pub struct blake2b_param {
    pub digest_length: u8,          /*  1 */
    pub key_length: u8,             /*  2 */
    pub fanout: u8,                 /*  3 */
    pub depth: u8,                  /*  4 */
    pub leaf_length: [u8; 4],       /*  8 */
    pub node_offset: [u8; 8],       /* 16 */
    pub node_depth: u8,             /* 17 */
    pub inner_length: u8,           /* 18 */
    pub reserved: [u8; 14],         /* 32 */
    pub salt: [u8; BLAKE2B_SALTBYTES], /* 48 */
    pub personal: [u8; BLAKE2B_PERSONALBYTES], /* 64 */
}

#[repr(C, packed)]
pub struct blake2b_state {
    pub h: [u64; 8],
    pub t: [u64; 2],
    pub f: [u64; 2],
    pub buf: [u8; 2 * 128],
    pub buflen: usize,
    pub last_node: u8,
}

/* ---- public state type (crypto_generichash_blake2b.h) ----
 * typedef struct CRYPTO_ALIGN(64) { unsigned char opaque[384]; }
 * declared under `#pragma pack(push, 1)`. The only member is a byte array
 * (1-byte alignment), so `pack(1)` is a no-op and the effective layout is
 * 384 bytes with 64-byte alignment. */
#[repr(C, align(64))]
pub struct crypto_generichash_blake2b_state {
    pub opaque: [c_uchar; 384],
}

/* ---- compress function pointer type ---- */
pub type blake2b_compress_fn =
    unsafe extern "C" fn(S: *mut blake2b_state, block: *const u8) -> c_int;

/* =====================================================================
 * blake2b-compress-ref.c
 * ===================================================================== */

// CRYPTO_ALIGN(64) static const uint64_t blake2b_IV[8]
#[repr(align(64))]
struct Blake2bIvAligned([u64; 8]);

static BLAKE2B_IV_COMPRESS: Blake2bIvAligned = Blake2bIvAligned([
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
]);

static blake2b_sigma: [[u8; 16]; 12] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_compress_ref(
    S: *mut blake2b_state,
    block: *const u8,
) -> c_int {
    let mut m: [u64; 16] = [0; 16];
    let mut v: [u64; 16] = [0; 16];
    let iv = &BLAKE2B_IV_COMPRESS.0;

    let mut i: i32 = 0;
    while i < 16 {
        m[i as usize] = load64_le(block.add((i as usize) * core::mem::size_of::<u64>()));
        i += 1;
    }
    i = 0;
    while i < 8 {
        v[i as usize] = (*S).h[i as usize];
        i += 1;
    }
    v[8] = iv[0];
    v[9] = iv[1];
    v[10] = iv[2];
    v[11] = iv[3];
    v[12] = (*S).t[0] ^ iv[4];
    v[13] = (*S).t[1] ^ iv[5];
    v[14] = (*S).f[0] ^ iv[6];
    v[15] = (*S).f[1] ^ iv[7];

    // G(r, i, a, b, c, d)
    macro_rules! g {
        ($r:expr, $i:expr, $a:expr, $b:expr, $c:expr, $d:expr) => {{
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[blake2b_sigma[$r][2 * $i + 0] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 32);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 24);
            v[$a] = v[$a]
                .wrapping_add(v[$b])
                .wrapping_add(m[blake2b_sigma[$r][2 * $i + 1] as usize]);
            v[$d] = rotr64(v[$d] ^ v[$a], 16);
            v[$c] = v[$c].wrapping_add(v[$d]);
            v[$b] = rotr64(v[$b] ^ v[$c], 63);
        }};
    }
    macro_rules! round {
        ($r:expr) => {{
            g!($r, 0, 0, 4, 8, 12);
            g!($r, 1, 1, 5, 9, 13);
            g!($r, 2, 2, 6, 10, 14);
            g!($r, 3, 3, 7, 11, 15);
            g!($r, 4, 0, 5, 10, 15);
            g!($r, 5, 1, 6, 11, 12);
            g!($r, 6, 2, 7, 8, 13);
            g!($r, 7, 3, 4, 9, 14);
        }};
    }
    round!(0);
    round!(1);
    round!(2);
    round!(3);
    round!(4);
    round!(5);
    round!(6);
    round!(7);
    round!(8);
    round!(9);
    round!(10);
    round!(11);

    i = 0;
    while i < 8 {
        let idx = i as usize;
        (*S).h[idx] = (*S).h[idx] ^ v[idx] ^ v[idx + 8];
        i += 1;
    }
    0
}

/* =====================================================================
 * blake2b-ref.c
 * ===================================================================== */

// static blake2b_compress_fn blake2b_compress = blake2b_compress_ref;
struct CompressFnCell(core::cell::UnsafeCell<blake2b_compress_fn>);
unsafe impl Sync for CompressFnCell {}

static blake2b_compress: CompressFnCell =
    CompressFnCell(core::cell::UnsafeCell::new(_sodium_blake2b_compress_ref));

#[inline(always)]
unsafe fn call_compress(S: *mut blake2b_state, block: *const u8) -> c_int {
    (*blake2b_compress.0.get())(S, block)
}

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

#[inline]
unsafe fn blake2b_set_lastnode(S: *mut blake2b_state) -> c_int {
    (*S).f[1] = -1i64 as u64;
    0
}

#[inline]
unsafe fn blake2b_is_lastblock(S: *const blake2b_state) -> c_int {
    ((*S).f[0] != 0) as c_int
}

#[inline]
unsafe fn blake2b_set_lastblock(S: *mut blake2b_state) -> c_int {
    if (*S).last_node != 0 {
        blake2b_set_lastnode(S);
    }
    (*S).f[0] = -1i64 as u64;
    0
}

#[inline]
unsafe fn blake2b_increment_counter(S: *mut blake2b_state, inc: u64) -> c_int {
    (*S).t[0] = (*S).t[0].wrapping_add(inc);
    (*S).t[1] = (*S).t[1].wrapping_add(((*S).t[0] < inc) as u64);
    0
}

#[inline]
unsafe fn blake2b_param_set_salt(P: *mut blake2b_param, salt: *const u8) -> c_int {
    memcpy(
        (*P).salt.as_mut_ptr(),
        salt,
        BLAKE2B_SALTBYTES,
    );
    0
}

#[inline]
unsafe fn blake2b_param_set_personal(P: *mut blake2b_param, personal: *const u8) -> c_int {
    memcpy(
        (*P).personal.as_mut_ptr(),
        personal,
        BLAKE2B_PERSONALBYTES,
    );
    0
}

#[inline]
unsafe fn blake2b_init0(S: *mut blake2b_state) -> c_int {
    let mut i: i32 = 0;
    while i < 8 {
        (*S).h[i as usize] = blake2b_IV[i as usize];
        i += 1;
    }
    /* zero everything between .t and .last_node */
    let base = S as *mut u8;
    let start = offset_of!(blake2b_state, t);
    let end = offset_of!(blake2b_state, last_node) + core::mem::size_of::<u8>();
    memset(base.add(start), 0, end - start);
    0
}

/* init xors IV with input parameter block */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_param(
    S: *mut blake2b_state,
    P: *const blake2b_param,
) -> c_int {
    // COMPILER_ASSERT(sizeof *P == 64);
    blake2b_init0(S);
    let p = P as *const u8;

    let mut i: usize = 0;
    while i < 8 {
        (*S).h[i] ^= load64_le(p.add(core::mem::size_of::<u64>() * i));
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init(S: *mut blake2b_state, outlen: u8) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();
    let P: *mut blake2b_param = &mut p;

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        sodium_misuse();
    }
    (*P).digest_length = outlen;
    (*P).key_length = 0;
    (*P).fanout = 1;
    (*P).depth = 1;
    store32_le((*P).leaf_length.as_mut_ptr(), 0);
    store64_le((*P).node_offset.as_mut_ptr(), 0);
    (*P).node_depth = 0;
    (*P).inner_length = 0;
    memset((*P).reserved.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).reserved));
    memset((*P).salt.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).salt));
    memset((*P).personal.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).personal));
    _sodium_blake2b_init_param(S, P)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    S: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();
    let P: *mut blake2b_param = &mut p;

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        sodium_misuse();
    }
    (*P).digest_length = outlen;
    (*P).key_length = 0;
    (*P).fanout = 1;
    (*P).depth = 1;
    store32_le((*P).leaf_length.as_mut_ptr(), 0);
    store64_le((*P).node_offset.as_mut_ptr(), 0);
    (*P).node_depth = 0;
    (*P).inner_length = 0;
    memset((*P).reserved.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).reserved));
    if !salt.is_null() {
        blake2b_param_set_salt(P, salt as *const u8);
    } else {
        memset((*P).salt.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).salt));
    }
    if !personal.is_null() {
        blake2b_param_set_personal(P, personal as *const u8);
    } else {
        memset((*P).personal.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).personal));
    }
    _sodium_blake2b_init_param(S, P)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    S: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();
    let P: *mut blake2b_param = &mut p;

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    (*P).digest_length = outlen;
    (*P).key_length = keylen;
    (*P).fanout = 1;
    (*P).depth = 1;
    store32_le((*P).leaf_length.as_mut_ptr(), 0);
    store64_le((*P).node_offset.as_mut_ptr(), 0);
    (*P).node_depth = 0;
    (*P).inner_length = 0;
    memset((*P).reserved.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).reserved));
    memset((*P).salt.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).salt));
    memset((*P).personal.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).personal));

    if _sodium_blake2b_init_param(S, P) < 0 {
        sodium_misuse();
    }
    {
        let mut block: [u8; BLAKE2B_BLOCKBYTES] = [0; BLAKE2B_BLOCKBYTES];
        memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize);
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
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
    let mut p: blake2b_param = core::mem::zeroed();
    let P: *mut blake2b_param = &mut p;

    if (outlen == 0) || (outlen as usize > BLAKE2B_OUTBYTES) {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    (*P).digest_length = outlen;
    (*P).key_length = keylen;
    (*P).fanout = 1;
    (*P).depth = 1;
    store32_le((*P).leaf_length.as_mut_ptr(), 0);
    store64_le((*P).node_offset.as_mut_ptr(), 0);
    (*P).node_depth = 0;
    (*P).inner_length = 0;
    memset((*P).reserved.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).reserved));
    if !salt.is_null() {
        blake2b_param_set_salt(P, salt as *const u8);
    } else {
        memset((*P).salt.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).salt));
    }
    if !personal.is_null() {
        blake2b_param_set_personal(P, personal as *const u8);
    } else {
        memset((*P).personal.as_mut_ptr(), 0, core::mem::size_of_val(&(*P).personal));
    }

    if _sodium_blake2b_init_param(S, P) < 0 {
        sodium_misuse();
    }
    {
        let mut block: [u8; BLAKE2B_BLOCKBYTES] = [0; BLAKE2B_BLOCKBYTES];
        memset(block.as_mut_ptr(), 0, BLAKE2B_BLOCKBYTES);
        memcpy(block.as_mut_ptr(), key as *const u8, keylen as usize);
        _sodium_blake2b_update(S, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
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
            memcpy((*S).buf.as_mut_ptr().add(left), in_, fill); /* Fill buffer */
            (*S).buflen += fill;
            blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
            call_compress(S, (*S).buf.as_ptr()); /* Compress */
            memcpy(
                (*S).buf.as_mut_ptr(),
                (*S).buf.as_ptr().add(BLAKE2B_BLOCKBYTES),
                BLAKE2B_BLOCKBYTES,
            ); /* Shift buffer left */
            (*S).buflen -= BLAKE2B_BLOCKBYTES;
            in_ = in_.add(fill);
            inlen -= fill as u64;
        } else
        /* inlen <= fill */
        {
            memcpy((*S).buf.as_mut_ptr().add(left), in_, inlen as usize);
            (*S).buflen += inlen as usize; /* Be lazy, do not compress */
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
    let mut buffer: [c_uchar; BLAKE2B_OUTBYTES] = [0; BLAKE2B_OUTBYTES];

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if blake2b_is_lastblock(S) != 0 {
        return -1;
    }
    if (*S).buflen > BLAKE2B_BLOCKBYTES {
        blake2b_increment_counter(S, BLAKE2B_BLOCKBYTES as u64);
        call_compress(S, (*S).buf.as_ptr());
        (*S).buflen -= BLAKE2B_BLOCKBYTES;
        // assert(S->buflen <= BLAKE2B_BLOCKBYTES);
        memcpy(
            (*S).buf.as_mut_ptr(),
            (*S).buf.as_ptr().add(BLAKE2B_BLOCKBYTES),
            (*S).buflen,
        );
    }

    blake2b_increment_counter(S, (*S).buflen as u64);
    blake2b_set_lastblock(S);
    memset(
        (*S).buf.as_mut_ptr().add((*S).buflen),
        0,
        2 * BLAKE2B_BLOCKBYTES - (*S).buflen,
    ); /* Padding */
    call_compress(S, (*S).buf.as_ptr());

    store64_le(buffer.as_mut_ptr().add(8 * 0), (*S).h[0]);
    store64_le(buffer.as_mut_ptr().add(8 * 1), (*S).h[1]);
    store64_le(buffer.as_mut_ptr().add(8 * 2), (*S).h[2]);
    store64_le(buffer.as_mut_ptr().add(8 * 3), (*S).h[3]);
    store64_le(buffer.as_mut_ptr().add(8 * 4), (*S).h[4]);
    store64_le(buffer.as_mut_ptr().add(8 * 5), (*S).h[5]);
    store64_le(buffer.as_mut_ptr().add(8 * 6), (*S).h[6]);
    store64_le(buffer.as_mut_ptr().add(8 * 7), (*S).h[7]);
    memcpy(out, buffer.as_ptr(), outlen as usize);

    sodium_memzero(
        (&raw mut (*S).h) as *mut c_void,
        core::mem::size_of::<[u64; 8]>(),
    );
    sodium_memzero(
        (&raw mut (*S).buf) as *mut c_void,
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
    let mut s: blake2b_state = core::mem::zeroed();
    let S: *mut blake2b_state = &mut s;

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse();
    }
    if out.is_null() {
        sodium_misuse();
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse();
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key(S, outlen, key, keylen) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init(S, outlen) < 0 {
        sodium_misuse();
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
    let mut s: blake2b_state = core::mem::zeroed();
    let S: *mut blake2b_state = &mut s;

    /* Verify parameters */
    if in_.is_null() && inlen > 0 {
        sodium_misuse();
    }
    if out.is_null() {
        sodium_misuse();
    }
    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() && keylen > 0 {
        sodium_misuse();
    }
    if keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    if keylen > 0 {
        if _sodium_blake2b_init_key_salt_personal(S, outlen, key, keylen, salt, personal) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init_salt_personal(S, outlen, salt, personal) < 0 {
        sodium_misuse();
    }

    _sodium_blake2b_update(S, in_ as *const u8, inlen);
    _sodium_blake2b_final(S, out, outlen);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_blake2b_pick_best_implementation() -> c_int {
    /* SIMD variants are compiled out; sodium_runtime_has_*() return 0. */
    *blake2b_compress.0.get() = _sodium_blake2b_compress_ref;
    0
}

/* =====================================================================
 * generichash_blake2b.c
 * ===================================================================== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut c_uchar,
    outlen: usize,
    in_: *const c_uchar,
    inlen: u64,
    key: *const c_uchar,
    keylen: usize,
) -> c_int {
    if outlen == 0
        || outlen > BLAKE2B_OUTBYTES
        || keylen > BLAKE2B_KEYBYTES
        || inlen > u64::MAX
    {
        return -1;
    }
    // assert(outlen <= UINT8_MAX); assert(keylen <= UINT8_MAX);

    _sodium_blake2b(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut c_uchar,
    outlen: usize,
    in_: *const c_uchar,
    inlen: u64,
    key: *const c_uchar,
    keylen: usize,
    salt: *const c_uchar,
    personal: *const c_uchar,
) -> c_int {
    if outlen == 0
        || outlen > BLAKE2B_OUTBYTES
        || keylen > BLAKE2B_KEYBYTES
        || inlen > u64::MAX
    {
        return -1;
    }
    _sodium_blake2b_salt_personal(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init(
    state: *mut crypto_generichash_blake2b_state,
    key: *const c_uchar,
    keylen: usize,
    outlen: usize,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    // COMPILER_ASSERT(sizeof(blake2b_state) <= sizeof *state);
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init(state as *mut c_void as *mut blake2b_state, outlen as u8) != 0 {
            return -1;
        }
    } else if _sodium_blake2b_init_key(
        state as *mut c_void as *mut blake2b_state,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
    ) != 0
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_init_salt_personal(
    state: *mut crypto_generichash_blake2b_state,
    key: *const c_uchar,
    keylen: usize,
    outlen: usize,
    salt: *const c_uchar,
    personal: *const c_uchar,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init_salt_personal(
            state as *mut c_void as *mut blake2b_state,
            outlen as u8,
            salt as *const c_void,
            personal as *const c_void,
        ) != 0
        {
            return -1;
        }
    } else if _sodium_blake2b_init_key_salt_personal(
        state as *mut c_void as *mut blake2b_state,
        outlen as u8,
        key as *const c_void,
        keylen as u8,
        salt as *const c_void,
        personal as *const c_void,
    ) != 0
    {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    in_: *const c_uchar,
    inlen: u64,
) -> c_int {
    _sodium_blake2b_update(
        state as *mut c_void as *mut blake2b_state,
        in_ as *const u8,
        inlen,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut c_uchar,
    outlen: usize,
) -> c_int {
    // assert(outlen <= UINT8_MAX);
    _sodium_blake2b_final(
        state as *mut c_void as *mut blake2b_state,
        out as *mut u8,
        outlen as u8,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int {
    _sodium_blake2b_pick_best_implementation()
}

/* =====================================================================
 * generichash_blake2.c
 * ===================================================================== */

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes_min() -> usize {
    crypto_generichash_blake2b_BYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes_max() -> usize {
    crypto_generichash_blake2b_BYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_bytes() -> usize {
    crypto_generichash_blake2b_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes_min() -> usize {
    crypto_generichash_blake2b_KEYBYTES_MIN
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes_max() -> usize {
    crypto_generichash_blake2b_KEYBYTES_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_keybytes() -> usize {
    crypto_generichash_blake2b_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_saltbytes() -> usize {
    crypto_generichash_blake2b_SALTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_personalbytes() -> usize {
    crypto_generichash_blake2b_PERSONALBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_generichash_blake2b_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_blake2b_state>() + 63usize) & !63usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_generichash_blake2b_keygen(k: *mut c_uchar) {
    randombytes_buf(
        k as *mut c_void,
        crypto_generichash_blake2b_KEYBYTES,
    );
}
