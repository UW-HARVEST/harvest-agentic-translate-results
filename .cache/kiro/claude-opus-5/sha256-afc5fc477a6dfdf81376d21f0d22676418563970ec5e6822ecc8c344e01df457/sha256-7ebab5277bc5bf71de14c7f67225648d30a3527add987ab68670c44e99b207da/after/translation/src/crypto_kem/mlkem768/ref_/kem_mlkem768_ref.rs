//! Translation of c_src/libsodium/crypto_kem/mlkem768/ref/kem_mlkem768_ref.c

use core::ffi::{c_int, c_uint, c_void};

const MLKEM768_Q: i32 = 3329;
const MLKEM768_N: usize = 256;
const MLKEM768_K: usize = 3;
// MLKEM768_ETA1 == 2 (unused directly here)
const MLKEM768_ETA2: usize = 2;
// MLKEM768_DU == 10, MLKEM768_DV == 4 (encoded via table sizes below)

const MLKEM768_POLYBYTES: usize = 384;
const MLKEM768_POLYVECBYTES: usize = MLKEM768_K * MLKEM768_POLYBYTES;
const MLKEM768_POLYCOMPRESSEDBYTES_DU: usize = 320;
const MLKEM768_POLYCOMPRESSEDBYTES_DV: usize = 128;
const MLKEM768_POLYVECCOMPRESSEDBYTES_DU: usize = MLKEM768_K * MLKEM768_POLYCOMPRESSEDBYTES_DU;

const CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES: usize = 1184;
const CRYPTO_KEM_MLKEM768_SECRETKEYBYTES: usize = 2400;
const CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES: usize = 1088;
const CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES: usize = 32;
const CRYPTO_KEM_MLKEM768_SEEDBYTES: usize = 64;

const CRYPTO_XOF_SHAKE128_BLOCKBYTES: usize = 168;

// crypto_hash_sha3.h / crypto_xof_shake*.h: CRYPTO_ALIGN(16) { unsigned char opaque[256]; }
#[repr(C, align(16))]
struct CryptoXofShake128State {
    opaque: [u8; 256],
}

#[repr(C, align(16))]
struct CryptoXofShake256State {
    opaque: [u8; 256],
}

extern "C" {
    fn crypto_hash_sha3256(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha3512(out: *mut u8, in_: *const u8, inlen: u64) -> c_int;

    fn crypto_xof_shake128_init(state: *mut CryptoXofShake128State) -> c_int;
    fn crypto_xof_shake128_update(
        state: *mut CryptoXofShake128State,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_xof_shake128_squeeze(
        state: *mut CryptoXofShake128State,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    fn crypto_xof_shake256_init(state: *mut CryptoXofShake256State) -> c_int;
    fn crypto_xof_shake256_update(
        state: *mut CryptoXofShake256State,
        in_: *const u8,
        inlen: u64,
    ) -> c_int;
    fn crypto_xof_shake256_squeeze(
        state: *mut CryptoXofShake256State,
        out: *mut u8,
        outlen: usize,
    ) -> c_int;

    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1_: *const c_void, b2_: *const c_void, len: usize) -> c_int;
    fn randombytes_buf(buf: *mut c_void, size: usize);
}

#[repr(C)]
struct poly {
    coeffs: [i16; MLKEM768_N],
}

#[repr(C)]
struct polyvec {
    vec: [poly; MLKEM768_K],
}

static zetas: [i16; 128] = [
    2285, 2571, 2970, 1812, 1493, 1422, 287, 202, 3158, 622, 1577, 182, 962, 2127, 1855, 1468,
    573, 2004, 264, 383, 2500, 1458, 1727, 3199, 2648, 1017, 732, 608, 1787, 411, 3124, 1758,
    1223, 652, 2777, 1015, 2036, 1491, 3047, 1785, 516, 3321, 3009, 2663, 1711, 2167, 126, 1469,
    2476, 3239, 3058, 830, 107, 1908, 3082, 2378, 2931, 961, 1821, 2604, 448, 2264, 677, 2054,
    2226, 430, 555, 843, 2078, 871, 1550, 105, 422, 587, 177, 3094, 3038, 2869, 1574, 1653,
    3083, 778, 1159, 3182, 2552, 1483, 2727, 1119, 1739, 644, 2457, 349, 418, 329, 3173, 3254,
    817, 1097, 603, 610, 1322, 2044, 1864, 384, 2114, 3193, 1218, 1994, 2455, 220, 2142, 1670,
    2144, 1799, 2051, 794, 1819, 2475, 2459, 478, 3221, 3021, 996, 991, 958, 1869, 1522, 1628,
];

unsafe fn montgomery_reduce(a: i32) -> i16 {
    let mut t: i16;

    // t = (int16_t)((uint16_t)a * 62209U);
    // (uint16_t)a truncates; promoted to unsigned int; * 62209U in u32; cast to i16.
    t = ((a as u16 as u32).wrapping_mul(62209u32)) as u16 as i16;
    // t = (int16_t)((a - (int32_t)t * MLKEM768_Q) >> 16);
    t = ((a.wrapping_sub((t as i32).wrapping_mul(MLKEM768_Q))) >> 16) as i16;

    t
}

unsafe fn barrett_reduce(a: i16) -> i16 {
    let mut t: i16;

    // t = (int16_t)(((int32_t)a * 20159) >> 26);
    t = (((a as i32).wrapping_mul(20159)) >> 26) as i16;
    // t = a - t * MLKEM768_Q;  (int16_t arithmetic via int promotion, truncated)
    t = (a as i32).wrapping_sub((t as i32).wrapping_mul(MLKEM768_Q)) as i16;

    t
}

unsafe fn csubq(mut a: i16) -> i16 {
    // a -= MLKEM768_Q;
    a = (a as i32).wrapping_sub(MLKEM768_Q) as i16;
    // a += (a >> 15) & MLKEM768_Q;  (a >> 15 is arithmetic on int16_t promoted to int)
    a = (a as i32).wrapping_add(((a as i32) >> 15) & MLKEM768_Q) as i16;

    a
}

unsafe fn poly_ntt(r: *mut poly) {
    let mut len: c_uint;
    let mut start: c_uint;
    let mut j: c_uint;
    let mut k: c_uint;
    let mut t: i16;
    let mut zeta: i16;

    let coeffs = (*r).coeffs.as_mut_ptr();

    k = 1;
    len = 128;
    while len >= 2 {
        start = 0;
        while (start as usize) < MLKEM768_N {
            zeta = zetas[k as usize];
            k += 1;
            j = start;
            while j < start + len {
                t = montgomery_reduce((zeta as i32).wrapping_mul(*coeffs.add((j + len) as usize) as i32));
                *coeffs.add((j + len) as usize) =
                    (*coeffs.add(j as usize) as i32).wrapping_sub(t as i32) as i16;
                *coeffs.add(j as usize) =
                    (*coeffs.add(j as usize) as i32).wrapping_add(t as i32) as i16;
                j += 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

unsafe fn poly_invntt(r: *mut poly) {
    let mut start: c_uint;
    let mut len: c_uint;
    let mut j: c_uint;
    let mut k: c_uint;
    let mut t: i16;
    let mut zeta: i16;
    let f: i16 = 1441;

    let coeffs = (*r).coeffs.as_mut_ptr();

    k = 127;
    len = 2;
    while len <= 128 {
        start = 0;
        while (start as usize) < MLKEM768_N {
            zeta = zetas[k as usize];
            k = k.wrapping_sub(1);
            j = start;
            while j < start + len {
                t = *coeffs.add(j as usize);
                *coeffs.add(j as usize) =
                    barrett_reduce((t as i32).wrapping_add(*coeffs.add((j + len) as usize) as i32) as i16);
                *coeffs.add((j + len) as usize) = montgomery_reduce(
                    (zeta as i32).wrapping_mul(
                        (*coeffs.add((j + len) as usize) as i32).wrapping_sub(t as i32),
                    ),
                );
                j += 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    let mut j2: usize = 0;
    while j2 < MLKEM768_N {
        *coeffs.add(j2) = montgomery_reduce((f as i32).wrapping_mul(*coeffs.add(j2) as i32));
        j2 += 1;
    }
}

unsafe fn poly_basemul(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;
    let mut zeta: i16;

    let rc = (*r).coeffs.as_mut_ptr();
    let ac = (*a).coeffs.as_ptr();
    let bc = (*b).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N / 4 {
        zeta = zetas[64 + i];

        *rc.add(4 * i) = montgomery_reduce(
            (*ac.add(4 * i + 1) as i32).wrapping_mul(*bc.add(4 * i + 1) as i32),
        );
        *rc.add(4 * i) =
            montgomery_reduce((*rc.add(4 * i) as i32).wrapping_mul(zeta as i32));
        *rc.add(4 * i) = (*rc.add(4 * i) as i32).wrapping_add(montgomery_reduce(
            (*ac.add(4 * i) as i32).wrapping_mul(*bc.add(4 * i) as i32),
        ) as i32) as i16;

        *rc.add(4 * i + 1) = montgomery_reduce(
            (*ac.add(4 * i) as i32).wrapping_mul(*bc.add(4 * i + 1) as i32),
        );
        *rc.add(4 * i + 1) = (*rc.add(4 * i + 1) as i32).wrapping_add(montgomery_reduce(
            (*ac.add(4 * i + 1) as i32).wrapping_mul(*bc.add(4 * i) as i32),
        ) as i32) as i16;

        *rc.add(4 * i + 2) = montgomery_reduce(
            (*ac.add(4 * i + 3) as i32).wrapping_mul(*bc.add(4 * i + 3) as i32),
        );
        *rc.add(4 * i + 2) = montgomery_reduce(
            (*rc.add(4 * i + 2) as i32).wrapping_mul((zeta as i32).wrapping_neg()),
        );
        *rc.add(4 * i + 2) = (*rc.add(4 * i + 2) as i32).wrapping_add(montgomery_reduce(
            (*ac.add(4 * i + 2) as i32).wrapping_mul(*bc.add(4 * i + 2) as i32),
        ) as i32) as i16;

        *rc.add(4 * i + 3) = montgomery_reduce(
            (*ac.add(4 * i + 2) as i32).wrapping_mul(*bc.add(4 * i + 3) as i32),
        );
        *rc.add(4 * i + 3) = (*rc.add(4 * i + 3) as i32).wrapping_add(montgomery_reduce(
            (*ac.add(4 * i + 3) as i32).wrapping_mul(*bc.add(4 * i + 2) as i32),
        ) as i32) as i16;

        i += 1;
    }
}

unsafe fn poly_tomont(r: *mut poly) {
    let mut i: usize;
    let f: i16 = 1353;

    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N {
        *rc.add(i) = montgomery_reduce((f as i32).wrapping_mul(*rc.add(i) as i32));
        i += 1;
    }
}

unsafe fn poly_reduce(r: *mut poly) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N {
        *rc.add(i) = barrett_reduce(*rc.add(i));
        i += 1;
    }
}

unsafe fn poly_add(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();
    let ac = (*a).coeffs.as_ptr();
    let bc = (*b).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N {
        *rc.add(i) = (*ac.add(i) as i32).wrapping_add(*bc.add(i) as i32) as i16;
        i += 1;
    }
}

unsafe fn poly_sub(r: *mut poly, a: *const poly, b: *const poly) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();
    let ac = (*a).coeffs.as_ptr();
    let bc = (*b).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N {
        *rc.add(i) = (*ac.add(i) as i32).wrapping_sub(*bc.add(i) as i32) as i16;
        i += 1;
    }
}

unsafe fn poly_csubq(r: *mut poly) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N {
        *rc.add(i) = csubq(*rc.add(i));
        i += 1;
    }
}

unsafe fn poly_cbd_eta2(r: *mut poly, buf: *const u8) {
    let mut i: usize;
    let mut j: usize;
    let mut t: u32;
    let mut d: u32;
    let mut a: i16;
    let mut b: i16;

    let rc = (*r).coeffs.as_mut_ptr();

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
            *rc.add(8 * i + j) = (a as i32).wrapping_sub(b as i32) as i16;
            j += 1;
        }
        i += 1;
    }
}

unsafe fn poly_getnoise_eta2(r: *mut poly, seed: *const u8, nonce: u8) {
    let mut buf: [u8; MLKEM768_ETA2 * MLKEM768_N / 4] = [0; MLKEM768_ETA2 * MLKEM768_N / 4];
    let mut state = CryptoXofShake256State { opaque: [0; 256] };
    let mut extseed: [u8; 33] = [0; 33];

    core::ptr::copy_nonoverlapping(seed, extseed.as_mut_ptr(), 32);
    extseed[32] = nonce;

    crypto_xof_shake256_init(&mut state);
    crypto_xof_shake256_update(&mut state, extseed.as_ptr(), 33);
    crypto_xof_shake256_squeeze(&mut state, buf.as_mut_ptr(), core::mem::size_of_val(&buf));

    poly_cbd_eta2(r, buf.as_ptr());
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of_val(&state));
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&buf));
}

unsafe fn poly_frombytes(r: *mut poly, a: *const u8) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N / 2 {
        *rc.add(2 * i) =
            (((*a.add(3 * i + 0) as u16) >> 0) | ((*a.add(3 * i + 1) as u16) << 8)) as i16 & 0xFFF;
        *rc.add(2 * i + 1) =
            (((*a.add(3 * i + 1) as u16) >> 4) | ((*a.add(3 * i + 2) as u16) << 4)) as i16 & 0xFFF;
        i += 1;
    }
}

unsafe fn poly_tobytes(r: *mut u8, a: *const poly) {
    let mut i: usize;
    let mut t0: u16;
    let mut t1: u16;
    let ac = (*a).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N / 2 {
        t0 = *ac.add(2 * i) as u16;
        t1 = *ac.add(2 * i + 1) as u16;
        *r.add(3 * i + 0) = (t0 >> 0) as u8;
        *r.add(3 * i + 1) = ((t0 >> 8) | (t1 << 4)) as u8;
        *r.add(3 * i + 2) = (t1 >> 4) as u8;
        i += 1;
    }
}

unsafe fn poly_frommsg(r: *mut poly, msg: *const u8) {
    let mut i: usize;
    let mut j: usize;
    let mut mask: i16;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N / 8 {
        j = 0;
        while j < 8 {
            // mask = -((msg[i] >> j) & 1);  int arithmetic, stored into int16_t
            mask = (((*msg.add(i) as i32) >> j) & 1).wrapping_neg() as i16;
            *rc.add(8 * i + j) = (mask as i32 & ((MLKEM768_Q + 1) / 2)) as i16;
            j += 1;
        }
        i += 1;
    }
}

unsafe fn poly_tomsg(msg: *mut u8, a: *const poly) {
    let mut i: usize;
    let mut j: usize;
    let mut t: u32;
    let ac = (*a).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N / 8 {
        *msg.add(i) = 0;
        j = 0;
        while j < 8 {
            t = *ac.add(8 * i + j) as u32;
            t = t.wrapping_add(((((*ac.add(8 * i + j) as i32) >> 15) & MLKEM768_Q) as u32));
            t = (((t << 1).wrapping_add((MLKEM768_Q / 2) as u32)).wrapping_mul(80635)) >> 28;
            t &= 1;
            *msg.add(i) |= (t << j) as u8;
            j += 1;
        }
        i += 1;
    }
}

unsafe fn poly_compress_du(r: *mut u8, a: *const poly) {
    let mut t: [u32; 4] = [0; 4];
    let mut i: usize;
    let mut j: usize;
    let ac = (*a).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N / 4 {
        j = 0;
        while j < 4 {
            t[j] = *ac.add(4 * i + j) as u32;
            t[j] = t[j].wrapping_add(((((*ac.add(4 * i + j) as i32) >> 15) & MLKEM768_Q) as u32));
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

unsafe fn poly_decompress_du(r: *mut poly, a: *const u8) {
    let mut t: [u16; 4] = [0; 4];
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N / 4 {
        t[0] = (*a.add(5 * i + 0) as u16 >> 0) | ((*a.add(5 * i + 1) as u16) << 8);
        t[1] = (*a.add(5 * i + 1) as u16 >> 2) | ((*a.add(5 * i + 2) as u16) << 6);
        t[2] = (*a.add(5 * i + 2) as u16 >> 4) | ((*a.add(5 * i + 3) as u16) << 4);
        t[3] = (*a.add(5 * i + 3) as u16 >> 6) | ((*a.add(5 * i + 4) as u16) << 2);

        *rc.add(4 * i + 0) =
            ((((t[0] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10) as i16;
        *rc.add(4 * i + 1) =
            ((((t[1] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10) as i16;
        *rc.add(4 * i + 2) =
            ((((t[2] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10) as i16;
        *rc.add(4 * i + 3) =
            ((((t[3] & 0x3FF) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(512)) >> 10) as i16;
        i += 1;
    }
}

unsafe fn poly_compress_dv(r: *mut u8, a: *const poly) {
    let mut t: [u32; 8] = [0; 8];
    let mut i: usize;
    let mut j: usize;
    let ac = (*a).coeffs.as_ptr();

    i = 0;
    while i < MLKEM768_N / 8 {
        j = 0;
        while j < 8 {
            t[j] = *ac.add(8 * i + j) as u32;
            t[j] = t[j].wrapping_add(((((*ac.add(8 * i + j) as i32) >> 15) & MLKEM768_Q) as u32));
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

unsafe fn poly_decompress_dv(r: *mut poly, a: *const u8) {
    let mut i: usize;
    let rc = (*r).coeffs.as_mut_ptr();

    i = 0;
    while i < MLKEM768_N / 2 {
        *rc.add(2 * i + 0) =
            (((((*a.add(i) & 15) as u16) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(8)) >> 4)
                as i16;
        *rc.add(2 * i + 1) =
            (((((*a.add(i) >> 4) as u16) as u32).wrapping_mul(MLKEM768_Q as u32).wrapping_add(8)) >> 4)
                as i16;
        i += 1;
    }
}

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
    let mut t = poly { coeffs: [0; MLKEM768_N] };
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
            if ((*a).vec[i].coeffs[j] as u16) >= MLKEM768_Q as u16 {
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

unsafe fn rej_uniform(r: *mut i16, len: c_uint, buf: *const u8, buflen: c_uint) -> c_uint {
    let mut ctr: c_uint;
    let mut pos: c_uint;
    let mut val0: u16;
    let mut val1: u16;

    ctr = 0;
    pos = 0;
    /* Variable-time rejection is fine here: callers only use public matrix seeds. */
    while ctr < len && pos + 3 <= buflen {
        val0 = (((*buf.add(pos as usize + 0) as u16) >> 0)
            | ((*buf.add(pos as usize + 1) as u16) << 8))
            & 0xFFF;
        val1 = (((*buf.add(pos as usize + 1) as u16) >> 4)
            | ((*buf.add(pos as usize + 2) as u16) << 4))
            & 0xFFF;
        pos += 3;

        if (val0 as i32) < MLKEM768_Q {
            *r.add(ctr as usize) = val0 as i16;
            ctr += 1;
        }
        if ctr < len && (val1 as i32) < MLKEM768_Q {
            *r.add(ctr as usize) = val1 as i16;
            ctr += 1;
        }
    }

    ctr
}

// GEN_MATRIX_NBLOCKS = ((12*256/8 * (1<<12)/3329 + 168) / 168)
const GEN_MATRIX_NBLOCKS: usize = (12 * MLKEM768_N / 8 * (1 << 12) / (MLKEM768_Q as usize)
    + CRYPTO_XOF_SHAKE128_BLOCKBYTES)
    / CRYPTO_XOF_SHAKE128_BLOCKBYTES;

unsafe fn gen_matrix(a: *mut polyvec, seed: *const u8, transposed: c_int) {
    let mut state = CryptoXofShake128State { opaque: [0; 256] };
    let mut buf: [u8; GEN_MATRIX_NBLOCKS * CRYPTO_XOF_SHAKE128_BLOCKBYTES + 2] =
        [0; GEN_MATRIX_NBLOCKS * CRYPTO_XOF_SHAKE128_BLOCKBYTES + 2];
    let mut extseed: [u8; 34] = [0; 34];
    let mut ctr: c_uint;
    let mut i: usize;
    let mut j: usize;
    let mut buflen: c_uint;

    core::ptr::copy_nonoverlapping(seed, extseed.as_mut_ptr(), 32);

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

            buflen = (GEN_MATRIX_NBLOCKS * CRYPTO_XOF_SHAKE128_BLOCKBYTES) as c_uint;
            crypto_xof_shake128_squeeze(&mut state, buf.as_mut_ptr(), buflen as usize);

            ctr = rej_uniform(
                (*a.add(i)).vec[j].coeffs.as_mut_ptr(),
                MLKEM768_N as c_uint,
                buf.as_ptr(),
                buflen,
            );

            /* Refill count depends on public XOF output, not on secret key material. */
            while (ctr as usize) < MLKEM768_N {
                crypto_xof_shake128_squeeze(
                    &mut state,
                    buf.as_mut_ptr(),
                    CRYPTO_XOF_SHAKE128_BLOCKBYTES,
                );
                ctr += rej_uniform(
                    (*a.add(i)).vec[j].coeffs.as_mut_ptr().add(ctr as usize),
                    MLKEM768_N as c_uint - ctr,
                    buf.as_ptr(),
                    CRYPTO_XOF_SHAKE128_BLOCKBYTES as c_uint,
                );
            }
            j += 1;
        }
        i += 1;
    }
}

unsafe fn indcpa_keypair(pk: *mut u8, sk: *mut u8, seed: *const u8) {
    let mut a: [polyvec; MLKEM768_K] = core::mem::zeroed();
    let mut e: polyvec = core::mem::zeroed();
    let mut pkpv: polyvec = core::mem::zeroed();
    let mut skpv: polyvec = core::mem::zeroed();
    let mut buf: [u8; 64] = [0; 64];
    let mut i: usize;
    let mut nonce: u8 = 0;

    crypto_hash_sha3512(buf.as_mut_ptr(), seed, 33);

    let publicseed = buf.as_mut_ptr();
    let noiseseed = buf.as_mut_ptr().add(32);

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

    polyvec_add(&mut pkpv, &pkpv, &e);
    polyvec_reduce(&mut pkpv);
    polyvec_csubq(&mut pkpv);
    polyvec_reduce(&mut skpv);
    polyvec_csubq(&mut skpv);

    polyvec_tobytes(sk, &skpv);
    polyvec_tobytes(pk, &pkpv);
    core::ptr::copy_nonoverlapping(publicseed, pk.add(MLKEM768_POLYVECBYTES), 32);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&buf));
    sodium_memzero(&mut skpv as *mut _ as *mut c_void, core::mem::size_of::<polyvec>());
    sodium_memzero(&mut e as *mut _ as *mut c_void, core::mem::size_of::<polyvec>());
}

unsafe fn indcpa_enc(ct: *mut u8, m: *const u8, pk: *const u8, coins: *const u8) {
    let mut sp: polyvec = core::mem::zeroed();
    let mut pkpv: polyvec = core::mem::zeroed();
    let mut ep: polyvec = core::mem::zeroed();
    let mut at: [polyvec; MLKEM768_K] = core::mem::zeroed();
    let mut b: polyvec = core::mem::zeroed();
    let mut v: poly = core::mem::zeroed();
    let mut k: poly = core::mem::zeroed();
    let mut epp: poly = core::mem::zeroed();
    let mut seed: [u8; 32] = [0; 32];
    let mut i: usize;
    let mut nonce: u8 = 0;

    core::ptr::copy_nonoverlapping(pk.add(MLKEM768_POLYVECBYTES), seed.as_mut_ptr(), 32);

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
    let _ = nonce;

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

    polyvec_add(&mut b, &b, &ep);
    poly_add(&mut v, &v, &epp);
    poly_add(&mut v, &v, &k);

    polyvec_reduce(&mut b);
    poly_reduce(&mut v);
    polyvec_csubq(&mut b);
    poly_csubq(&mut v);

    polyvec_compress(ct, &b);
    poly_compress_dv(ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU), &v);
    sodium_memzero(&mut sp as *mut _ as *mut c_void, core::mem::size_of::<polyvec>());
    sodium_memzero(&mut ep as *mut _ as *mut c_void, core::mem::size_of::<polyvec>());
    sodium_memzero(&mut epp as *mut _ as *mut c_void, core::mem::size_of::<poly>());
    sodium_memzero(&mut k as *mut _ as *mut c_void, core::mem::size_of::<poly>());
}

unsafe fn indcpa_dec(m: *mut u8, ct: *const u8, sk: *const u8) {
    let mut b: polyvec = core::mem::zeroed();
    let mut skpv: polyvec = core::mem::zeroed();
    let mut v: poly = core::mem::zeroed();
    let mut mp: poly = core::mem::zeroed();

    polyvec_decompress(&mut b, ct);
    poly_decompress_dv(&mut v, ct.add(MLKEM768_POLYVECCOMPRESSEDBYTES_DU));

    polyvec_frombytes(&mut skpv, sk);

    polyvec_ntt(&mut b);
    polyvec_reduce(&mut b);
    polyvec_basemul_acc(&mut mp, &skpv, &b);
    poly_invntt(&mut mp);

    poly_sub(&mut mp, &v, &mp);
    poly_reduce(&mut mp);
    poly_csubq(&mut mp);

    poly_tomsg(m, &mp);
    sodium_memzero(&mut skpv as *mut _ as *mut c_void, core::mem::size_of::<polyvec>());
    sodium_memzero(&mut mp as *mut _ as *mut c_void, core::mem::size_of::<poly>());
}

unsafe fn cmov(r: *mut u8, x: *const u8, len: usize, b: u8) {
    let mut i: usize;
    let mask: u8;

    // mask = (unsigned char)(-(int)b);
    mask = (b as i32).wrapping_neg() as u8;

    // HAVE_INLINE_ASM undefined: no volatile barrier

    i = 0;
    while i < len {
        *r.add(i) ^= mask & (*r.add(i) ^ *x.add(i));
        i += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut indseed: [u8; 33] = [0; 33];

    core::ptr::copy_nonoverlapping(seed, indseed.as_mut_ptr(), 32);
    indseed[32] = MLKEM768_K as u8;

    indcpa_keypair(pk, sk, indseed.as_ptr());
    sodium_memzero(indseed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&indseed));
    core::ptr::copy_nonoverlapping(pk, sk.add(MLKEM768_POLYVECBYTES), CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES);
    crypto_hash_sha3256(
        sk.add(MLKEM768_POLYVECBYTES + CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES),
        pk,
        CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES as u64,
    );
    core::ptr::copy_nonoverlapping(
        seed.add(32),
        sk.add(MLKEM768_POLYVECBYTES + CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES + 32),
        32,
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed: [u8; CRYPTO_KEM_MLKEM768_SEEDBYTES] = [0; CRYPTO_KEM_MLKEM768_SEEDBYTES];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, CRYPTO_KEM_MLKEM768_SEEDBYTES);
    ret = _sodium_mlkem768_ref_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let mut pkpv: polyvec = core::mem::zeroed();
    let mut buf: [u8; 64] = [0; 64];
    let mut kr: [u8; 64] = [0; 64];

    polyvec_frombytes(&mut pkpv, pk);
    if polyvec_is_canonical(&pkpv) == 0 {
        return -1;
    }

    core::ptr::copy_nonoverlapping(seed, buf.as_mut_ptr(), 32);
    crypto_hash_sha3256(buf.as_mut_ptr().add(32), pk, CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES as u64);

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(ct, buf.as_ptr(), pk, kr.as_ptr().add(32));

    core::ptr::copy_nonoverlapping(kr.as_ptr(), ss, CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&buf));
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kr));

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed: [u8; 32] = [0; 32];
    let ret: c_int;

    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
    ret = _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&seed));

    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_dec(
    ss: *mut u8,
    ct: *const u8,
    sk: *const u8,
) -> c_int {
    let mut buf: [u8; 64] = [0; 64];
    let mut kr: [u8; 64] = [0; 64];
    let mut k_bar: [u8; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES] =
        [0; CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES];
    let mut cmp: [u8; CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES] =
        [0; CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES];
    let pk: *const u8 = sk.add(MLKEM768_POLYVECBYTES);
    let hpk: *const u8 = sk.add(MLKEM768_POLYVECBYTES + CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES);
    let z: *const u8 = sk.add(MLKEM768_POLYVECBYTES + CRYPTO_KEM_MLKEM768_PUBLICKEYBYTES + 32);
    let fail: c_int;
    let mut fail_mask: c_uint;
    let mut state = CryptoXofShake256State { opaque: [0; 256] };

    indcpa_dec(buf.as_mut_ptr(), ct, sk);

    core::ptr::copy_nonoverlapping(hpk, buf.as_mut_ptr().add(32), 32);

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    indcpa_enc(cmp.as_mut_ptr(), buf.as_ptr(), pk, kr.as_ptr().add(32));

    fail = sodium_memcmp(
        ct as *const c_void,
        cmp.as_ptr() as *const c_void,
        CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES,
    );
    fail_mask = fail as c_uint;
    fail_mask >>= core::mem::size_of::<c_uint>() * 8 - 1;

    crypto_xof_shake256_init(&mut state);
    crypto_xof_shake256_update(&mut state, z, 32);
    crypto_xof_shake256_update(&mut state, ct, CRYPTO_KEM_MLKEM768_CIPHERTEXTBYTES as u64);
    crypto_xof_shake256_squeeze(
        &mut state,
        k_bar.as_mut_ptr(),
        CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES,
    );

    cmov(
        kr.as_mut_ptr(),
        k_bar.as_ptr(),
        CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES,
        fail_mask as u8,
    );

    core::ptr::copy_nonoverlapping(kr.as_ptr(), ss, CRYPTO_KEM_MLKEM768_SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&buf));
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&kr));
    sodium_memzero(k_bar.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&k_bar));
    sodium_memzero(cmp.as_mut_ptr() as *mut c_void, core::mem::size_of_val(&cmp));
    sodium_memzero(&mut state as *mut _ as *mut c_void, core::mem::size_of_val(&state));

    0
}
