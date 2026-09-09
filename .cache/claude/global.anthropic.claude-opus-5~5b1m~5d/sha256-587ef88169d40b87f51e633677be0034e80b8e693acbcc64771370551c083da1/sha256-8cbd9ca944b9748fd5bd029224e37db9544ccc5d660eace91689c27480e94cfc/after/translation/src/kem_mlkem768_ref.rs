//! Translation of `crypto_kem/mlkem768/ref/kem_mlkem768_ref.c`.
//!
//! ML-KEM-768 (Kyber768, `KYBER_K = 3`) reference implementation:
//! Montgomery/Barrett-reduced NTT arithmetic over `poly`/`polyvec`,
//! (de)compression, the IND-CPA public-key encryption scheme
//! (`indcpa_keypair`/`indcpa_enc`/`indcpa_dec`), and the Fujisaki-Okamoto
//! transform that turns it into an IND-CCA2 KEM.
//!
//! Headers: `crypto_kem/mlkem768/ref/kem_mlkem768_ref.h` (declares the five
//! public entry points below with their un-prefixed names) and
//! `include/sodium/crypto_kem_mlkem768.h` (the public
//! `crypto_kem_mlkem768_*` API implemented on top of this file, in
//! `kem_mlkem768.c` / `kem.rs`).
//!
//! Every function in the C source except the five listed in
//! `kem_mlkem768_ref.h` is `static`; `private/quirks.h` renames those five
//! (`mlkem768_ref_keypair` -> `_sodium_mlkem768_ref_keypair`, etc.) and
//! they are the only symbols from this translation unit that appear in the
//! final shared object (see `_cbuild/persym.txt`). All other functions
//! below (`poly_*`, `polyvec_*`, `indcpa_*`, `gen_matrix`, `rej_uniform`,
//! `cmov`, the reductions) are translated as private Rust functions with
//! no `#[no_mangle]`, exactly mirroring their `static` C linkage.

use core::ffi::{c_int, c_void};

use crate::csys::memcpy;

// ---------------------------------------------------------------------
// Cross-module declarations (SHA3-256/512, SHAKE128/256 XOFs, misc utils)
// ---------------------------------------------------------------------

/// `crypto_xof_shake128_state` from `crypto_xof_shake128.h`: an opaque,
/// 16-byte-aligned, 256-byte blob. Duplicated locally per translation
/// conventions (cross-module calls only share the final linker name, not
/// the Rust type).
#[repr(C, align(16))]
struct crypto_xof_shake128_state {
    opaque: [u8; 256],
}

/// `crypto_xof_shake256_state` from `crypto_xof_shake256.h`.
#[repr(C, align(16))]
struct crypto_xof_shake256_state {
    opaque: [u8; 256],
}

extern "C" {
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

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn randombytes_buf(buf: *mut u8, size: usize);
}

// ---------------------------------------------------------------------
// Kyber / ML-KEM-768 parameters (KYBER_K = 3)
// ---------------------------------------------------------------------

const KYBER_N: usize = 256;
const KYBER_Q: i32 = 3329;
const KYBER_K: usize = 3;
const KYBER_SYMBYTES: usize = 32;
const KYBER_POLYBYTES: usize = 384;
const KYBER_POLYVECBYTES: usize = KYBER_K * KYBER_POLYBYTES; // 1152
const KYBER_POLYCOMPRESSEDBYTES_DV: usize = 128;
const KYBER_POLYVECCOMPRESSEDBYTES_DU: usize = KYBER_K * 320; // 960

const KYBER_INDCPA_PUBLICKEYBYTES: usize = KYBER_POLYVECBYTES + KYBER_SYMBYTES; // 1184
const KYBER_INDCPA_SECRETKEYBYTES: usize = KYBER_POLYVECBYTES; // 1152
const KYBER_INDCPA_BYTES: usize = KYBER_POLYVECCOMPRESSEDBYTES_DU + KYBER_POLYCOMPRESSEDBYTES_DV; // 1088

const KYBER_PUBLICKEYBYTES: usize = KYBER_INDCPA_PUBLICKEYBYTES; // 1184
const KYBER_SECRETKEYBYTES: usize =
    KYBER_INDCPA_SECRETKEYBYTES + KYBER_INDCPA_PUBLICKEYBYTES + 2 * KYBER_SYMBYTES; // 2400
const KYBER_CIPHERTEXTBYTES: usize = KYBER_INDCPA_BYTES; // 1088

// gen_matrix buffer size: ((12*256/8*(1<<12)/3329 + 168) / 168) * 168 [+2]
const GEN_MATRIX_BUFLEN: usize = ((12 * 256 / 8 * (1usize << 12) / 3329 + 168) / 168) * 168;
const GEN_MATRIX_BUF_SIZE: usize = GEN_MATRIX_BUFLEN + 2;

// ---------------------------------------------------------------------
// poly / polyvec types
// ---------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct poly {
    coeffs: [i16; KYBER_N],
}

impl poly {
    const fn zeroed() -> poly {
        poly {
            coeffs: [0i16; KYBER_N],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct polyvec {
    vec: [poly; KYBER_K],
}

impl polyvec {
    const fn zeroed() -> polyvec {
        polyvec {
            vec: [poly::zeroed(); KYBER_K],
        }
    }
}

static ZETAS: [i16; 128] = [
    2285, 2571, 2970, 1812, 1493, 1422, 287, 202, 3158, 622, 1577, 182, 962, 2127, 1855, 1468,
    573, 2004, 264, 383, 2500, 1458, 1727, 3199, 2648, 1017, 732, 608, 1787, 411, 3124, 1758,
    1223, 652, 2777, 1015, 2036, 1491, 3047, 1785, 516, 3321, 3009, 2663, 1711, 2167, 126, 1469,
    2476, 3239, 3058, 830, 107, 1908, 3082, 2378, 2931, 961, 1821, 2604, 448, 2264, 677, 2054,
    2226, 430, 555, 843, 2078, 871, 1550, 105, 422, 587, 177, 3094, 3038, 2869, 1574, 1653, 3083,
    778, 1159, 3182, 2552, 1483, 2727, 1119, 1739, 644, 2457, 349, 418, 329, 3173, 3254, 817,
    1097, 603, 610, 1322, 2044, 1864, 384, 2114, 3193, 1218, 1994, 2455, 220, 2142, 1670, 2144,
    1799, 2051, 794, 1819, 2475, 2459, 478, 3221, 3021, 996, 991, 958, 1869, 1522, 1628,
];

// ---------------------------------------------------------------------
// Reductions
// ---------------------------------------------------------------------

fn montgomery_reduce(a: i32) -> i16 {
    let mut t: i16;

    t = (((a as u16 as u32).wrapping_mul(62209u32)) as u16) as i16;
    t = (a.wrapping_sub((t as i32).wrapping_mul(3329)) >> 16) as i16;

    t
}

fn barrett_reduce(a: i16) -> i16 {
    let mut t: i16;

    t = (((a as i32).wrapping_mul(20159)) >> 26) as i16;
    t = a.wrapping_sub(t.wrapping_mul(3329));

    t
}

fn csubq(a: i16) -> i16 {
    let mut a = a;
    a = a.wrapping_sub(3329);
    a = a.wrapping_add((a >> 15) & 3329);

    a
}

// ---------------------------------------------------------------------
// poly_* / polyvec_* helpers (all `static` in C)
// ---------------------------------------------------------------------

unsafe fn poly_ntt(r: *mut poly) {
    let mut len: u32;
    let mut start: u32;
    let mut j: u32;
    let mut k: u32;
    let mut t: i16;
    let mut zeta: i16;

    k = 1;
    len = 128;
    while len >= 2 {
        start = 0;
        while start < 256 {
            zeta = ZETAS[k as usize];
            k += 1;
            j = start;
            while j < start + len {
                t = montgomery_reduce(
                    (zeta as i32).wrapping_mul((*r).coeffs[(j + len) as usize] as i32),
                );
                (*r).coeffs[(j + len) as usize] = (*r).coeffs[j as usize].wrapping_sub(t);
                (*r).coeffs[j as usize] = (*r).coeffs[j as usize].wrapping_add(t);
                j = j + 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

unsafe fn poly_invntt(r: *mut poly) {
    let mut start: u32;
    let mut len: u32;
    let mut j: u32;
    let mut k: i32;
    let mut t: i16;
    let mut zeta: i16;
    let f: i16 = 1441;

    k = 127;
    len = 2;
    while len <= 128 {
        start = 0;
        while start < 256 {
            zeta = ZETAS[k as usize];
            k -= 1;
            j = start;
            while j < start + len {
                t = (*r).coeffs[j as usize];
                (*r).coeffs[j as usize] =
                    barrett_reduce(t.wrapping_add((*r).coeffs[(j + len) as usize]));
                (*r).coeffs[(j + len) as usize] = montgomery_reduce(
                    (zeta as i32)
                        .wrapping_mul((((*r).coeffs[(j + len) as usize]).wrapping_sub(t)) as i32),
                );
                j = j + 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    for j in 0..256usize {
        (*r).coeffs[j] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[j] as i32));
    }
}

unsafe fn poly_basemul(r: *mut poly, a: *const poly, b: *const poly) {
    let mut zeta: i16;

    for i in 0..(256 / 4) {
        zeta = ZETAS[64 + i];

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
            ((*r).coeffs[4 * i + 2] as i32).wrapping_mul((-zeta) as i32),
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
}

unsafe fn poly_tomont(r: *mut poly) {
    let f: i16 = 1353;

    for i in 0..KYBER_N {
        (*r).coeffs[i] = montgomery_reduce((f as i32).wrapping_mul((*r).coeffs[i] as i32));
    }
}

unsafe fn poly_reduce(r: *mut poly) {
    for i in 0..KYBER_N {
        (*r).coeffs[i] = barrett_reduce((*r).coeffs[i]);
    }
}

unsafe fn poly_add(r: *mut poly, a: *const poly, b: *const poly) {
    for i in 0..KYBER_N {
        (*r).coeffs[i] = (*a).coeffs[i].wrapping_add((*b).coeffs[i]);
    }
}

unsafe fn poly_sub(r: *mut poly, a: *const poly, b: *const poly) {
    for i in 0..KYBER_N {
        (*r).coeffs[i] = (*a).coeffs[i].wrapping_sub((*b).coeffs[i]);
    }
}

unsafe fn poly_csubq(r: *mut poly) {
    for i in 0..KYBER_N {
        (*r).coeffs[i] = csubq((*r).coeffs[i]);
    }
}

unsafe fn poly_cbd_eta2(r: *mut poly, buf: *const u8) {
    let mut t: u32;
    let mut d: u32;
    let mut a: i16;
    let mut b: i16;

    for i in 0..(256 / 8) {
        t = (*buf.add(4 * i) as u32)
            | ((*buf.add(4 * i + 1) as u32) << 8)
            | ((*buf.add(4 * i + 2) as u32) << 16)
            | ((*buf.add(4 * i + 3) as u32) << 24);

        d = t & 0x5555_5555;
        d = d.wrapping_add((t >> 1) & 0x5555_5555);

        for j in 0..8usize {
            a = ((d >> (4 * j)) & 0x3) as i16;
            b = ((d >> (4 * j + 2)) & 0x3) as i16;
            (*r).coeffs[8 * i + j] = a.wrapping_sub(b);
        }
    }
}

unsafe fn poly_getnoise_eta2(r: *mut poly, seed: *const u8, nonce: u8) {
    let mut buf: [u8; 2 * 256 / 4] = [0u8; 2 * 256 / 4];
    let mut state: crypto_xof_shake256_state = crypto_xof_shake256_state { opaque: [0u8; 256] };
    let mut extseed: [u8; 33] = [0u8; 33];

    memcpy(
        extseed.as_mut_ptr() as *mut c_void,
        seed as *const c_void,
        32,
    );
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

unsafe fn poly_frombytes(r: *mut poly, a: *const u8) {
    for i in 0..(KYBER_N / 2) {
        let a0 = *a.add(3 * i) as u16;
        let a1 = *a.add(3 * i + 1) as u16;
        let a2 = *a.add(3 * i + 2) as u16;
        (*r).coeffs[2 * i] = (((a0 >> 0) | (a1 << 8)) & 0xFFF) as i16;
        (*r).coeffs[2 * i + 1] = (((a1 >> 4) | (a2 << 4)) & 0xFFF) as i16;
    }
}

unsafe fn poly_tobytes(r: *mut u8, a: *const poly) {
    let mut t0: u16;
    let mut t1: u16;

    for i in 0..(KYBER_N / 2) {
        t0 = (*a).coeffs[2 * i] as u16;
        t1 = (*a).coeffs[2 * i + 1] as u16;
        *r.add(3 * i) = (t0 >> 0) as u8;
        *r.add(3 * i + 1) = (((t0 >> 8) | (t1 << 4)) & 0xFF) as u8;
        *r.add(3 * i + 2) = (t1 >> 4) as u8;
    }
}

unsafe fn poly_frommsg(r: *mut poly, msg: *const u8) {
    let mut mask: i16;

    for i in 0..(KYBER_N / 8) {
        for j in 0..8usize {
            mask = 0i16.wrapping_sub((((*msg.add(i)) >> j) & 1) as i16);
            (*r).coeffs[8 * i + j] = mask & ((3329 + 1) / 2);
        }
    }
}

unsafe fn poly_tomsg(msg: *mut u8, a: *const poly) {
    let mut t: u32;

    for i in 0..(KYBER_N / 8) {
        *msg.add(i) = 0;
        for j in 0..8usize {
            t = (*a).coeffs[8 * i + j] as i32 as u32;
            t = t.wrapping_add((((*a).coeffs[8 * i + j] as i32 >> 15) & 3329) as u32);
            t = (((t << 1).wrapping_add(3329 / 2)).wrapping_mul(80635)) >> 28;
            t &= 1;
            *msg.add(i) |= (t << j) as u8;
        }
    }
}

unsafe fn poly_compress_du(r: *mut u8, a: *const poly) {
    let mut t: [u32; 4] = [0u32; 4];

    for i in 0..(KYBER_N / 4) {
        for j in 0..4usize {
            t[j] = (*a).coeffs[4 * i + j] as i32 as u32;
            t[j] = t[j].wrapping_add((((*a).coeffs[4 * i + j] as i32 >> 15) & 3329) as u32);
            t[j] = ((((t[j] as u64) << 10).wrapping_add(3329 / 2)).wrapping_mul(161271u64) >> 29)
                as u32;
            t[j] &= 0x3ff;
        }

        *r.add(5 * i) = (t[0] >> 0) as u8;
        *r.add(5 * i + 1) = ((t[0] >> 8) | (t[1] << 2)) as u8;
        *r.add(5 * i + 2) = ((t[1] >> 6) | (t[2] << 4)) as u8;
        *r.add(5 * i + 3) = ((t[2] >> 4) | (t[3] << 6)) as u8;
        *r.add(5 * i + 4) = (t[3] >> 2) as u8;
    }
}

unsafe fn poly_decompress_du(r: *mut poly, a: *const u8) {
    let mut t: [u16; 4] = [0u16; 4];

    for i in 0..(KYBER_N / 4) {
        let a0 = *a.add(5 * i) as u16;
        let a1 = *a.add(5 * i + 1) as u16;
        let a2 = *a.add(5 * i + 2) as u16;
        let a3 = *a.add(5 * i + 3) as u16;
        let a4 = *a.add(5 * i + 4) as u16;

        t[0] = (a0 >> 0) | (a1 << 8);
        t[1] = (a1 >> 2) | (a2 << 6);
        t[2] = (a2 >> 4) | (a3 << 4);
        t[3] = (a3 >> 6) | (a4 << 2);

        (*r).coeffs[4 * i] = ((((t[0] & 0x3FF) as u32).wrapping_mul(3329) + 512) >> 10) as i16;
        (*r).coeffs[4 * i + 1] =
            ((((t[1] & 0x3FF) as u32).wrapping_mul(3329) + 512) >> 10) as i16;
        (*r).coeffs[4 * i + 2] =
            ((((t[2] & 0x3FF) as u32).wrapping_mul(3329) + 512) >> 10) as i16;
        (*r).coeffs[4 * i + 3] =
            ((((t[3] & 0x3FF) as u32).wrapping_mul(3329) + 512) >> 10) as i16;
    }
}

unsafe fn poly_compress_dv(r: *mut u8, a: *const poly) {
    let mut t: [u32; 8] = [0u32; 8];

    for i in 0..(KYBER_N / 8) {
        for j in 0..8usize {
            t[j] = (*a).coeffs[8 * i + j] as i32 as u32;
            t[j] = t[j].wrapping_add((((*a).coeffs[8 * i + j] as i32 >> 15) & 3329) as u32);
            t[j] = ((((t[j] as u64) << 4).wrapping_add(3329 / 2)).wrapping_mul(161271u64) >> 29)
                as u32;
            t[j] &= 0xf;
        }

        *r.add(4 * i) = (t[0] | (t[1] << 4)) as u8;
        *r.add(4 * i + 1) = (t[2] | (t[3] << 4)) as u8;
        *r.add(4 * i + 2) = (t[4] | (t[5] << 4)) as u8;
        *r.add(4 * i + 3) = (t[6] | (t[7] << 4)) as u8;
    }
}

unsafe fn poly_decompress_dv(r: *mut poly, a: *const u8) {
    for i in 0..(KYBER_N / 2) {
        let ai = *a.add(i) as u16;
        (*r).coeffs[2 * i] = ((((ai & 15) as u32).wrapping_mul(3329) + 8) >> 4) as i16;
        (*r).coeffs[2 * i + 1] = ((((ai >> 4) as u32).wrapping_mul(3329) + 8) >> 4) as i16;
    }
}

unsafe fn polyvec_ntt(r: *mut polyvec) {
    for i in 0..KYBER_K {
        poly_ntt(&mut (*r).vec[i]);
    }
}

unsafe fn polyvec_invntt(r: *mut polyvec) {
    for i in 0..KYBER_K {
        poly_invntt(&mut (*r).vec[i]);
    }
}

unsafe fn polyvec_basemul_acc(r: *mut poly, a: *const polyvec, b: *const polyvec) {
    let mut t: poly = poly::zeroed();

    poly_basemul(r, &(*a).vec[0], &(*b).vec[0]);
    for i in 1..KYBER_K {
        poly_basemul(&mut t, &(*a).vec[i], &(*b).vec[i]);
        poly_add(r, r, &t);
    }

    poly_reduce(r);
}

unsafe fn polyvec_reduce(r: *mut polyvec) {
    for i in 0..KYBER_K {
        poly_reduce(&mut (*r).vec[i]);
    }
}

unsafe fn polyvec_csubq(r: *mut polyvec) {
    for i in 0..KYBER_K {
        poly_csubq(&mut (*r).vec[i]);
    }
}

unsafe fn polyvec_add(r: *mut polyvec, a: *const polyvec, b: *const polyvec) {
    for i in 0..KYBER_K {
        poly_add(&mut (*r).vec[i], &(*a).vec[i], &(*b).vec[i]);
    }
}

unsafe fn polyvec_tobytes(r: *mut u8, a: *const polyvec) {
    for i in 0..KYBER_K {
        poly_tobytes(r.add(i * KYBER_POLYBYTES), &(*a).vec[i]);
    }
}

unsafe fn polyvec_frombytes(r: *mut polyvec, a: *const u8) {
    for i in 0..KYBER_K {
        poly_frombytes(&mut (*r).vec[i], a.add(i * KYBER_POLYBYTES));
    }
}

unsafe fn polyvec_is_canonical(a: *const polyvec) -> c_int {
    for i in 0..KYBER_K {
        for j in 0..KYBER_N {
            if ((*a).vec[i].coeffs[j] as u16) >= 3329 {
                return 0;
            }
        }
    }
    1
}

unsafe fn polyvec_compress(r: *mut u8, a: *const polyvec) {
    for i in 0..KYBER_K {
        poly_compress_du(r.add(i * 320), &(*a).vec[i]);
    }
}

unsafe fn polyvec_decompress(r: *mut polyvec, a: *const u8) {
    for i in 0..KYBER_K {
        poly_decompress_du(&mut (*r).vec[i], a.add(i * 320));
    }
}

unsafe fn rej_uniform(r: *mut i16, len: u32, buf: *const u8, buflen: u32) -> u32 {
    let mut ctr: u32;
    let mut pos: u32;
    let mut val0: u16;
    let mut val1: u16;

    ctr = 0;
    pos = 0;

    while ctr < len && pos + 3 <= buflen {
        let b0 = *buf.add(pos as usize) as u16;
        let b1 = *buf.add(pos as usize + 1) as u16;
        let b2 = *buf.add(pos as usize + 2) as u16;

        val0 = ((b0 >> 0) | (b1 << 8)) & 0xFFF;
        val1 = ((b1 >> 4) | (b2 << 4)) & 0xFFF;
        pos += 3;

        if val0 < 3329 {
            *r.add(ctr as usize) = val0 as i16;
            ctr += 1;
        }
        if ctr < len && val1 < 3329 {
            *r.add(ctr as usize) = val1 as i16;
            ctr += 1;
        }
    }

    ctr
}

unsafe fn gen_matrix(a: *mut polyvec, seed: *const u8, transposed: c_int) {
    let mut state: crypto_xof_shake128_state = crypto_xof_shake128_state { opaque: [0u8; 256] };
    let mut buf: [u8; GEN_MATRIX_BUF_SIZE] = [0u8; GEN_MATRIX_BUF_SIZE];
    let mut extseed: [u8; 34] = [0u8; 34];
    let mut ctr: u32;
    let mut buflen: u32;

    memcpy(
        extseed.as_mut_ptr() as *mut c_void,
        seed as *const c_void,
        32,
    );

    for i in 0..KYBER_K {
        for j in 0..KYBER_K {
            if transposed != 0 {
                extseed[32] = i as u8;
                extseed[33] = j as u8;
            } else {
                extseed[32] = j as u8;
                extseed[33] = i as u8;
            }

            crypto_xof_shake128_init(&mut state);
            crypto_xof_shake128_update(&mut state, extseed.as_ptr(), 34);

            buflen = GEN_MATRIX_BUFLEN as u32;
            crypto_xof_shake128_squeeze(&mut state, buf.as_mut_ptr(), buflen as usize);

            ctr = rej_uniform(
                (*a.add(i)).vec[j].coeffs.as_mut_ptr(),
                256,
                buf.as_ptr(),
                buflen,
            );

            while ctr < 256 {
                crypto_xof_shake128_squeeze(&mut state, buf.as_mut_ptr(), 168);
                ctr += rej_uniform(
                    (*a.add(i)).vec[j].coeffs.as_mut_ptr().add(ctr as usize),
                    256 - ctr,
                    buf.as_ptr(),
                    168,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// IND-CPA public-key encryption scheme
// ---------------------------------------------------------------------

unsafe fn indcpa_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) {
    let mut a: [polyvec; KYBER_K] = [polyvec::zeroed(); KYBER_K];
    let mut e: polyvec = polyvec::zeroed();
    let mut pkpv: polyvec = polyvec::zeroed();
    let mut skpv: polyvec = polyvec::zeroed();
    let mut buf: [u8; 64] = [0u8; 64];
    let mut nonce: u8 = 0;

    let publicseed = buf.as_mut_ptr();
    let noiseseed = buf.as_mut_ptr().add(32);

    // NB: `seed` is declared `const unsigned char seed[32]` in the C
    // source, but the sole caller (`_sodium_mlkem768_ref_seed_keypair`)
    // always passes a 33-byte buffer (32-byte seed + 1-byte domain
    // separator), and 33 bytes are hashed here -- reproduced verbatim.
    crypto_hash_sha3512(buf.as_mut_ptr(), seed, 33);

    gen_matrix(a.as_mut_ptr(), publicseed, 0);

    for i in 0..KYBER_K {
        poly_getnoise_eta2(&mut skpv.vec[i], noiseseed, nonce);
        nonce = nonce.wrapping_add(1);
    }
    for i in 0..KYBER_K {
        poly_getnoise_eta2(&mut e.vec[i], noiseseed, nonce);
        nonce = nonce.wrapping_add(1);
    }

    polyvec_ntt(&mut skpv);
    polyvec_ntt(&mut e);

    for i in 0..KYBER_K {
        polyvec_basemul_acc(&mut pkpv.vec[i], &a[i], &skpv);
        poly_tomont(&mut pkpv.vec[i]);
    }

    polyvec_add(&mut pkpv, &pkpv, &e);
    polyvec_reduce(&mut pkpv);
    polyvec_csubq(&mut pkpv);
    polyvec_reduce(&mut skpv);
    polyvec_csubq(&mut skpv);

    polyvec_tobytes(sk, &skpv);
    polyvec_tobytes(pk, &pkpv);
    memcpy(
        pk.add(KYBER_POLYVECBYTES) as *mut c_void,
        publicseed as *const c_void,
        32,
    );
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(
        &mut skpv as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
    sodium_memzero(
        &mut e as *mut polyvec as *mut c_void,
        core::mem::size_of::<polyvec>(),
    );
}

unsafe fn indcpa_enc(ct: *mut u8, m: *const u8, pk: *const u8, coins: *const u8) {
    let mut sp: polyvec = polyvec::zeroed();
    let mut pkpv: polyvec = polyvec::zeroed();
    let mut ep: polyvec = polyvec::zeroed();
    let mut at: [polyvec; KYBER_K] = [polyvec::zeroed(); KYBER_K];
    let mut b: polyvec = polyvec::zeroed();
    let mut v: poly = poly::zeroed();
    let mut k: poly = poly::zeroed();
    let mut epp: poly = poly::zeroed();
    let mut seed: [u8; 32] = [0u8; 32];
    let mut nonce: u8 = 0;

    memcpy(
        seed.as_mut_ptr() as *mut c_void,
        pk.add(KYBER_POLYVECBYTES) as *const c_void,
        32,
    );

    polyvec_frombytes(&mut pkpv, pk);

    poly_frommsg(&mut k, m);

    gen_matrix(at.as_mut_ptr(), seed.as_ptr(), 1);

    for i in 0..KYBER_K {
        poly_getnoise_eta2(&mut sp.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
    }
    for i in 0..KYBER_K {
        poly_getnoise_eta2(&mut ep.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
    }
    poly_getnoise_eta2(&mut epp, coins, nonce);
    nonce = nonce.wrapping_add(1);

    polyvec_ntt(&mut sp);
    polyvec_reduce(&mut sp);

    for i in 0..KYBER_K {
        polyvec_basemul_acc(&mut b.vec[i], &at[i], &sp);
    }

    polyvec_basemul_acc(&mut v, &pkpv, &sp);

    polyvec_invntt(&mut b);
    poly_invntt(&mut v);

    polyvec_add(&mut b, &b, &ep);
    poly_add(&mut v, &v, &epp);
    poly_add(&mut v, &v, &k);

    polyvec_reduce(&mut b);
    poly_reduce(&mut v);
    polyvec_csubq(&mut b);
    poly_csubq(&mut v);

    polyvec_compress(ct, &b);
    poly_compress_dv(ct.add(KYBER_POLYVECCOMPRESSEDBYTES_DU), &v);
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
    sodium_memzero(&mut k as *mut poly as *mut c_void, core::mem::size_of::<poly>());
}

unsafe fn indcpa_dec(m: *mut u8, ct: *const u8, sk: *const u8) {
    let mut b: polyvec = polyvec::zeroed();
    let mut skpv: polyvec = polyvec::zeroed();
    let mut v: poly = poly::zeroed();
    let mut mp: poly = poly::zeroed();

    polyvec_decompress(&mut b, ct);
    poly_decompress_dv(&mut v, ct.add(KYBER_POLYVECCOMPRESSEDBYTES_DU));

    polyvec_frombytes(&mut skpv, sk);

    polyvec_ntt(&mut b);
    polyvec_reduce(&mut b);
    polyvec_basemul_acc(&mut mp, &skpv, &b);
    poly_invntt(&mut mp);

    poly_sub(&mut mp, &v, &mp);
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

unsafe fn cmov(r: *mut u8, x: *const u8, len: usize, b: u8) {
    let mask: u8 = (-(b as i32)) as u8;

    for i in 0..len {
        let ri = *r.add(i);
        let xi = *x.add(i);
        *r.add(i) = ri ^ (mask & (ri ^ xi));
    }
}

// ---------------------------------------------------------------------
// Public KEM entry points (renamed by `private/quirks.h` to `_sodium_*`)
// ---------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn _sodium_mlkem768_ref_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut indseed: [u8; 33] = [0u8; 33];

    memcpy(
        indseed.as_mut_ptr() as *mut c_void,
        seed as *const c_void,
        32,
    );
    indseed[32] = 3;

    indcpa_keypair(pk, sk, indseed.as_ptr());
    sodium_memzero(indseed.as_mut_ptr() as *mut c_void, indseed.len());
    memcpy(
        sk.add(KYBER_INDCPA_SECRETKEYBYTES) as *mut c_void,
        pk as *const c_void,
        KYBER_INDCPA_PUBLICKEYBYTES,
    );
    crypto_hash_sha3256(
        sk.add(KYBER_INDCPA_SECRETKEYBYTES + KYBER_INDCPA_PUBLICKEYBYTES),
        pk,
        KYBER_INDCPA_PUBLICKEYBYTES as u64,
    );
    memcpy(
        sk.add(KYBER_INDCPA_SECRETKEYBYTES + KYBER_INDCPA_PUBLICKEYBYTES + 32) as *mut c_void,
        seed.add(32) as *const c_void,
        32,
    );

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed: [u8; 64] = [0u8; 64];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr(), 64);
    ret = _sodium_mlkem768_ref_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let mut pkpv: polyvec = polyvec::zeroed();
    let mut buf: [u8; 64] = [0u8; 64];
    let mut kr: [u8; 64] = [0u8; 64];

    polyvec_frombytes(&mut pkpv, pk);
    if polyvec_is_canonical(&pkpv) == 0 {
        return -1;
    }

    memcpy(
        buf.as_mut_ptr() as *mut c_void,
        seed as *const c_void,
        32,
    );
    crypto_hash_sha3256(
        buf.as_mut_ptr().add(32),
        pk,
        KYBER_INDCPA_PUBLICKEYBYTES as u64,
    );

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(ct, buf.as_ptr(), pk, kr.as_ptr().add(32));

    memcpy(
        ss as *mut c_void,
        kr.as_ptr() as *const c_void,
        32,
    );
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, kr.len());

    0
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed: [u8; 32] = [0u8; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr(), 32);
    ret = _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());

    ret
}

#[no_mangle]
pub unsafe extern "C" fn _sodium_mlkem768_ref_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    let mut buf: [u8; 64] = [0u8; 64];
    let mut kr: [u8; 64] = [0u8; 64];
    let mut k_bar: [u8; 32] = [0u8; 32];
    let mut cmp: [u8; KYBER_CIPHERTEXTBYTES] = [0u8; KYBER_CIPHERTEXTBYTES];
    let pk = sk.add(KYBER_INDCPA_SECRETKEYBYTES);
    let hpk = sk.add(KYBER_INDCPA_SECRETKEYBYTES + KYBER_INDCPA_PUBLICKEYBYTES);
    let z = sk.add(KYBER_INDCPA_SECRETKEYBYTES + KYBER_INDCPA_PUBLICKEYBYTES + 32);
    let fail: c_int;
    let mut fail_mask: u32;
    let mut state: crypto_xof_shake256_state = crypto_xof_shake256_state { opaque: [0u8; 256] };

    indcpa_dec(buf.as_mut_ptr(), ct, sk);

    memcpy(
        buf.as_mut_ptr().add(32) as *mut c_void,
        hpk as *const c_void,
        32,
    );

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(cmp.as_mut_ptr(), buf.as_ptr(), pk, kr.as_ptr().add(32));

    fail = sodium_memcmp(
        ct as *const c_void,
        cmp.as_ptr() as *const c_void,
        KYBER_CIPHERTEXTBYTES,
    );
    fail_mask = fail as u32;
    fail_mask >>= (core::mem::size_of::<u32>() * 8 - 1) as u32;

    crypto_xof_shake256_init(&mut state);
    crypto_xof_shake256_update(&mut state, z, 32);
    crypto_xof_shake256_update(&mut state, ct, KYBER_CIPHERTEXTBYTES as u64);
    crypto_xof_shake256_squeeze(&mut state, k_bar.as_mut_ptr(), 32);

    cmov(kr.as_mut_ptr(), k_bar.as_ptr(), 32, fail_mask as u8);

    memcpy(
        ss as *mut c_void,
        kr.as_ptr() as *const c_void,
        32,
    );
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, kr.len());
    sodium_memzero(k_bar.as_mut_ptr() as *mut c_void, k_bar.len());
    sodium_memzero(cmp.as_mut_ptr() as *mut c_void, cmp.len());
    sodium_memzero(
        &mut state as *mut crypto_xof_shake256_state as *mut c_void,
        core::mem::size_of::<crypto_xof_shake256_state>(),
    );

    0
}
