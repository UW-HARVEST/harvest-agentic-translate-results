//! Translation of `crypto_kem/mlkem768/ref/kem_mlkem768_ref.c`.
//!
//! The public entry points are renamed by `include/sodium/private/quirks.h`
//! (`mlkem768_ref_*` -> `_sodium_mlkem768_ref_*`), which is the name used by
//! the linker, so that is the exported symbol name here.
//!
//! `HAVE_INLINE_ASM` is **not** defined in the reference build, so `cmov()`
//! carries no `__asm__` barrier.
//!
//! All of the polynomial arithmetic is `int16_t`/`int32_t` with deliberate
//! truncation: every C expression below is reproduced with the exact same
//! promotion/truncation width.

use core::ffi::{c_int, c_void};

use crate::common::memcpy;
use crate::randombytes::randombytes_buf;
use crate::sodium::utils::{sodium_memcmp, sodium_memzero};

// ---------------------------------------------------------------------------
// Constants (include/sodium/crypto_kem_mlkem768.h, crypto_xof_shake128.h)
// ---------------------------------------------------------------------------

const crypto_kem_mlkem768_PUBLICKEYBYTES: usize = 1184;
const crypto_kem_mlkem768_SECRETKEYBYTES: usize = 2400;
const crypto_kem_mlkem768_CIPHERTEXTBYTES: usize = 1088;
const crypto_kem_mlkem768_SHAREDSECRETBYTES: usize = 32;
const crypto_kem_mlkem768_SEEDBYTES: usize = 64;

const crypto_xof_shake128_BLOCKBYTES: usize = 168;

const MLKEM768_Q: i32 = 3329;
const MLKEM768_N: usize = 256;
const MLKEM768_K: usize = 3;
const MLKEM768_ETA2: usize = 2;

const MLKEM768_POLYBYTES: usize = 384;
const MLKEM768_POLYVECBYTES: usize = MLKEM768_K * MLKEM768_POLYBYTES;
const MLKEM768_POLYCOMPRESSEDBYTES_DU: usize = 320;
const MLKEM768_POLYCOMPRESSEDBYTES_DV: usize = 128;
const MLKEM768_POLYVECCOMPRESSEDBYTES_DU: usize = MLKEM768_K * MLKEM768_POLYCOMPRESSEDBYTES_DU;

// ---------------------------------------------------------------------------
// Cross-file declarations
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct CRYPTO_ALIGN(16) crypto_xof_shake128_state {
///     unsigned char opaque[256];
/// } crypto_xof_shake128_state;
/// ```
#[repr(C, align(16))]
struct crypto_xof_shake128_state {
    opaque: [u8; 256],
}

/// ```c
/// typedef struct CRYPTO_ALIGN(16) crypto_xof_shake256_state {
///     unsigned char opaque[256];
/// } crypto_xof_shake256_state;
/// ```
#[repr(C, align(16))]
struct crypto_xof_shake256_state {
    opaque: [u8; 256],
}

unsafe extern "C" {
    fn crypto_hash_sha3256(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha3512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn crypto_xof_shake128_init(state: *mut crypto_xof_shake128_state) -> c_int;
    fn crypto_xof_shake128_update(
        state: *mut crypto_xof_shake128_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_xof_shake128_squeeze(
        state: *mut crypto_xof_shake128_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
    fn crypto_xof_shake256_init(state: *mut crypto_xof_shake256_state) -> c_int;
    fn crypto_xof_shake256_update(
        state: *mut crypto_xof_shake256_state,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_xof_shake256_squeeze(
        state: *mut crypto_xof_shake256_state,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// poly / polyvec
// ---------------------------------------------------------------------------

/// ```c
/// typedef struct poly { int16_t coeffs[MLKEM768_N]; } poly;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
struct poly {
    coeffs: [i16; MLKEM768_N],
}

/// ```c
/// typedef struct polyvec { poly vec[MLKEM768_K]; } polyvec;
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
struct polyvec {
    vec: [poly; MLKEM768_K],
}

const POLY_ZERO: poly = poly { coeffs: [0; MLKEM768_N] };
const POLYVEC_ZERO: polyvec = polyvec { vec: [POLY_ZERO; MLKEM768_K] };

/// `&pv->vec[i]` without forming a Rust reference (the C code freely aliases
/// the destination with a source operand).
#[inline(always)]
unsafe fn vec_at(pv: *const polyvec, i: usize) -> *const poly {
    unsafe { (pv as *const poly).add(i) }
}

#[inline(always)]
unsafe fn vec_at_mut(pv: *mut polyvec, i: usize) -> *mut poly {
    unsafe { (pv as *mut poly).add(i) }
}

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
    t = ((a as i32).wrapping_sub((t as i32).wrapping_mul(MLKEM768_Q))) as i16;

    t
}

/// ```c
/// a -= MLKEM768_Q;
/// a += (a >> 15) & MLKEM768_Q;
/// ```
#[inline]
fn csubq(a: i16) -> i16 {
    let mut a = a;

    a = a.wrapping_sub(MLKEM768_Q as i16);
    a = a.wrapping_add((((a as i32) >> 15) & MLKEM768_Q) as i16);

    a
}

#[allow(unused_assignments)]
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
        j = 0;
        while start < MLKEM768_N {
            zeta = zetas[k];
            k += 1;
            j = start;
            while j < start + len {
                unsafe {
                    t = montgomery_reduce((zeta as i32).wrapping_mul((*r).coeffs[j + len] as i32));
                    (*r).coeffs[j + len] = (*r).coeffs[j].wrapping_sub(t);
                    (*r).coeffs[j] = (*r).coeffs[j].wrapping_add(t);
                }
                j += 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

#[allow(unused_assignments)]
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
        j = 0;
        while start < MLKEM768_N {
            zeta = zetas[k];
            k = k.wrapping_sub(1);
            j = start;
            while j < start + len {
                unsafe {
                    t = (*r).coeffs[j];
                    (*r).coeffs[j] = barrett_reduce(t.wrapping_add((*r).coeffs[j + len]));
                    // The subtraction happens in `int`, *before* the widening
                    // multiplication: no int16_t truncation here.
                    (*r).coeffs[j + len] = montgomery_reduce(
                        (zeta as i32)
                            .wrapping_mul(((*r).coeffs[j + len] as i32).wrapping_sub(t as i32)),
                    );
                }
                j += 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    j = 0;
    while j < MLKEM768_N {
        unsafe {
            (*r).coeffs[j] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[j] as i32));
        }
        j += 1;
    }
}

unsafe fn poly_basemul(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;
    let mut zeta: i16;

    i = 0;
    while i < MLKEM768_N / 4 {
        zeta = zetas[64 + i];

        unsafe {
            (*r).coeffs[4 * i] = montgomery_reduce(
                ((*a).coeffs[4 * i + 1] as i32).wrapping_mul((*b).coeffs[4 * i + 1] as i32),
            );
            (*r).coeffs[4 * i] = montgomery_reduce(
                ((*r).coeffs[4 * i] as i32).wrapping_mul(zeta as i32),
            );
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
        }
        i += 1;
    }
}

unsafe fn poly_tomont(r: *mut poly) {
    let f: i16 = 1353;

    for i in 0..MLKEM768_N {
        unsafe {
            (*r).coeffs[i] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[i] as i32));
        }
    }
}

unsafe fn poly_reduce(r: *mut poly) {
    for i in 0..MLKEM768_N {
        unsafe {
            (*r).coeffs[i] = barrett_reduce((*r).coeffs[i]);
        }
    }
}

unsafe fn poly_add(r: *mut poly, a: *const poly, b: *const poly) {
    for i in 0..MLKEM768_N {
        unsafe {
            (*r).coeffs[i] = (*a).coeffs[i].wrapping_add((*b).coeffs[i]);
        }
    }
}

unsafe fn poly_sub(r: *mut poly, a: *const poly, b: *const poly) {
    for i in 0..MLKEM768_N {
        unsafe {
            (*r).coeffs[i] = (*a).coeffs[i].wrapping_sub((*b).coeffs[i]);
        }
    }
}

unsafe fn poly_csubq(r: *mut poly) {
    for i in 0..MLKEM768_N {
        unsafe {
            (*r).coeffs[i] = csubq((*r).coeffs[i]);
        }
    }
}

unsafe fn poly_cbd_eta2(r: *mut poly, buf: *const u8) {
    let mut t: u32;
    let mut d: u32;
    let mut a: i16;
    let mut b: i16;

    for i in 0..MLKEM768_N / 8 {
        unsafe {
            t = (*buf.add(4 * i) as u32)
                | ((*buf.add(4 * i + 1) as u32) << 8)
                | ((*buf.add(4 * i + 2) as u32) << 16)
                | ((*buf.add(4 * i + 3) as u32) << 24);

            d = t & 0x55555555;
            d = d.wrapping_add((t >> 1) & 0x55555555);

            for j in 0..8usize {
                a = ((d >> (4 * j)) & 0x3) as i16;
                b = ((d >> (4 * j + 2)) & 0x3) as i16;
                (*r).coeffs[8 * i + j] = a.wrapping_sub(b);
            }
        }
    }
}

unsafe fn poly_getnoise_eta2(r: *mut poly, seed: *const u8, nonce: u8) {
    let mut buf = [0u8; MLKEM768_ETA2 * MLKEM768_N / 4];
    let mut state = crypto_xof_shake256_state { opaque: [0u8; 256] };
    let mut extseed = [0u8; 33];

    unsafe {
        memcpy(extseed.as_mut_ptr(), seed, 32);
        extseed[32] = nonce;

        crypto_xof_shake256_init(&mut state);
        crypto_xof_shake256_update(&mut state, extseed.as_ptr(), 33);
        crypto_xof_shake256_squeeze(&mut state, buf.as_mut_ptr(), buf.len());

        poly_cbd_eta2(r, buf.as_ptr());
        sodium_memzero(
            (&mut state) as *mut crypto_xof_shake256_state as *mut c_void,
            core::mem::size_of::<crypto_xof_shake256_state>(),
        );
        sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    }
}

unsafe fn poly_frombytes(r: *mut poly, a: *const u8) {
    for i in 0..MLKEM768_N / 2 {
        unsafe {
            (*r).coeffs[2 * i] = ((((*a.add(3 * i + 0) as u32) >> 0)
                | ((*a.add(3 * i + 1) as u32) << 8))
                & 0xFFF) as i16;
            (*r).coeffs[2 * i + 1] = ((((*a.add(3 * i + 1) as u32) >> 4)
                | ((*a.add(3 * i + 2) as u32) << 4))
                & 0xFFF) as i16;
        }
    }
}

unsafe fn poly_tobytes(r: *mut u8, a: *const poly) {
    let mut t0: u16;
    let mut t1: u16;

    for i in 0..MLKEM768_N / 2 {
        unsafe {
            t0 = (*a).coeffs[2 * i] as u16;
            t1 = (*a).coeffs[2 * i + 1] as u16;
            *r.add(3 * i + 0) = ((t0 as u32) >> 0) as u8;
            *r.add(3 * i + 1) = (((t0 as u32) >> 8) | ((t1 as u32) << 4)) as u8;
            *r.add(3 * i + 2) = ((t1 as u32) >> 4) as u8;
        }
    }
}

unsafe fn poly_frommsg(r: *mut poly, msg: *const u8) {
    let mut mask: i16;

    for i in 0..MLKEM768_N / 8 {
        for j in 0..8usize {
            unsafe {
                mask = (((*msg.add(i) as i32) >> j) & 1).wrapping_neg() as i16;
                (*r).coeffs[8 * i + j] = ((mask as i32) & ((MLKEM768_Q + 1) / 2)) as i16;
            }
        }
    }
}

unsafe fn poly_tomsg(msg: *mut u8, a: *const poly) {
    let mut t: u32;

    for i in 0..MLKEM768_N / 8 {
        unsafe {
            *msg.add(i) = 0;
            for j in 0..8usize {
                t = ((*a).coeffs[8 * i + j] as i32) as u32;
                t = t.wrapping_add(((((*a).coeffs[8 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
                t = ((t << 1).wrapping_add((MLKEM768_Q / 2) as u32)).wrapping_mul(80635) >> 28;
                t &= 1;
                *msg.add(i) |= (t << j) as u8;
            }
        }
    }
}

unsafe fn poly_compress_du(r: *mut u8, a: *const poly) {
    let mut t = [0u32; 4];

    for i in 0..MLKEM768_N / 4 {
        unsafe {
            for j in 0..4usize {
                t[j] = ((*a).coeffs[4 * i + j] as i32) as u32;
                t[j] = t[j]
                    .wrapping_add(((((*a).coeffs[4 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
                t[j] = ((((t[j] as u64) << 10).wrapping_add((MLKEM768_Q / 2) as u64))
                    .wrapping_mul(161271u64)
                    >> 29) as u32;
                t[j] &= 0x3ff;
            }

            *r.add(5 * i + 0) = (t[0] >> 0) as u8;
            *r.add(5 * i + 1) = ((t[0] >> 8) | (t[1] << 2)) as u8;
            *r.add(5 * i + 2) = ((t[1] >> 6) | (t[2] << 4)) as u8;
            *r.add(5 * i + 3) = ((t[2] >> 4) | (t[3] << 6)) as u8;
            *r.add(5 * i + 4) = (t[3] >> 2) as u8;
        }
    }
}

unsafe fn poly_decompress_du(r: *mut poly, a: *const u8) {
    let mut t = [0u16; 4];

    for i in 0..MLKEM768_N / 4 {
        unsafe {
            t[0] = (((*a.add(5 * i + 0) as u32) >> 0) | ((*a.add(5 * i + 1) as u32) << 8)) as u16;
            t[1] = (((*a.add(5 * i + 1) as u32) >> 2) | ((*a.add(5 * i + 2) as u32) << 6)) as u16;
            t[2] = (((*a.add(5 * i + 2) as u32) >> 4) | ((*a.add(5 * i + 3) as u32) << 4)) as u16;
            t[3] = (((*a.add(5 * i + 3) as u32) >> 6) | ((*a.add(5 * i + 4) as u32) << 2)) as u16;

            (*r).coeffs[4 * i + 0] = ((((t[0] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32))
                .wrapping_add(512)
                >> 10) as i16;
            (*r).coeffs[4 * i + 1] = ((((t[1] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32))
                .wrapping_add(512)
                >> 10) as i16;
            (*r).coeffs[4 * i + 2] = ((((t[2] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32))
                .wrapping_add(512)
                >> 10) as i16;
            (*r).coeffs[4 * i + 3] = ((((t[3] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32))
                .wrapping_add(512)
                >> 10) as i16;
        }
    }
}

unsafe fn poly_compress_dv(r: *mut u8, a: *const poly) {
    let mut t = [0u32; 8];

    for i in 0..MLKEM768_N / 8 {
        unsafe {
            for j in 0..8usize {
                t[j] = ((*a).coeffs[8 * i + j] as i32) as u32;
                t[j] = t[j]
                    .wrapping_add(((((*a).coeffs[8 * i + j] as i32) >> 15) & MLKEM768_Q) as u32);
                t[j] = ((((t[j] as u64) << 4).wrapping_add((MLKEM768_Q / 2) as u64))
                    .wrapping_mul(161271u64)
                    >> 29) as u32;
                t[j] &= 0xf;
            }

            *r.add(4 * i + 0) = (t[0] | (t[1] << 4)) as u8;
            *r.add(4 * i + 1) = (t[2] | (t[3] << 4)) as u8;
            *r.add(4 * i + 2) = (t[4] | (t[5] << 4)) as u8;
            *r.add(4 * i + 3) = (t[6] | (t[7] << 4)) as u8;
        }
    }
}

unsafe fn poly_decompress_dv(r: *mut poly, a: *const u8) {
    for i in 0..MLKEM768_N / 2 {
        unsafe {
            // `(uint16_t)(a[i] & 15)` is promoted to `int`, so the whole
            // expression is evaluated in `int`.
            (*r).coeffs[2 * i + 0] =
                (((((*a.add(i) & 15) as i32).wrapping_mul(MLKEM768_Q)).wrapping_add(8)) >> 4) as i16;
            (*r).coeffs[2 * i + 1] =
                (((((*a.add(i) >> 4) as i32).wrapping_mul(MLKEM768_Q)).wrapping_add(8)) >> 4) as i16;
        }
    }
}

unsafe fn polyvec_ntt(r: *mut polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_ntt(vec_at_mut(r, i)) };
    }
}

unsafe fn polyvec_invntt(r: *mut polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_invntt(vec_at_mut(r, i)) };
    }
}

unsafe fn polyvec_basemul_acc(r: *mut poly, a: *const polyvec, b: *const polyvec) {
    let mut t = POLY_ZERO;

    unsafe {
        poly_basemul(r, vec_at(a, 0), vec_at(b, 0));
        for i in 1..MLKEM768_K {
            poly_basemul(&mut t, vec_at(a, i), vec_at(b, i));
            poly_add(r, r, &t);
        }

        poly_reduce(r);
    }
}

unsafe fn polyvec_reduce(r: *mut polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_reduce(vec_at_mut(r, i)) };
    }
}

unsafe fn polyvec_csubq(r: *mut polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_csubq(vec_at_mut(r, i)) };
    }
}

unsafe fn polyvec_add(r: *mut polyvec, a: *const polyvec, b: *const polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_add(vec_at_mut(r, i), vec_at(a, i), vec_at(b, i)) };
    }
}

unsafe fn polyvec_tobytes(r: *mut u8, a: *const polyvec) {
    for i in 0..MLKEM768_K {
        unsafe { poly_tobytes(r.add(i * MLKEM768_POLYBYTES), vec_at(a, i)) };
    }
}

unsafe fn polyvec_frombytes(r: *mut polyvec, a: *const u8) {
    for i in 0..MLKEM768_K {
        unsafe { poly_frombytes(vec_at_mut(r, i), a.add(i * MLKEM768_POLYBYTES)) };
    }
}

unsafe fn polyvec_is_canonical(a: *const polyvec) -> c_int {
    for i in 0..MLKEM768_K {
        for j in 0..MLKEM768_N {
            unsafe {
                if ((*vec_at(a, i)).coeffs[j] as u16) as i32 >= MLKEM768_Q {
                    return 0;
                }
            }
        }
    }
    1
}

unsafe fn polyvec_compress(r: *mut u8, a: *const polyvec) {
    for i in 0..MLKEM768_K {
        unsafe {
            poly_compress_du(r.add(i * MLKEM768_POLYCOMPRESSEDBYTES_DU), vec_at(a, i));
        }
    }
}

unsafe fn polyvec_decompress(r: *mut polyvec, a: *const u8) {
    for i in 0..MLKEM768_K {
        unsafe {
            poly_decompress_du(vec_at_mut(r, i), a.add(i * MLKEM768_POLYCOMPRESSEDBYTES_DU));
        }
    }
}

unsafe fn rej_uniform(r: *mut i16, len: usize, buf: *const u8, buflen: usize) -> usize {
    let mut ctr: usize;
    let mut pos: usize;
    let mut val0: u16;
    let mut val1: u16;

    ctr = 0;
    pos = 0;
    /* Variable-time rejection is fine here: callers only use public matrix seeds. */
    while ctr < len && pos + 3 <= buflen {
        unsafe {
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
    }

    ctr
}

/// ```c
/// #define GEN_MATRIX_NBLOCKS \
///     ((12 * MLKEM768_N / 8 * (1 << 12) / MLKEM768_Q + crypto_xof_shake128_BLOCKBYTES) / \
///      crypto_xof_shake128_BLOCKBYTES)
/// ```
const GEN_MATRIX_NBLOCKS: usize = (12 * MLKEM768_N / 8 * (1 << 12) / (MLKEM768_Q as usize)
    + crypto_xof_shake128_BLOCKBYTES)
    / crypto_xof_shake128_BLOCKBYTES;

unsafe fn gen_matrix(a: *mut polyvec, seed: *const u8, transposed: c_int) {
    let mut state = crypto_xof_shake128_state { opaque: [0u8; 256] };
    let mut buf = [0u8; GEN_MATRIX_NBLOCKS * crypto_xof_shake128_BLOCKBYTES + 2];
    let mut extseed = [0u8; 34];
    let mut ctr: usize;
    let mut buflen: usize;

    unsafe {
        memcpy(extseed.as_mut_ptr(), seed, 32);

        for i in 0..MLKEM768_K {
            for j in 0..MLKEM768_K {
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

                // &a[i].vec[j].coeffs[0]
                let coeffs = vec_at_mut(a.add(i), j) as *mut i16;

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
            }
        }
    }
}

unsafe fn indcpa_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) {
    let mut a = [POLYVEC_ZERO; MLKEM768_K];
    let mut e = POLYVEC_ZERO;
    let mut pkpv = POLYVEC_ZERO;
    let mut skpv = POLYVEC_ZERO;
    let mut buf = [0u8; 64];
    let mut nonce: u8 = 0;

    let a_p: *mut polyvec = a.as_mut_ptr();
    let e_p: *mut polyvec = &mut e;
    let pkpv_p: *mut polyvec = &mut pkpv;
    let skpv_p: *mut polyvec = &mut skpv;

    unsafe {
        let publicseed: *mut u8 = buf.as_mut_ptr();
        let noiseseed: *mut u8 = buf.as_mut_ptr().add(32);

        crypto_hash_sha3512(buf.as_mut_ptr(), seed, 33);

        gen_matrix(a_p, publicseed, 0);

        for i in 0..MLKEM768_K {
            poly_getnoise_eta2(vec_at_mut(skpv_p, i), noiseseed, nonce);
            nonce = nonce.wrapping_add(1);
        }
        for i in 0..MLKEM768_K {
            poly_getnoise_eta2(vec_at_mut(e_p, i), noiseseed, nonce);
            nonce = nonce.wrapping_add(1);
        }

        polyvec_ntt(skpv_p);
        polyvec_ntt(e_p);

        for i in 0..MLKEM768_K {
            polyvec_basemul_acc(vec_at_mut(pkpv_p, i), a_p.add(i), skpv_p);
            poly_tomont(vec_at_mut(pkpv_p, i));
        }

        polyvec_add(pkpv_p, pkpv_p, e_p);
        polyvec_reduce(pkpv_p);
        polyvec_csubq(pkpv_p);
        polyvec_reduce(skpv_p);
        polyvec_csubq(skpv_p);

        polyvec_tobytes(sk, skpv_p);
        polyvec_tobytes(pk, pkpv_p);
        memcpy(pk.add(MLKEM768_POLYVECBYTES), publicseed, 32);
        sodium_memzero(buf.as_mut_ptr() as *mut c_void, 64);
        sodium_memzero(skpv_p as *mut c_void, core::mem::size_of::<polyvec>());
        sodium_memzero(e_p as *mut c_void, core::mem::size_of::<polyvec>());
    }
}

unsafe fn indcpa_enc(ct: *mut u8, m: *const u8, pk: *const u8, coins: *const u8) {
    let mut sp = POLYVEC_ZERO;
    let mut pkpv = POLYVEC_ZERO;
    let mut ep = POLYVEC_ZERO;
    let mut at = [POLYVEC_ZERO; MLKEM768_K];
    let mut b = POLYVEC_ZERO;
    let mut v = POLY_ZERO;
    let mut k = POLY_ZERO;
    let mut epp = POLY_ZERO;
    let mut seed = [0u8; 32];
    let mut nonce: u8 = 0;

    let sp_p: *mut polyvec = &mut sp;
    let pkpv_p: *mut polyvec = &mut pkpv;
    let ep_p: *mut polyvec = &mut ep;
    let at_p: *mut polyvec = at.as_mut_ptr();
    let b_p: *mut polyvec = &mut b;
    let v_p: *mut poly = &mut v;
    let k_p: *mut poly = &mut k;
    let epp_p: *mut poly = &mut epp;

    unsafe {
        memcpy(seed.as_mut_ptr(), pk.add(MLKEM768_POLYVECBYTES), 32);

        polyvec_frombytes(pkpv_p, pk);

        poly_frommsg(k_p, m);

        gen_matrix(at_p, seed.as_ptr(), 1);

        for i in 0..MLKEM768_K {
            poly_getnoise_eta2(vec_at_mut(sp_p, i), coins, nonce);
            nonce = nonce.wrapping_add(1);
        }
        for i in 0..MLKEM768_K {
            poly_getnoise_eta2(vec_at_mut(ep_p, i), coins, nonce);
            nonce = nonce.wrapping_add(1);
        }
        poly_getnoise_eta2(epp_p, coins, nonce);
        nonce = nonce.wrapping_add(1);
        let _ = nonce;

        polyvec_ntt(sp_p);
        polyvec_reduce(sp_p);

        for i in 0..MLKEM768_K {
            polyvec_basemul_acc(vec_at_mut(b_p, i), at_p.add(i), sp_p);
        }

        polyvec_basemul_acc(v_p, pkpv_p, sp_p);

        polyvec_invntt(b_p);
        poly_invntt(v_p);

        polyvec_add(b_p, b_p, ep_p);
        poly_add(v_p, v_p, epp_p);
        poly_add(v_p, v_p, k_p);

        polyvec_reduce(b_p);
        poly_reduce(v_p);
        polyvec_csubq(b_p);
        poly_csubq(v_p);

        polyvec_compress(ct, b_p);
        poly_compress_dv(ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU), v_p);
        sodium_memzero(sp_p as *mut c_void, core::mem::size_of::<polyvec>());
        sodium_memzero(ep_p as *mut c_void, core::mem::size_of::<polyvec>());
        sodium_memzero(epp_p as *mut c_void, core::mem::size_of::<poly>());
        sodium_memzero(k_p as *mut c_void, core::mem::size_of::<poly>());
    }
}

unsafe fn indcpa_dec(m: *mut u8, ct: *const u8, sk: *const u8) {
    let mut b = POLYVEC_ZERO;
    let mut skpv = POLYVEC_ZERO;
    let mut v = POLY_ZERO;
    let mut mp = POLY_ZERO;

    let b_p: *mut polyvec = &mut b;
    let skpv_p: *mut polyvec = &mut skpv;
    let v_p: *mut poly = &mut v;
    let mp_p: *mut poly = &mut mp;

    unsafe {
        polyvec_decompress(b_p, ct);
        poly_decompress_dv(v_p, ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU));

        polyvec_frombytes(skpv_p, sk);

        polyvec_ntt(b_p);
        polyvec_reduce(b_p);
        polyvec_basemul_acc(mp_p, skpv_p, b_p);
        poly_invntt(mp_p);

        poly_sub(mp_p, v_p, mp_p);
        poly_reduce(mp_p);
        poly_csubq(mp_p);

        poly_tomsg(m, mp_p);
        sodium_memzero(skpv_p as *mut c_void, core::mem::size_of::<polyvec>());
        sodium_memzero(mp_p as *mut c_void, core::mem::size_of::<poly>());
    }
}

/// `HAVE_INLINE_ASM` is undefined -> no `__asm__` barrier on `mask`.
unsafe fn cmov(r: *mut u8, x: *const u8, len: usize, b: u8) {
    let mask: u8;

    mask = ((b as c_int).wrapping_neg()) as u8;

    for i in 0..len {
        unsafe {
            *r.add(i) ^= mask & (*r.add(i) ^ *x.add(i));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut indseed = [0u8; 33];

    unsafe {
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
            crypto_kem_mlkem768_PUBLICKEYBYTES as u64,
        );
        memcpy(
            sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES + 32),
            seed.add(32),
            32,
        );
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; crypto_kem_mlkem768_SEEDBYTES];
    let ret: c_int;

    unsafe {
        randombytes_buf(
            seed.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SEEDBYTES,
        );
        ret = _sodium_mlkem768_ref_seed_keypair(pk, sk, seed.as_ptr());
        sodium_memzero(
            seed.as_mut_ptr() as *mut c_void,
            crypto_kem_mlkem768_SEEDBYTES,
        );
    }

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let mut pkpv = POLYVEC_ZERO;
    let mut buf = [0u8; 64];
    let mut kr = [0u8; 64];

    unsafe {
        polyvec_frombytes(&mut pkpv, pk);
        if polyvec_is_canonical(&pkpv) == 0 {
            return -1;
        }

        memcpy(buf.as_mut_ptr(), seed, 32);
        crypto_hash_sha3256(
            buf.as_mut_ptr().add(32),
            pk,
            crypto_kem_mlkem768_PUBLICKEYBYTES as u64,
        );

        crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

        indcpa_enc(ct, buf.as_ptr(), pk, kr.as_ptr().add(32));

        memcpy(ss, kr.as_ptr(), crypto_kem_mlkem768_SHAREDSECRETBYTES);
        sodium_memzero(buf.as_mut_ptr() as *mut c_void, 64);
        sodium_memzero(kr.as_mut_ptr() as *mut c_void, 64);
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed = [0u8; 32];
    let ret: c_int;

    unsafe {
        randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
        ret = _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed.as_ptr());
        sodium_memzero(seed.as_mut_ptr() as *mut c_void, 32);
    }

    ret
}

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
    let fail: c_int;
    let mut fail_mask: u32;
    let mut state = crypto_xof_shake256_state { opaque: [0u8; 256] };

    unsafe {
        let pk: *const u8 = sk.add(MLKEM768_POLYVECBYTES);
        let hpk: *const u8 = sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES);
        let z: *const u8 =
            sk.add(MLKEM768_POLYVECBYTES + crypto_kem_mlkem768_PUBLICKEYBYTES + 32);

        indcpa_dec(buf.as_mut_ptr(), ct, sk);

        memcpy(buf.as_mut_ptr().add(32), hpk, 32);

        crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

        indcpa_enc(cmp.as_mut_ptr(), buf.as_ptr(), pk, kr.as_ptr().add(32));

        fail = sodium_memcmp(
            ct as *const c_void,
            cmp.as_ptr() as *const c_void,
            crypto_kem_mlkem768_CIPHERTEXTBYTES,
        );
        fail_mask = fail as u32;
        fail_mask >>= core::mem::size_of::<u32>() as u32 * 8u32 - 1u32;

        crypto_xof_shake256_init(&mut state);
        crypto_xof_shake256_update(&mut state, z, 32);
        crypto_xof_shake256_update(&mut state, ct, crypto_kem_mlkem768_CIPHERTEXTBYTES as u64);
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
            (&mut state) as *mut crypto_xof_shake256_state as *mut c_void,
            core::mem::size_of::<crypto_xof_shake256_state>(),
        );
    }

    0
}
