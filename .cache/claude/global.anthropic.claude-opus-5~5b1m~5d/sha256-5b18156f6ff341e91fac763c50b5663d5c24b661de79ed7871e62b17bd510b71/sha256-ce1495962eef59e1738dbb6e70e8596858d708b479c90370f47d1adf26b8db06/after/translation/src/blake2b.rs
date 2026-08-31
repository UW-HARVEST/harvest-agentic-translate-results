//! Translation of the BLAKE2b / `crypto_generichash` family:
//!
//! * `crypto_generichash/blake2b/ref/blake2b-ref.c`
//! * `crypto_generichash/blake2b/ref/blake2b-compress-ref.c`
//! * `crypto_generichash/blake2b/ref/generichash_blake2b.c`
//! * `crypto_generichash/blake2b/generichash_blake2.c`
//! * `crypto_generichash/crypto_generichash.c`
//! * `crypto_pwhash/argon2/blake2b-long.c`
//!
//! The reference build has no SIMD feature macros defined, so
//! `blake2b_pick_best_implementation()` always selects the portable
//! reference compression function (`blake2b_compress_ref`), matching this
//! translation exactly.

use core::ffi::{c_char, c_int, c_void};

use crate::common::{load64_le, rotr64, store32_le, store64_le};
use crate::csys::{memcpy, memset};
use crate::types::{blake2b_state, crypto_generichash_blake2b_state};

extern "C" {
    fn sodium_misuse() -> !;
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

// ===========================================================================
// crypto_generichash/blake2b/ref/blake2.h
// ===========================================================================

const BLAKE2B_BLOCKBYTES: usize = 128;
const BLAKE2B_OUTBYTES: usize = 64;
const BLAKE2B_KEYBYTES: usize = 64;
const BLAKE2B_SALTBYTES: usize = 16;
const BLAKE2B_PERSONALBYTES: usize = 16;

/// `blake2b_param` from `blake2.h`. Every field is `uint8_t` or an array of
/// `uint8_t`, so `repr(C, packed)` has identical layout to the C
/// `#pragma pack(push, 1)` struct, and — because all fields already have
/// natural alignment 1 — ordinary field access is safe (no unaligned
/// reference is ever created).
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct blake2b_param {
    digest_length: u8,
    key_length: u8,
    fanout: u8,
    depth: u8,
    leaf_length: [u8; 4],
    node_offset: [u8; 8],
    node_depth: u8,
    inner_length: u8,
    reserved: [u8; 14],
    salt: [u8; BLAKE2B_SALTBYTES],
    personal: [u8; BLAKE2B_PERSONALBYTES],
}

const _: () = assert!(core::mem::size_of::<blake2b_param>() == 64);

// ===========================================================================
// crypto_generichash/blake2b/ref/blake2b-compress-ref.c
// ===========================================================================

static BLAKE2B_IV: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

static BLAKE2B_SIGMA: [[u8; 16]; 12] = [
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

#[inline(always)]
fn blake2b_g(v: &mut [u64; 16], m: &[u64; 16], sigma_r: &[u8; 16], idx: usize, a: usize, b: usize, c: usize, d: usize) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma_r[2 * idx] as usize]);
    v[d] = rotr64(v[d] ^ v[a], 32);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 24);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(m[sigma_r[2 * idx + 1] as usize]);
    v[d] = rotr64(v[d] ^ v[a], 16);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotr64(v[b] ^ v[c], 63);
}

#[inline(always)]
fn blake2b_round(r: usize, v: &mut [u64; 16], m: &[u64; 16]) {
    let sigma_r = &BLAKE2B_SIGMA[r];
    blake2b_g(v, m, sigma_r, 0, 0, 4, 8, 12);
    blake2b_g(v, m, sigma_r, 1, 1, 5, 9, 13);
    blake2b_g(v, m, sigma_r, 2, 2, 6, 10, 14);
    blake2b_g(v, m, sigma_r, 3, 3, 7, 11, 15);
    blake2b_g(v, m, sigma_r, 4, 0, 5, 10, 15);
    blake2b_g(v, m, sigma_r, 5, 1, 6, 11, 12);
    blake2b_g(v, m, sigma_r, 6, 2, 7, 8, 13);
    blake2b_g(v, m, sigma_r, 7, 3, 4, 9, 14);
}

/// `blake2b_compress_ref` -> `_sodium_blake2b_compress_ref` (`private/quirks.h`).
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_compress_ref(s: *mut blake2b_state, block: *const u8) -> c_int {
    let mut m = [0u64; 16];
    let mut v = [0u64; 16];

    for i in 0..16 {
        m[i] = load64_le(block.add(i * core::mem::size_of::<u64>()));
    }
    for i in 0..8 {
        v[i] = (*s).h[i];
    }
    v[8] = BLAKE2B_IV[0];
    v[9] = BLAKE2B_IV[1];
    v[10] = BLAKE2B_IV[2];
    v[11] = BLAKE2B_IV[3];
    v[12] = (*s).t[0] ^ BLAKE2B_IV[4];
    v[13] = (*s).t[1] ^ BLAKE2B_IV[5];
    v[14] = (*s).f[0] ^ BLAKE2B_IV[6];
    v[15] = (*s).f[1] ^ BLAKE2B_IV[7];

    blake2b_round(0, &mut v, &m);
    blake2b_round(1, &mut v, &m);
    blake2b_round(2, &mut v, &m);
    blake2b_round(3, &mut v, &m);
    blake2b_round(4, &mut v, &m);
    blake2b_round(5, &mut v, &m);
    blake2b_round(6, &mut v, &m);
    blake2b_round(7, &mut v, &m);
    blake2b_round(8, &mut v, &m);
    blake2b_round(9, &mut v, &m);
    blake2b_round(10, &mut v, &m);
    blake2b_round(11, &mut v, &m);

    for i in 0..8 {
        (*s).h[i] = (*s).h[i] ^ v[i] ^ v[i + 8];
    }

    0
}

// ===========================================================================
// crypto_generichash/blake2b/ref/blake2b-ref.c
// ===========================================================================

type Blake2bCompressFn = unsafe extern "C" fn(*mut blake2b_state, *const u8) -> c_int;

static mut BLAKE2B_COMPRESS: Blake2bCompressFn = _sodium_blake2b_compress_ref;

#[inline(always)]
unsafe fn blake2b_set_lastnode(s: *mut blake2b_state) -> c_int {
    (*s).f[1] = u64::MAX;
    0
}

#[inline(always)]
unsafe fn blake2b_is_lastblock(s: *const blake2b_state) -> c_int {
    ((*s).f[0] != 0) as c_int
}

#[inline(always)]
unsafe fn blake2b_set_lastblock(s: *mut blake2b_state) -> c_int {
    if (*s).last_node != 0 {
        blake2b_set_lastnode(s);
    }
    (*s).f[0] = u64::MAX;
    0
}

#[inline(always)]
unsafe fn blake2b_increment_counter(s: *mut blake2b_state, inc: u64) -> c_int {
    (*s).t[0] = (*s).t[0].wrapping_add(inc);
    let carry = ((*s).t[0] < inc) as u64;
    (*s).t[1] = (*s).t[1].wrapping_add(carry);
    0
}

#[inline(always)]
unsafe fn blake2b_param_set_salt(p: *mut blake2b_param, salt: *const u8) -> c_int {
    memcpy((*p).salt.as_mut_ptr() as *mut c_void, salt as *const c_void, BLAKE2B_SALTBYTES);
    0
}

#[inline(always)]
unsafe fn blake2b_param_set_personal(p: *mut blake2b_param, personal: *const u8) -> c_int {
    memcpy(
        (*p).personal.as_mut_ptr() as *mut c_void,
        personal as *const c_void,
        BLAKE2B_PERSONALBYTES,
    );
    0
}

#[inline(always)]
unsafe fn blake2b_init0(s: *mut blake2b_state) -> c_int {
    for i in 0..8 {
        (*s).h[i] = BLAKE2B_IV[i];
    }
    // memset(&S->t, 0, offsetof(last_node) + sizeof(last_node) - offsetof(t))
    // i.e. zero t, f, buf, buflen and last_node.
    (*s).t = [0; 2];
    (*s).f = [0; 2];
    for i in 0..(2 * BLAKE2B_BLOCKBYTES) {
        (*s).buf[i] = 0;
    }
    (*s).buflen = 0;
    (*s).last_node = 0;
    0
}

/// `blake2b_init_param` -> `_sodium_blake2b_init_param`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_init_param(s: *mut blake2b_state, p: *const blake2b_param) -> c_int {
    blake2b_init0(s);
    let pp = p as *const u8;
    for i in 0..8 {
        (*s).h[i] ^= load64_le(pp.add(core::mem::size_of::<u64>() * i));
    }
    0
}

/// `blake2b_init` -> `_sodium_blake2b_init`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_init(s: *mut blake2b_state, outlen: u8) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    p.digest_length = outlen;
    p.key_length = 0;
    p.fanout = 1;
    p.depth = 1;
    store32_le(p.leaf_length.as_mut_ptr(), 0);
    store64_le(p.node_offset.as_mut_ptr(), 0);
    p.node_depth = 0;
    p.inner_length = 0;
    // p.reserved, p.salt, p.personal are already zero from `zeroed()`.
    _sodium_blake2b_init_param(s, &p)
}

/// `blake2b_init_salt_personal` -> `_sodium_blake2b_init_salt_personal`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_init_salt_personal(
    s: *mut blake2b_state,
    outlen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    p.digest_length = outlen;
    p.key_length = 0;
    p.fanout = 1;
    p.depth = 1;
    store32_le(p.leaf_length.as_mut_ptr(), 0);
    store64_le(p.node_offset.as_mut_ptr(), 0);
    p.node_depth = 0;
    p.inner_length = 0;
    if !salt.is_null() {
        blake2b_param_set_salt(&mut p, salt as *const u8);
    }
    if !personal.is_null() {
        blake2b_param_set_personal(&mut p, personal as *const u8);
    }
    _sodium_blake2b_init_param(s, &p)
}

/// `blake2b_init_key` -> `_sodium_blake2b_init_key`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_init_key(
    s: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    p.digest_length = outlen;
    p.key_length = keylen;
    p.fanout = 1;
    p.depth = 1;
    store32_le(p.leaf_length.as_mut_ptr(), 0);
    store64_le(p.node_offset.as_mut_ptr(), 0);
    p.node_depth = 0;
    p.inner_length = 0;

    if _sodium_blake2b_init_param(s, &p) < 0 {
        sodium_misuse();
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        memcpy(block.as_mut_ptr() as *mut c_void, key, keylen as usize);
        _sodium_blake2b_update(s, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

/// `blake2b_init_key_salt_personal` -> `_sodium_blake2b_init_key_salt_personal`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_init_key_salt_personal(
    s: *mut blake2b_state,
    outlen: u8,
    key: *const c_void,
    keylen: u8,
    salt: *const c_void,
    personal: *const c_void,
) -> c_int {
    let mut p: blake2b_param = core::mem::zeroed();

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if key.is_null() || keylen == 0 || keylen as usize > BLAKE2B_KEYBYTES {
        sodium_misuse();
    }
    p.digest_length = outlen;
    p.key_length = keylen;
    p.fanout = 1;
    p.depth = 1;
    store32_le(p.leaf_length.as_mut_ptr(), 0);
    store64_le(p.node_offset.as_mut_ptr(), 0);
    p.node_depth = 0;
    p.inner_length = 0;
    if !salt.is_null() {
        blake2b_param_set_salt(&mut p, salt as *const u8);
    }
    if !personal.is_null() {
        blake2b_param_set_personal(&mut p, personal as *const u8);
    }

    if _sodium_blake2b_init_param(s, &p) < 0 {
        sodium_misuse();
    }
    {
        let mut block = [0u8; BLAKE2B_BLOCKBYTES];
        memcpy(block.as_mut_ptr() as *mut c_void, key, keylen as usize);
        _sodium_blake2b_update(s, block.as_ptr(), BLAKE2B_BLOCKBYTES as u64);
        sodium_memzero(block.as_mut_ptr() as *mut c_void, BLAKE2B_BLOCKBYTES);
    }
    0
}

/// `blake2b_update` -> `_sodium_blake2b_update`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_update(s: *mut blake2b_state, in_: *const u8, mut inlen: u64) -> c_int {
    let mut inp = in_;

    while inlen > 0 {
        let left = (*s).buflen;
        let fill = 2 * BLAKE2B_BLOCKBYTES - left;

        if inlen > fill as u64 {
            memcpy(
                (*s).buf.as_mut_ptr().add(left) as *mut c_void,
                inp as *const c_void,
                fill,
            );
            (*s).buflen += fill;
            blake2b_increment_counter(s, BLAKE2B_BLOCKBYTES as u64);
            BLAKE2B_COMPRESS(s, (*s).buf.as_ptr());
            memcpy(
                (*s).buf.as_mut_ptr() as *mut c_void,
                (*s).buf.as_ptr().add(BLAKE2B_BLOCKBYTES) as *const c_void,
                BLAKE2B_BLOCKBYTES,
            );
            (*s).buflen -= BLAKE2B_BLOCKBYTES;
            inp = inp.add(fill);
            inlen -= fill as u64;
        } else {
            memcpy(
                (*s).buf.as_mut_ptr().add(left) as *mut c_void,
                inp as *const c_void,
                inlen as usize,
            );
            (*s).buflen += inlen as usize;
            inp = inp.add(inlen as usize);
            inlen -= inlen;
        }
    }

    0
}

/// `blake2b_final` -> `_sodium_blake2b_final`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_final(s: *mut blake2b_state, out: *mut u8, outlen: u8) -> c_int {
    let mut buffer = [0u8; BLAKE2B_OUTBYTES];

    if outlen == 0 || outlen as usize > BLAKE2B_OUTBYTES {
        sodium_misuse();
    }
    if blake2b_is_lastblock(s) != 0 {
        return -1;
    }
    if (*s).buflen > BLAKE2B_BLOCKBYTES {
        blake2b_increment_counter(s, BLAKE2B_BLOCKBYTES as u64);
        BLAKE2B_COMPRESS(s, (*s).buf.as_ptr());
        (*s).buflen -= BLAKE2B_BLOCKBYTES;
        memcpy(
            (*s).buf.as_mut_ptr() as *mut c_void,
            (*s).buf.as_ptr().add(BLAKE2B_BLOCKBYTES) as *const c_void,
            (*s).buflen,
        );
    }

    blake2b_increment_counter(s, (*s).buflen as u64);
    blake2b_set_lastblock(s);
    memset(
        (*s).buf.as_mut_ptr().add((*s).buflen) as *mut c_void,
        0,
        2 * BLAKE2B_BLOCKBYTES - (*s).buflen,
    );
    BLAKE2B_COMPRESS(s, (*s).buf.as_ptr());

    for i in 0..8 {
        store64_le(buffer.as_mut_ptr().add(8 * i), (*s).h[i]);
    }
    memcpy(out as *mut c_void, buffer.as_ptr() as *const c_void, outlen as usize);

    sodium_memzero((*s).h.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&(*s).h));
    sodium_memzero((*s).buf.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&(*s).buf));

    0
}

/// `blake2b` -> `_sodium_blake2b`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b(
    out: *mut u8,
    in_: *const c_void,
    key: *const c_void,
    outlen: u8,
    inlen: u64,
    keylen: u8,
) -> c_int {
    let mut s: blake2b_state = core::mem::zeroed();

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
        if _sodium_blake2b_init_key(&mut s, outlen, key, keylen) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init(&mut s, outlen) < 0 {
        sodium_misuse();
    }

    _sodium_blake2b_update(&mut s, in_ as *const u8, inlen);
    _sodium_blake2b_final(&mut s, out, outlen);
    0
}

/// `blake2b_salt_personal` -> `_sodium_blake2b_salt_personal`.
#[no_mangle]
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
        if _sodium_blake2b_init_key_salt_personal(&mut s, outlen, key, keylen, salt, personal) < 0 {
            sodium_misuse();
        }
    } else if _sodium_blake2b_init_salt_personal(&mut s, outlen, salt, personal) < 0 {
        sodium_misuse();
    }

    _sodium_blake2b_update(&mut s, in_ as *const u8, inlen);
    _sodium_blake2b_final(&mut s, out, outlen);
    0
}

/// `blake2b_pick_best_implementation` -> `_sodium_blake2b_pick_best_implementation`.
///
/// No SIMD feature macros are defined in the reference build, so this
/// always selects `blake2b_compress_ref`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_pick_best_implementation() -> c_int {
    BLAKE2B_COMPRESS = _sodium_blake2b_compress_ref;
    0
}

// ===========================================================================
// crypto_generichash/blake2b/ref/generichash_blake2b.c
// ===========================================================================

const CRYPTO_GENERICHASH_BLAKE2B_BYTES_MIN: usize = 16;
const CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX: usize = 64;
const CRYPTO_GENERICHASH_BLAKE2B_BYTES: usize = 32;
const CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MIN: usize = 16;
const CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MAX: usize = 64;
const CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES: usize = 32;
const CRYPTO_GENERICHASH_BLAKE2B_SALTBYTES: usize = 16;
const CRYPTO_GENERICHASH_BLAKE2B_PERSONALBYTES: usize = 16;

/// `crypto_generichash_blake2b`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> c_int {
    // The C source also checks `inlen > UINT64_MAX`, which can never be
    // true since `inlen` is already `uint64_t` (`unsigned long long`).
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    _sodium_blake2b(
        out,
        in_ as *const c_void,
        key as *const c_void,
        outlen as u8,
        inlen,
        keylen as u8,
    )
}

/// `crypto_generichash_blake2b_salt_personal`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_salt_personal(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
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

/// `crypto_generichash_blake2b_init`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_init(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    let s = state.cast::<blake2b_state>();
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init(s, outlen as u8) != 0 {
            return -1;
        }
    } else if _sodium_blake2b_init_key(s, outlen as u8, key as *const c_void, keylen as u8) != 0 {
        return -1;
    }
    0
}

/// `crypto_generichash_blake2b_init_salt_personal`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_init_salt_personal(
    state: *mut crypto_generichash_blake2b_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
    salt: *const u8,
    personal: *const u8,
) -> c_int {
    if outlen == 0 || outlen > BLAKE2B_OUTBYTES || keylen > BLAKE2B_KEYBYTES {
        return -1;
    }
    let s = state.cast::<blake2b_state>();
    if key.is_null() || keylen == 0 {
        if _sodium_blake2b_init_salt_personal(
            s,
            outlen as u8,
            salt as *const c_void,
            personal as *const c_void,
        ) != 0
        {
            return -1;
        }
    } else if _sodium_blake2b_init_key_salt_personal(
        s,
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

/// `crypto_generichash_blake2b_update`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_update(
    state: *mut crypto_generichash_blake2b_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    _sodium_blake2b_update(state.cast::<blake2b_state>(), in_, inlen)
}

/// `crypto_generichash_blake2b_final`.
#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_final(
    state: *mut crypto_generichash_blake2b_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    // C: assert(outlen <= UINT8_MAX);  the reference build does not define
    // NDEBUG, so this is live and aborts.  Without it an `outlen` such as 257
    // would silently truncate to 1 and succeed instead of terminating.
    if outlen > 0xff {
        crate::csys::abort();
    }
    _sodium_blake2b_final(state.cast::<blake2b_state>(), out, outlen as u8)
}

/// `_crypto_generichash_blake2b_pick_best_implementation`.
#[no_mangle]
pub unsafe extern "C" fn _crypto_generichash_blake2b_pick_best_implementation() -> c_int {
    _sodium_blake2b_pick_best_implementation()
}

// ===========================================================================
// crypto_generichash/blake2b/generichash_blake2.c
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes_min() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes_max() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_bytes() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes_min() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes_max() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_keybytes() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_saltbytes() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_SALTBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_personalbytes() -> usize {
    CRYPTO_GENERICHASH_BLAKE2B_PERSONALBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_blake2b_state>() + 63) & !63
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_blake2b_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES);
}

// ===========================================================================
// crypto_generichash/crypto_generichash.c
// ===========================================================================

/// `typedef crypto_generichash_blake2b_state crypto_generichash_state;`
type crypto_generichash_state = crypto_generichash_blake2b_state;

const CRYPTO_GENERICHASH_BYTES_MIN: usize = CRYPTO_GENERICHASH_BLAKE2B_BYTES_MIN;
const CRYPTO_GENERICHASH_BYTES_MAX: usize = CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX;
const CRYPTO_GENERICHASH_BYTES: usize = CRYPTO_GENERICHASH_BLAKE2B_BYTES;
const CRYPTO_GENERICHASH_KEYBYTES_MIN: usize = CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MIN;
const CRYPTO_GENERICHASH_KEYBYTES_MAX: usize = CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES_MAX;
const CRYPTO_GENERICHASH_KEYBYTES: usize = CRYPTO_GENERICHASH_BLAKE2B_KEYBYTES;

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_bytes_min() -> usize {
    CRYPTO_GENERICHASH_BYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_bytes_max() -> usize {
    CRYPTO_GENERICHASH_BYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_bytes() -> usize {
    CRYPTO_GENERICHASH_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_keybytes_min() -> usize {
    CRYPTO_GENERICHASH_KEYBYTES_MIN
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_keybytes_max() -> usize {
    CRYPTO_GENERICHASH_KEYBYTES_MAX
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_keybytes() -> usize {
    CRYPTO_GENERICHASH_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_primitive() -> *const c_char {
    b"blake2b\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_statebytes() -> usize {
    (core::mem::size_of::<crypto_generichash_state>() + 63) & !63
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash(
    out: *mut u8,
    outlen: usize,
    in_: *const u8,
    inlen: u64,
    key: *const u8,
    keylen: usize,
) -> c_int {
    crypto_generichash_blake2b(out, outlen, in_, inlen, key, keylen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_init(
    state: *mut crypto_generichash_state,
    key: *const u8,
    keylen: usize,
    outlen: usize,
) -> c_int {
    crypto_generichash_blake2b_init(state.cast::<crypto_generichash_blake2b_state>(), key, keylen, outlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_update(
    state: *mut crypto_generichash_state,
    in_: *const u8,
    inlen: u64,
) -> c_int {
    crypto_generichash_blake2b_update(state.cast::<crypto_generichash_blake2b_state>(), in_, inlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_final(
    state: *mut crypto_generichash_state,
    out: *mut u8,
    outlen: usize,
) -> c_int {
    crypto_generichash_blake2b_final(state.cast::<crypto_generichash_blake2b_state>(), out, outlen)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_generichash_keygen(k: *mut u8) {
    randombytes_buf(k as *mut c_void, CRYPTO_GENERICHASH_KEYBYTES);
}

// ===========================================================================
// crypto_pwhash/argon2/blake2b-long.c
// ===========================================================================

/// `blake2b_long` -> `_sodium_blake2b_long`.
#[no_mangle]
pub unsafe extern "C" fn _sodium_blake2b_long(
    pout: *mut c_void,
    outlen: usize,
    in_: *const c_void,
    inlen: usize,
) -> c_int {
    let mut out = pout as *mut u8;
    let mut blake_state: crypto_generichash_blake2b_state = core::mem::zeroed();
    let mut outlen_bytes = [0u8; 4 /* sizeof(uint32_t) */];
    let mut ret: c_int = -1;

    'fail: {
        if outlen > u32::MAX as usize {
            break 'fail; /* LCOV_EXCL_LINE */
        }

        // Ensure little-endian byte order!
        store32_le(outlen_bytes.as_mut_ptr(), outlen as u32);

        if outlen <= CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX {
            ret = crypto_generichash_blake2b_init(&mut blake_state, core::ptr::null(), 0, outlen);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(&mut blake_state, outlen_bytes.as_ptr(), outlen_bytes.len() as u64);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(&mut blake_state, in_ as *const u8, inlen as u64);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(&mut blake_state, out, outlen);
            if ret < 0 {
                break 'fail;
            }
        } else {
            let mut out_buffer = [0u8; CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX];
            let mut in_buffer = [0u8; CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX];

            ret = crypto_generichash_blake2b_init(
                &mut blake_state,
                core::ptr::null(),
                0,
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(&mut blake_state, outlen_bytes.as_ptr(), outlen_bytes.len() as u64);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_update(&mut blake_state, in_ as *const u8, inlen as u64);
            if ret < 0 {
                break 'fail;
            }
            ret = crypto_generichash_blake2b_final(
                &mut blake_state,
                out_buffer.as_mut_ptr(),
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
            );
            if ret < 0 {
                break 'fail;
            }
            memcpy(
                out as *mut c_void,
                out_buffer.as_ptr() as *const c_void,
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2,
            );
            out = out.add(CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2);
            let mut toproduce: u32 = outlen as u32 - (CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2) as u32;

            while toproduce > CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX as u32 {
                memcpy(
                    in_buffer.as_mut_ptr() as *mut c_void,
                    out_buffer.as_ptr() as *const c_void,
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
                );
                ret = crypto_generichash_blake2b(
                    out_buffer.as_mut_ptr(),
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
                    in_buffer.as_ptr(),
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX as u64,
                    core::ptr::null(),
                    0,
                );
                if ret < 0 {
                    break 'fail;
                }
                memcpy(
                    out as *mut c_void,
                    out_buffer.as_ptr() as *const c_void,
                    CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2,
                );
                out = out.add(CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2);
                toproduce -= (CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX / 2) as u32;
            }

            memcpy(
                in_buffer.as_mut_ptr() as *mut c_void,
                out_buffer.as_ptr() as *const c_void,
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX,
            );
            ret = crypto_generichash_blake2b(
                out_buffer.as_mut_ptr(),
                toproduce as usize,
                in_buffer.as_ptr(),
                CRYPTO_GENERICHASH_BLAKE2B_BYTES_MAX as u64,
                core::ptr::null(),
                0,
            );
            if ret < 0 {
                break 'fail;
            }
            memcpy(out as *mut c_void, out_buffer.as_ptr() as *const c_void, toproduce as usize);
        }
    }

    sodium_memzero(
        &mut blake_state as *mut crypto_generichash_blake2b_state as *mut c_void,
        core::mem::size_of::<crypto_generichash_blake2b_state>(),
    );
    ret
}
