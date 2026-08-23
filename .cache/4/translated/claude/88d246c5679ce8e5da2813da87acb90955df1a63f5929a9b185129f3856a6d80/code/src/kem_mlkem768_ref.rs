//! Translation of `c_src/libsodium/crypto_kem/mlkem768/ref/kem_mlkem768_ref.c`.
//!
//! ML-KEM-768 (FIPS 203) reference implementation.
//!
//! Exported symbols (after the `private/quirks.h` renaming):
//!   `mlkem768_ref_keypair`           -> `_sodium_mlkem768_ref_keypair`
//!   `mlkem768_ref_seed_keypair`      -> `_sodium_mlkem768_ref_seed_keypair`
//!   `mlkem768_ref_enc`               -> `_sodium_mlkem768_ref_enc`
//!   `mlkem768_ref_enc_deterministic` -> `_sodium_mlkem768_ref_enc_deterministic`
//!   `mlkem768_ref_dec`               -> `_sodium_mlkem768_ref_dec`

use crate::common::*;
use core::ffi::{c_int, c_ulonglong, c_void};

/// C `unsigned int` on this target.
type c_uint_local = u32;

/* ------------------------------------------------------------------ */
/* Constants from crypto_kem_mlkem768.h                               */
/* ------------------------------------------------------------------ */

const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_mlkem768_SEEDBYTES: usize = 64;

/* crypto_xof_shake128.h / crypto_xof_shake256.h */
const crypto_xof_shake128_BLOCKBYTES: usize = 168;
const crypto_xof_shake256_BLOCKBYTES: usize = 136;

/* ------------------------------------------------------------------ */
/* Local #defines                                                     */
/* ------------------------------------------------------------------ */

const MLKEM768_Q: i32 = 3329;
const MLKEM768_N: usize = 256;
const MLKEM768_K: usize = 3;
const MLKEM768_ETA1: usize = 2;
const MLKEM768_ETA2: usize = 2;
const MLKEM768_DU: usize = 10;
const MLKEM768_DV: usize = 4;

const MLKEM768_POLYBYTES: usize = 384;
const MLKEM768_POLYVECBYTES: usize = MLKEM768_K * MLKEM768_POLYBYTES;
const MLKEM768_POLYCOMPRESSEDBYTES_DU: usize = 320;
const MLKEM768_POLYCOMPRESSEDBYTES_DV: usize = 128;
const MLKEM768_POLYVECCOMPRESSEDBYTES_DU: usize = MLKEM768_K * MLKEM768_POLYCOMPRESSEDBYTES_DU;

/* ------------------------------------------------------------------ */
/* Opaque state types of the SHA-3 / SHAKE subsystems                 */
/* ------------------------------------------------------------------ */

/// `crypto_xof_shake128_state` from `include/sodium/crypto_xof_shake128.h`.
#[repr(C, align(16))]
struct crypto_xof_shake128_state {
    opaque: [u8; 256],
}

/// `crypto_xof_shake256_state` from `include/sodium/crypto_xof_shake256.h`.
#[repr(C, align(16))]
struct crypto_xof_shake256_state {
    opaque: [u8; 256],
}

extern "C" {
    /* crypto_hash/sha3/hash_sha3.c */
    fn crypto_hash_sha3256(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;
    fn crypto_hash_sha3512(out: *mut u8, in_: *const u8, inlen: c_ulonglong) -> c_int;

    /* crypto_xof/shake/xof_shake128.c */
    fn crypto_xof_shake128_init(state: *mut crypto_xof_shake128_state) -> c_int;
    fn crypto_xof_shake128_update(
        state: *mut crypto_xof_shake128_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_xof_shake128_squeeze(
        state: *mut crypto_xof_shake128_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    /* crypto_xof/shake/xof_shake256.c */
    fn crypto_xof_shake256_init(state: *mut crypto_xof_shake256_state) -> c_int;
    fn crypto_xof_shake256_update(
        state: *mut crypto_xof_shake256_state,
        in_: *const u8,
        inlen: c_ulonglong,
    ) -> c_int;
    fn crypto_xof_shake256_squeeze(
        state: *mut crypto_xof_shake256_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    /* randombytes/randombytes.c */
    fn randombytes_buf(buf: *mut c_void, size: usize);

    /* sodium/utils.c */
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
}

/* ------------------------------------------------------------------ */
/* poly / polyvec                                                     */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy)]
struct poly {
    coeffs: [i16; MLKEM768_N],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct polyvec {
    vec: [poly; MLKEM768_K],
}

const POLY_ZERO: poly = poly {
    coeffs: [0i16; MLKEM768_N],
};
const POLYVEC_ZERO: polyvec = polyvec {
    vec: [POLY_ZERO; MLKEM768_K],
};

/* `static const int16_t zetas[128]` — 128 elements, verified. */
static zetas: [i16; 128] = [
    2285, 2571, 2970, 1812, 1493, 1422, 287, 202, 3158, 622, 1577, 182, 962, 2127, 1855, 1468, 573,
    2004, 264, 383, 2500, 1458, 1727, 3199, 2648, 1017, 732, 608, 1787, 411, 3124, 1758, 1223, 652,
    2777, 1015, 2036, 1491, 3047, 1785, 516, 3321, 3009, 2663, 1711, 2167, 126, 1469, 2476, 3239,
    3058, 830, 107, 1908, 3082, 2378, 2931, 961, 1821, 2604, 448, 2264, 677, 2054, 2226, 430, 555,
    843, 2078, 871, 1550, 105, 422, 587, 177, 3094, 3038, 2869, 1574, 1653, 3083, 778, 1159, 3182,
    2552, 1483, 2727, 1119, 1739, 644, 2457, 349, 418, 329, 3173, 3254, 817, 1097, 603, 610, 1322,
    2044, 1864, 384, 2114, 3193, 1218, 1994, 2455, 220, 2142, 1670, 2144, 1799, 2051, 794, 1819,
    2475, 2459, 478, 3221, 3021, 996, 991, 958, 1869, 1522, 1628,
];

/* ------------------------------------------------------------------ */
/* Modular arithmetic                                                 */
/* ------------------------------------------------------------------ */

/// ```c
/// t = (int16_t) ((uint16_t) a * 62209U);
/// t = (int16_t) ((a - (int32_t) t * MLKEM768_Q) >> 16);
/// ```
#[inline]
fn montgomery_reduce(a: i32) -> i16 {
    let mut t: i16;

    t = ((a as u16 as u32).wrapping_mul(62209u32)) as u16 as i16;
    t = (a.wrapping_sub((t as i32).wrapping_mul(MLKEM768_Q)) >> 16) as i16;

    t
}

/// ```c
/// t = (int16_t) (((int32_t) a * 20159) >> 26);
/// t = a - t * MLKEM768_Q;
/// ```
#[inline]
fn barrett_reduce(a: i16) -> i16 {
    let mut t: i16;

    t = (((a as i32).wrapping_mul(20159)) >> 26) as i16;
    t = (a as i32).wrapping_sub((t as i32).wrapping_mul(MLKEM768_Q)) as i16;

    t
}

/// ```c
/// a -= MLKEM768_Q;
/// a += (a >> 15) & MLKEM768_Q;
/// ```
#[inline]
fn csubq(a: i16) -> i16 {
    let mut a: i16 = a;

    a = a.wrapping_sub(MLKEM768_Q as i16);
    a = a.wrapping_add((((a as i32) >> 15) & MLKEM768_Q) as i16);

    a
}

/* ------------------------------------------------------------------ */
/* poly operations                                                    */
/* ------------------------------------------------------------------ */

unsafe fn poly_ntt(r: *mut poly) {
    let mut len: usize;
    let mut start: usize;
    let mut j: usize;
    let mut k: usize;
    let mut t: i16;
    let mut zeta: i16;

    k = 1;
    len = 128;
    while len >= 2 {
        start = 0;
        while start < MLKEM768_N {
            zeta = zetas[k];
            k += 1;
            j = start;
            while j < start + len {
                t = montgomery_reduce((zeta as i32).wrapping_mul((*r).coeffs[j + len] as i32));
                (*r).coeffs[j + len] = (*r).coeffs[j].wrapping_sub(t);
                (*r).coeffs[j] = (*r).coeffs[j].wrapping_add(t);
                j += 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

unsafe fn poly_invntt(r: *mut poly) {
    let mut start: usize;
    let mut len: usize;
    let mut j: usize;
    let mut k: usize;
    let mut t: i16;
    let mut zeta: i16;
    let f: i16 = 1441;

    k = 127;
    len = 2;
    while len <= 128 {
        start = 0;
        while start < MLKEM768_N {
            zeta = zetas[k];
            k = k.wrapping_sub(1);
            j = start;
            while j < start + len {
                t = (*r).coeffs[j];
                (*r).coeffs[j] = barrett_reduce(t.wrapping_add((*r).coeffs[j + len]));
                (*r).coeffs[j + len] = montgomery_reduce(
                    (zeta as i32).wrapping_mul(((*r).coeffs[j + len] as i32) - (t as i32)),
                );
                j += 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    j = 0;
    while j < MLKEM768_N {
        (*r).coeffs[j] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[j] as i32));
        j += 1;
    }
}

unsafe fn poly_basemul(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;
    let mut zeta: i16;

    i = 0;
    while i < MLKEM768_N / 4 {
        zeta = zetas[64 + i];

        (*r).coeffs[4 * i] = montgomery_reduce(
            ((*a).coeffs[4 * i + 1] as i32).wrapping_mul((*b).coeffs[4 * i + 1] as i32),
        );
        (*r).coeffs[4 * i] =
            montgomery_reduce(((*r).coeffs[4 * i] as i32).wrapping_mul(zeta as i32));
        (*r).coeffs[4 * i] = (*r).coeffs[4 * i].wrapping_add(montgomery_reduce(
            ((*a).coeffs[4 * i] as i32).wrapping_mul((*b).coeffs[4 * i] as i32),
        ));

        (*r).coeffs[4 * i + 1] = montgomery_reduce(
            ((*a).coeffs[4 * i] as i32).wrapping_mul((*b).coeffs[4 * i + 1] as i32),
        );
        (*r).coeffs[4 * i + 1] = (*r).coeffs[4 * i + 1].wrapping_add(montgomery_reduce(
            ((*a).coeffs[4 * i + 1] as i32).wrapping_mul((*b).coeffs[4 * i] as i32),
        ));

        (*r).coeffs[4 * i + 2] = montgomery_reduce(
            ((*a).coeffs[4 * i + 3] as i32).wrapping_mul((*b).coeffs[4 * i + 3] as i32),
        );
        (*r).coeffs[4 * i + 2] = montgomery_reduce(
            ((*r).coeffs[4 * i + 2] as i32).wrapping_mul((zeta as i32).wrapping_neg()),
        );
        (*r).coeffs[4 * i + 2] = (*r).coeffs[4 * i + 2].wrapping_add(montgomery_reduce(
            ((*a).coeffs[4 * i + 2] as i32).wrapping_mul((*b).coeffs[4 * i + 2] as i32),
        ));

        (*r).coeffs[4 * i + 3] = montgomery_reduce(
            ((*a).coeffs[4 * i + 2] as i32).wrapping_mul((*b).coeffs[4 * i + 3] as i32),
        );
        (*r).coeffs[4 * i + 3] = (*r).coeffs[4 * i + 3].wrapping_add(montgomery_reduce(
            ((*a).coeffs[4 * i + 3] as i32).wrapping_mul((*b).coeffs[4 * i + 2] as i32),
        ));

        i += 1;
    }
}

unsafe fn poly_tomont(r: *mut poly) {
    let mut i: usize;
    let f: i16 = 1353;

    i = 0;
    while i < MLKEM768_N {
        (*r).coeffs[i] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[i] as i32));
        i += 1;
    }
}

unsafe fn poly_reduce(r: *mut poly) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N {
        (*r).coeffs[i] = barrett_reduce((*r).coeffs[i]);
        i += 1;
    }
}

unsafe fn poly_add(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N {
        (*r).coeffs[i] = (*a).coeffs[i].wrapping_add((*b).coeffs[i]);
        i += 1;
    }
}

unsafe fn poly_sub(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N {
        (*r).coeffs[i] = (*a).coeffs[i].wrapping_sub((*b).coeffs[i]);
        i += 1;
    }
}

unsafe fn poly_csubq(r: *mut poly) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N {
        (*r).coeffs[i] = csubq((*r).coeffs[i]);
        i += 1;
    }
}

/// `static void poly_cbd_eta2(poly *r, const unsigned char buf[128])`
unsafe fn poly_cbd_eta2(r: *mut poly, buf: *const u8) {
    let mut i: usize;
    let mut j: usize;
    let mut t: u32;
    let mut d: u32;
    let mut a: i16;
    let mut b: i16;

    i = 0;
    while i < MLKEM768_N / 8 {
        t = (*buf.add(4 * i) as u32)
            | ((*buf.add(4 * i + 1) as u32) << 8)
            | ((*buf.add(4 * i + 2) as u32) << 16)
            | ((*buf.add(4 * i + 3) as u32) << 24);

        d = t & 0x55555555;
        d = d.wrapping_add((t >> 1) & 0x55555555);

        j = 0;
        while j < 8 {
            a = ((d >> (4 * j)) & 0x3) as i16;
            b = ((d >> (4 * j + 2)) & 0x3) as i16;
            (*r).coeffs[8 * i + j] = a.wrapping_sub(b);
            j += 1;
        }
        i += 1;
    }
}

/// `static void poly_getnoise_eta2(poly *r, const unsigned char seed[32], uint8_t nonce)`
unsafe fn poly_getnoise_eta2(r: *mut poly, seed: *const u8, nonce: u8) {
    let mut buf = [0u8; MLKEM768_ETA2 * MLKEM768_N / 4];
    let mut state = crypto_xof_shake256_state { opaque: [0u8; 256] };
    let mut extseed = [0u8; 33];

    memcpy(extseed.as_mut_ptr(), seed, 32);
    extseed[32] = nonce;

    crypto_xof_shake256_init(&mut state);
    crypto_xof_shake256_update(&mut state, extseed.as_ptr(), 33);
    crypto_xof_shake256_squeeze(&mut state, buf.as_mut_ptr(), buf.len());

    poly_cbd_eta2(r, buf.as_ptr());
    sodium_memzero(
        &mut state as *mut crypto_xof_shake256_state as *mut c_void,
        core::mem::size_of::<crypto_xof_shake256_state>(),
    );
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
}

/// `static void poly_frombytes(poly *r, const unsigned char a[MLKEM768_POLYBYTES])`
unsafe fn poly_frombytes(r: *mut poly, a: *const u8) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N / 2 {
        (*r).coeffs[2 * i] = ((((*a.add(3 * i + 0) as u32) >> 0)
            | ((*a.add(3 * i + 1) as u32) << 8))
            & 0xFFF) as i16;
        (*r).coeffs[2 * i + 1] = ((((*a.add(3 * i + 1) as u32) >> 4)
            | ((*a.add(3 * i + 2) as u32) << 4))
            & 0xFFF) as i16;
        i += 1;
    }
}

/// `static void poly_tobytes(unsigned char r[MLKEM768_POLYBYTES], const poly *a)`
unsafe fn poly_tobytes(r: *mut u8, a: *const poly) {
    let mut i: usize;
    let mut t0: u16;
    let mut t1: u16;

    i = 0;
    while i < MLKEM768_N / 2 {
        t0 = (*a).coeffs[2 * i] as u16;
        t1 = (*a).coeffs[2 * i + 1] as u16;
        *r.add(3 * i + 0) = ((t0 as u32) >> 0) as u8;
        *r.add(3 * i + 1) = (((t0 as u32) >> 8) | ((t1 as u32) << 4)) as u8;
        *r.add(3 * i + 2) = ((t1 as u32) >> 4) as u8;
        i += 1;
    }
}

/// `static void poly_frommsg(poly *r, const unsigned char msg[32])`
unsafe fn poly_frommsg(r: *mut poly, msg: *const u8) {
    let mut i: usize;
    let mut j: usize;
    let mut mask: i16;

    i = 0;
    while i < MLKEM768_N / 8 {
        j = 0;
        while j < 8 {
            mask = (((*msg.add(i) as i32) >> j) & 1).wrapping_neg() as i16;
            (*r).coeffs[8 * i + j] = mask & (((MLKEM768_Q + 1) / 2) as i16);
            j += 1;
        }
        i += 1;
    }
}

/// `static void poly_tomsg(unsigned char msg[32], const poly *a)`
unsafe fn poly_tomsg(msg: *mut u8, a: *const poly) {
    let mut i: usize;
    let mut j: usize;
    let mut t: u32;

    i = 0;
    while i < MLKEM768_N / 8 {
        *msg.add(i) = 0;
        j = 0;
        while j < 8 {
            t = (*a).coeffs[8 * i + j] as i32 as u32;
            t = t.wrapping_add(((((*a).coeffs[8 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
            t = (t
                .wrapping_shl(1)
                .wrapping_add((MLKEM768_Q / 2) as u32)
                .wrapping_mul(80635))
                >> 28;
            t &= 1;
            *msg.add(i) |= (t << j) as u8;
            j += 1;
        }
        i += 1;
    }
}

/// `static void poly_compress_du(unsigned char r[MLKEM768_POLYCOMPRESSEDBYTES_DU], const poly *a)`
unsafe fn poly_compress_du(r: *mut u8, a: *const poly) {
    let mut t = [0u32; 4];
    let mut i: usize;
    let mut j: usize;

    i = 0;
    while i < MLKEM768_N / 4 {
        j = 0;
        while j < 4 {
            t[j] = (*a).coeffs[4 * i + j] as i32 as u32;
            t[j] = t[j].wrapping_add(((((*a).coeffs[4 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
            t[j] = ((((t[j] as u64) << 10).wrapping_add((MLKEM768_Q / 2) as u64))
                .wrapping_mul(161271u64)
                >> 29) as u32;
            t[j] &= 0x3ff;
            j += 1;
        }

        *r.add(5 * i + 0) = (t[0] >> 0) as u8;
        *r.add(5 * i + 1) = ((t[0] >> 8) | (t[1] << 2)) as u8;
        *r.add(5 * i + 2) = ((t[1] >> 6) | (t[2] << 4)) as u8;
        *r.add(5 * i + 3) = ((t[2] >> 4) | (t[3] << 6)) as u8;
        *r.add(5 * i + 4) = (t[3] >> 2) as u8;
        i += 1;
    }
}

/// `static void poly_decompress_du(poly *r, const unsigned char a[MLKEM768_POLYCOMPRESSEDBYTES_DU])`
unsafe fn poly_decompress_du(r: *mut poly, a: *const u8) {
    let mut t = [0u16; 4];
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N / 4 {
        t[0] = ((((*a.add(5 * i + 0) as u32) >> 0) | ((*a.add(5 * i + 1) as u32) << 8)) & 0xFFFF)
            as u16;
        t[1] = ((((*a.add(5 * i + 1) as u32) >> 2) | ((*a.add(5 * i + 2) as u32) << 6)) & 0xFFFF)
            as u16;
        t[2] = ((((*a.add(5 * i + 2) as u32) >> 4) | ((*a.add(5 * i + 3) as u32) << 4)) & 0xFFFF)
            as u16;
        t[3] = ((((*a.add(5 * i + 3) as u32) >> 6) | ((*a.add(5 * i + 4) as u32) << 2)) & 0xFFFF)
            as u16;

        (*r).coeffs[4 * i + 0] =
            ((((t[0] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10)
                as i16;
        (*r).coeffs[4 * i + 1] =
            ((((t[1] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10)
                as i16;
        (*r).coeffs[4 * i + 2] =
            ((((t[2] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10)
                as i16;
        (*r).coeffs[4 * i + 3] =
            ((((t[3] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10)
                as i16;
        i += 1;
    }
}

/// `static void poly_compress_dv(unsigned char r[MLKEM768_POLYCOMPRESSEDBYTES_DV], const poly *a)`
unsafe fn poly_compress_dv(r: *mut u8, a: *const poly) {
    let mut t = [0u32; 8];
    let mut i: usize;
    let mut j: usize;

    i = 0;
    while i < MLKEM768_N / 8 {
        j = 0;
        while j < 8 {
            t[j] = (*a).coeffs[8 * i + j] as i32 as u32;
            t[j] = t[j].wrapping_add(((((*a).coeffs[8 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
            t[j] = ((((t[j] as u64) << 4).wrapping_add((MLKEM768_Q / 2) as u64))
                .wrapping_mul(161271u64)
                >> 29) as u32;
            t[j] &= 0xf;
            j += 1;
        }

        *r.add(4 * i + 0) = (t[0] | (t[1] << 4)) as u8;
        *r.add(4 * i + 1) = (t[2] | (t[3] << 4)) as u8;
        *r.add(4 * i + 2) = (t[4] | (t[5] << 4)) as u8;
        *r.add(4 * i + 3) = (t[6] | (t[7] << 4)) as u8;
        i += 1;
    }
}

/// `static void poly_decompress_dv(poly *r, const unsigned char a[MLKEM768_POLYCOMPRESSEDBYTES_DV])`
unsafe fn poly_decompress_dv(r: *mut poly, a: *const u8) {
    let mut i: usize;

    i = 0;
    while i < MLKEM768_N / 2 {
        (*r).coeffs[2 * i + 0] = (((((*a.add(i) & 15) as u32).wrapping_mul(MLKEM768_Q as u32))
            .wrapping_add(8))
            >> 4) as i16;
        (*r).coeffs[2 * i + 1] = (((((*a.add(i) >> 4) as u32).wrapping_mul(MLKEM768_Q as u32))
            .wrapping_add(8))
            >> 4) as i16;
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/* polyvec operations                                                 */
/* ------------------------------------------------------------------ */

unsafe fn polyvec_ntt(r: *mut polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_ntt(&mut (*r).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_invntt(r: *mut polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_invntt(&mut (*r).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_basemul_acc(r: *mut poly, a: *const polyvec, b: *const polyvec) {
    let mut t: poly = POLY_ZERO;
    let mut i: usize;

    poly_basemul(r, &(*a).vec[0], &(*b).vec[0]);
    i = 1;
    while i < MLKEM768_K {
        poly_basemul(&mut t, &(*a).vec[i], &(*b).vec[i]);
        poly_add(r, r, &t);
        i += 1;
    }

    poly_reduce(r);
}

unsafe fn polyvec_reduce(r: *mut polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_reduce(&mut (*r).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_csubq(r: *mut polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_csubq(&mut (*r).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_add(r: *mut polyvec, a: *const polyvec, b: *const polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_add(&mut (*r).vec[i], &(*a).vec[i], &(*b).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_tobytes(r: *mut u8, a: *const polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_tobytes(r.add(i * MLKEM768_POLYBYTES), &(*a).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_frombytes(r: *mut polyvec, a: *const u8) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_frombytes(&mut (*r).vec[i], a.add(i * MLKEM768_POLYBYTES));
        i += 1;
    }
}

unsafe fn polyvec_is_canonical(a: *const polyvec) -> c_int {
    let mut i: usize;
    let mut j: usize;

    i = 0;
    while i < MLKEM768_K {
        j = 0;
        while j < MLKEM768_N {
            if ((*a).vec[i].coeffs[j] as u16 as i32) >= MLKEM768_Q {
                return 0;
            }
            j += 1;
        }
        i += 1;
    }
    1
}

unsafe fn polyvec_compress(r: *mut u8, a: *const polyvec) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_compress_du(r.add(i * MLKEM768_POLYCOMPRESSEDBYTES_DU), &(*a).vec[i]);
        i += 1;
    }
}

unsafe fn polyvec_decompress(r: *mut polyvec, a: *const u8) {
    let mut i: usize = 0;

    while i < MLKEM768_K {
        poly_decompress_du(&mut (*r).vec[i], a.add(i * MLKEM768_POLYCOMPRESSEDBYTES_DU));
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/* Matrix generation                                                  */
/* ------------------------------------------------------------------ */

unsafe fn rej_uniform(r: *mut i16, len: usize, buf: *const u8, buflen: usize) -> usize {
    let mut ctr: usize;
    let mut pos: usize;
    let mut val0: u16;
    let mut val1: u16;

    ctr = 0;
    pos = 0;
    /* Variable-time rejection is fine here: callers only use public matrix seeds. */
    while ctr < len && pos + 3 <= buflen {
        val0 = ((((*buf.add(pos + 0) as u32) >> 0) | ((*buf.add(pos + 1) as u32) << 8)) & 0xFFF)
            as u16;
        val1 = ((((*buf.add(pos + 1) as u32) >> 4) | ((*buf.add(pos + 2) as u32) << 4)) & 0xFFF)
            as u16;
        pos += 3;

        if (val0 as i32) < MLKEM768_Q {
            *r.add(ctr) = val0 as i16;
            ctr += 1;
        }
        if ctr < len && (val1 as i32) < MLKEM768_Q {
            *r.add(ctr) = val1 as i16;
            ctr += 1;
        }
    }

    ctr
}

/// ```c
/// #define GEN_MATRIX_NBLOCKS \
///     ((12 * MLKEM768_N / 8 * (1 << 12) / MLKEM768_Q + crypto_xof_shake128_BLOCKBYTES) / \
///      crypto_xof_shake128_BLOCKBYTES)
/// ```
const GEN_MATRIX_NBLOCKS: usize = (12 * MLKEM768_N / 8 * (1usize << 12) / (MLKEM768_Q as usize)
    + crypto_xof_shake128_BLOCKBYTES)
    / crypto_xof_shake128_BLOCKBYTES;

/// `static void gen_matrix(polyvec *a, const unsigned char seed[32], int transposed)`
unsafe fn gen_matrix(a: *mut polyvec, seed: *const u8, transposed: c_int) {
    let mut state = crypto_xof_shake128_state { opaque: [0u8; 256] };
    let mut buf = [0u8; GEN_MATRIX_NBLOCKS * crypto_xof_shake128_BLOCKBYTES + 2];
    let mut extseed = [0u8; 34];
    let mut ctr: usize;
    let mut i: usize;
    let mut j: usize;
    let mut buflen: usize;

    memcpy(extseed.as_mut_ptr(), seed, 32);

    i = 0;
    while i < MLKEM768_K {
        j = 0;
        while j < MLKEM768_K {
            if transposed != 0 {
                extseed[32] = i as u8;
                extseed[33] = j as u8;
            } else {
                extseed[32] = j as u8;
                extseed[33] = i as u8;
            }

            crypto_xof_shake128_init(&mut state);
            crypto_xof_shake128_update(&mut state, extseed.as_ptr(), 34);

            buflen = GEN_MATRIX_NBLOCKS * crypto_xof_shake128_BLOCKBYTES;
            crypto_xof_shake128_squeeze(&mut state, buf.as_mut_ptr(), buflen);

            let coeffs = (*a.add(i)).vec[j].coeffs.as_mut_ptr();
            ctr = rej_uniform(coeffs, MLKEM768_N, buf.as_ptr(), buflen);

            /* Refill count depends on public XOF output, not on secret key material. */
            while ctr < MLKEM768_N {
                crypto_xof_shake128_squeeze(
                    &mut state,
                    buf.as_mut_ptr(),
                    crypto_xof_shake128_BLOCKBYTES,
                );
                ctr += rej_uniform(
                    coeffs.add(ctr),
                    MLKEM768_N - ctr,
                    buf.as_ptr(),
                    crypto_xof_shake128_BLOCKBYTES,
                );
            }
            j += 1;
        }
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/* IND-CPA layer                                                      */
/* ------------------------------------------------------------------ */

/// ```c
/// static void indcpa_keypair(unsigned char pk[crypto_kem_mlkem768_PUBLICKEYBYTES],
///                            unsigned char sk[MLKEM768_POLYVECBYTES],
///                            const unsigned char seed[32]);
/// ```
unsafe fn indcpa_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) {
    let mut a = [POLYVEC_ZERO; MLKEM768_K];
    let mut e: polyvec = POLYVEC_ZERO;
    let mut pkpv: polyvec = POLYVEC_ZERO;
    let mut skpv: polyvec = POLYVEC_ZERO;
    let mut buf = [0u8; 64];
    let mut i: usize;
    let mut nonce: u8 = 0;

    crypto_hash_sha3512(buf.as_mut_ptr(), seed, 33);

    let publicseed: *mut u8 = buf.as_mut_ptr();
    let noiseseed: *mut u8 = buf.as_mut_ptr().add(32);

    gen_matrix(a.as_mut_ptr(), publicseed, 0);

    i = 0;
    while i < MLKEM768_K {
        poly_getnoise_eta2(&mut skpv.vec[i], noiseseed, nonce);
        nonce = nonce.wrapping_add(1);
        i += 1;
    }
    i = 0;
    while i < MLKEM768_K {
        poly_getnoise_eta2(&mut e.vec[i], noiseseed, nonce);
        nonce = nonce.wrapping_add(1);
        i += 1;
    }

    polyvec_ntt(&mut skpv);
    polyvec_ntt(&mut e);

    i = 0;
    while i < MLKEM768_K {
        polyvec_basemul_acc(&mut pkpv.vec[i], &a[i], &skpv);
        poly_tomont(&mut pkpv.vec[i]);
        i += 1;
    }

    /* `polyvec_add(&pkpv, &pkpv, &e)` — output aliases the first input, so go
     * through a raw pointer to mirror the C exactly. */
    let pkpv_p: *mut polyvec = &mut pkpv;
    polyvec_add(pkpv_p, pkpv_p, &e);
    polyvec_reduce(&mut pkpv);
    polyvec_csubq(&mut pkpv);
    polyvec_reduce(&mut skpv);
    polyvec_csubq(&mut skpv);

    polyvec_tobytes(sk, &skpv);
    polyvec_tobytes(pk, &pkpv);
    memcpy(pk.add(MLKEM768_POLYVECBYTES), publicseed, 32);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(
        &mut skpv as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
    sodium_memzero(
        &mut e as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
}

/// ```c
/// static void indcpa_enc(unsigned char ct[crypto_kem_mlkem768_CIPHERTEXTBYTES],
///                        const unsigned char m[32],
///                        const unsigned char pk[crypto_kem_mlkem768_PUBLICKEYBYTES],
///                        const unsigned char coins[32]);
/// ```
unsafe fn indcpa_enc(ct: *mut u8, m: *const u8, pk: *const u8, coins: *const u8) {
    let mut sp: polyvec = POLYVEC_ZERO;
    let mut pkpv: polyvec = POLYVEC_ZERO;
    let mut ep: polyvec = POLYVEC_ZERO;
    let mut at = [POLYVEC_ZERO; MLKEM768_K];
    let mut b: polyvec = POLYVEC_ZERO;
    let mut v: poly = POLY_ZERO;
    let mut k: poly = POLY_ZERO;
    let mut epp: poly = POLY_ZERO;
    let mut seed = [0u8; 32];
    let mut i: usize;
    let mut nonce: u8 = 0;

    memcpy(seed.as_mut_ptr(), pk.add(MLKEM768_POLYVECBYTES), 32);

    polyvec_frombytes(&mut pkpv, pk);

    poly_frommsg(&mut k, m);

    gen_matrix(at.as_mut_ptr(), seed.as_ptr(), 1);

    i = 0;
    while i < MLKEM768_K {
        poly_getnoise_eta2(&mut sp.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
        i += 1;
    }
    i = 0;
    while i < MLKEM768_K {
        poly_getnoise_eta2(&mut ep.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
        i += 1;
    }
    poly_getnoise_eta2(&mut epp, coins, nonce);
    nonce = nonce.wrapping_add(1);

    polyvec_ntt(&mut sp);
    polyvec_reduce(&mut sp);

    i = 0;
    while i < MLKEM768_K {
        polyvec_basemul_acc(&mut b.vec[i], &at[i], &sp);
        i += 1;
    }

    polyvec_basemul_acc(&mut v, &pkpv, &sp);

    polyvec_invntt(&mut b);
    poly_invntt(&mut v);

    /* Outputs alias the first inputs here; use raw pointers. */
    let b_p: *mut polyvec = &mut b;
    let v_p: *mut poly = &mut v;
    polyvec_add(b_p, b_p, &ep);
    poly_add(v_p, v_p, &epp);
    poly_add(v_p, v_p, &k);

    polyvec_reduce(&mut b);
    poly_reduce(&mut v);
    polyvec_csubq(&mut b);
    poly_csubq(&mut v);

    polyvec_compress(ct, &b);
    poly_compress_dv(ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU), &v);
    sodium_memzero(
        &mut sp as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
    sodium_memzero(
        &mut ep as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
    sodium_memzero(
        &mut epp as *mut poly as *mut c_void,
        core::mem::size_of::<poly>(),
    );
    sodium_memzero(
        &mut k as *mut poly as *mut c_void,
        core::mem::size_of::<poly>(),
    );
}

/// ```c
/// static void indcpa_dec(unsigned char m[32],
///                        const unsigned char ct[crypto_kem_mlkem768_CIPHERTEXTBYTES],
///                        const unsigned char sk[MLKEM768_POLYVECBYTES]);
/// ```
unsafe fn indcpa_dec(m: *mut u8, ct: *const u8, sk: *const u8) {
    let mut b: polyvec = POLYVEC_ZERO;
    let mut skpv: polyvec = POLYVEC_ZERO;
    let mut v: poly = POLY_ZERO;
    let mut mp: poly = POLY_ZERO;

    polyvec_decompress(&mut b, ct);
    poly_decompress_dv(&mut v, ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU));

    polyvec_frombytes(&mut skpv, sk);

    polyvec_ntt(&mut b);
    polyvec_reduce(&mut b);
    polyvec_basemul_acc(&mut mp, &skpv, &b);
    poly_invntt(&mut mp);

    /* `poly_sub(&mp, &v, &mp)` — output aliases the second input. */
    let mp_p: *mut poly = &mut mp;
    poly_sub(mp_p, &v, mp_p);
    poly_reduce(&mut mp);
    poly_csubq(&mut mp);

    poly_tomsg(m, &mp);
    sodium_memzero(
        &mut skpv as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
    sodium_memzero(
        &mut mp as *mut poly as *mut c_void,
        core::mem::size_of::<poly>(),
    );
}

/// `static void cmov(unsigned char *r, const unsigned char *x, size_t len, unsigned char b)`
unsafe fn cmov(r: *mut u8, x: *const u8, len: usize, b: u8) {
    let mut i: usize;
    let mask: u8;

    mask = (b as c_int).wrapping_neg() as u8;

    i = 0;
    while i < len {
        *r.add(i) ^= mask & (*r.add(i) ^ *x.add(i));
        i += 1;
    }
}

/* ------------------------------------------------------------------ */
/* Public (renamed) API                                               */
/* ------------------------------------------------------------------ */

/// `int mlkem768_ref_seed_keypair(unsigned char *pk, unsigned char *sk, const unsigned char *seed)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut indseed = [0u8; 33];

    memcpy(indseed.as_mut_ptr(), seed, 32);
    indseed[32] = MLKEM768_K as u8;

    indcpa_keypair(pk, sk, indseed.as_ptr());
    sodium_memzero(indseed.as_mut_ptr() as *mut c_void, 33);
    memcpy(
        sk.add(MLKEM768_POLYVECBYTES),
        pk,
        crypto_kem_mlkem768_PUBLICKEYBYTES,
    );
    crypto_hash_sha3256(
        sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES),
        pk,
        crypto_kem_mlkem768_PUBLICKEYBYTES as c_ulonglong,
    );
    memcpy(
        sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES + 32),
        seed.add(32),
        32,
    );

    0
}

/// `int mlkem768_ref_keypair(unsigned char *pk, unsigned char *sk)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; crypto_kem_mlkem768_SEEDBYTES];
    let ret: c_int;

    randombytes_buf(
        seed.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SEEDBYTES,
    );
    ret = _sodium_mlkem768_ref_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(
        seed.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SEEDBYTES,
    );

    ret
}

/// ```c
/// int mlkem768_ref_enc_deterministic(unsigned char *ct, unsigned char *ss,
///                                    const unsigned char *pk, const unsigned char *seed);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let mut pkpv: polyvec = POLYVEC_ZERO;
    let mut buf = [0u8; 64];
    let mut kr = [0u8; 64];

    polyvec_frombytes(&mut pkpv, pk);
    if polyvec_is_canonical(&pkpv) == 0 {
        return -1;
    }

    memcpy(buf.as_mut_ptr(), seed, 32);
    crypto_hash_sha3256(
        buf.as_mut_ptr().add(32),
        pk,
        crypto_kem_mlkem768_PUBLICKEYBYTES as c_ulonglong,
    );

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(ct, buf.as_ptr(), pk, kr.as_ptr().add(32));

    memcpy(ss, kr.as_ptr(), crypto_kem_mlkem768_SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, 64);

    0
}

/// `int mlkem768_ref_enc(unsigned char *ct, unsigned char *ss, const unsigned char *pk)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed = [0u8; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
    ret = _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, 32);

    ret
}

/// `int mlkem768_ref_dec(unsigned char *ss, const unsigned char *ct, const unsigned char *sk)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    let mut buf = [0u8; 64];
    let mut kr = [0u8; 64];
    let mut k_bar = [0u8; crypto_kem_mlkem768_SHAREDSECRETBYTES];
    let mut cmp = [0u8; crypto_kem_mlkem768_CIPHERTEXTBYTES];
    let pk: *const u8 = sk.add(MLKEM768_POLYVECBYTES);
    let hpk: *const u8 = sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES);
    let z: *const u8 = sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES + 32);
    let fail: c_int;
    let mut fail_mask: c_uint_local;
    let mut state = crypto_xof_shake256_state { opaque: [0u8; 256] };

    indcpa_dec(buf.as_mut_ptr(), ct, sk);

    memcpy(buf.as_mut_ptr().add(32), hpk, 32);

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(cmp.as_mut_ptr(), buf.as_ptr(), pk, kr.as_ptr().add(32));

    fail = sodium_memcmp(
        ct as *const c_void,
        cmp.as_ptr() as *const c_void,
        crypto_kem_mlkem768_CIPHERTEXTBYTES,
    );
    fail_mask = fail as c_uint_local;
    fail_mask >>= core::mem::size_of::<c_uint_local>() * 8 - 1;

    crypto_xof_shake256_init(&mut state);
    crypto_xof_shake256_update(&mut state, z, 32);
    crypto_xof_shake256_update(
        &mut state,
        ct,
        crypto_kem_mlkem768_CIPHERTEXTBYTES as c_ulonglong,
    );
    crypto_xof_shake256_squeeze(
        &mut state,
        k_bar.as_mut_ptr(),
        crypto_kem_mlkem768_SHAREDSECRETBYTES,
    );

    cmov(
        kr.as_mut_ptr(),
        k_bar.as_ptr(),
        crypto_kem_mlkem768_SHAREDSECRETBYTES,
        fail_mask as u8,
    );

    memcpy(ss, kr.as_ptr(), crypto_kem_mlkem768_SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, 64);
    sodium_memzero(
        k_bar.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_SHAREDSECRETBYTES,
    );
    sodium_memzero(
        cmp.as_mut_ptr() as *mut c_void,
        crypto_kem_mlkem768_CIPHERTEXTBYTES,
    );
    sodium_memzero(
        &mut state as *mut crypto_xof_shake256_state as *mut c_void,
        core::mem::size_of::<crypto_xof_shake256_state>(),
    );

    0
}
