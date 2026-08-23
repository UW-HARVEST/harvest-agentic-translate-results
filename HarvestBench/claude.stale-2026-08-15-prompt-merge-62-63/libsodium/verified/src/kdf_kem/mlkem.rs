// Translation of crypto_kem/mlkem768/kem_mlkem768.c and
// crypto_kem/mlkem768/ref/kem_mlkem768_ref.c (reference implementation).
//
// Internal arithmetic reproduces the C int16_t/int32_t/uint32_t semantics exactly
// (wrapping where C relies on modular truncation) for byte-identical output.

use core::ffi::c_int;
use core::ffi::c_void;

const Q: i32 = 3329;
const N: usize = 256;
const K: usize = 3;
const ETA2: usize = 2;
const POLYBYTES: usize = 384;
const POLYVECBYTES: usize = K * POLYBYTES;
const POLYCOMPRESSEDBYTES_DU: usize = 320;
const POLYCOMPRESSEDBYTES_DV: usize = 128;
const POLYVECCOMPRESSEDBYTES_DU: usize = K * POLYCOMPRESSEDBYTES_DU;

const PUBLICKEYBYTES: usize = 1184;
const SECRETKEYBYTES: usize = 2400;
const CIPHERTEXTBYTES: usize = 1088;
const SHAREDSECRETBYTES: usize = 32;
const SEEDBYTES: usize = 64;

const SHAKE128_BLOCKBYTES: usize = 168;

extern "C" {
    fn randombytes_buf(buf: *mut c_void, size: usize);
    fn sodium_memzero(pnt: *mut c_void, len: usize);
    fn sodium_memcmp(b1: *const c_void, b2: *const c_void, len: usize) -> c_int;

    fn crypto_hash_sha3256(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_hash_sha3512(out: *mut u8, inp: *const u8, inlen: u64) -> c_int;

    fn crypto_xof_shake128_init(state: *mut c_void) -> c_int;
    fn crypto_xof_shake128_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_xof_shake128_squeeze(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;

    fn crypto_xof_shake256_init(state: *mut c_void) -> c_int;
    fn crypto_xof_shake256_update(state: *mut c_void, inp: *const u8, inlen: u64) -> c_int;
    fn crypto_xof_shake256_squeeze(state: *mut c_void, out: *mut u8, outlen: usize) -> c_int;
}

#[repr(C, align(16))]
struct XofState {
    opaque: [u8; 256],
}

impl XofState {
    fn new() -> Self {
        XofState { opaque: [0u8; 256] }
    }
}

static ZETAS: [i16; 128] = [
    2285, 2571, 2970, 1812, 1493, 1422, 287, 202, 3158, 622, 1577, 182, 962, 2127, 1855, 1468, 573,
    2004, 264, 383, 2500, 1458, 1727, 3199, 2648, 1017, 732, 608, 1787, 411, 3124, 1758, 1223, 652,
    2777, 1015, 2036, 1491, 3047, 1785, 516, 3321, 3009, 2663, 1711, 2167, 126, 1469, 2476, 3239,
    3058, 830, 107, 1908, 3082, 2378, 2931, 961, 1821, 2604, 448, 2264, 677, 2054, 2226, 430, 555,
    843, 2078, 871, 1550, 105, 422, 587, 177, 3094, 3038, 2869, 1574, 1653, 3083, 778, 1159, 3182,
    2552, 1483, 2727, 1119, 1739, 644, 2457, 349, 418, 329, 3173, 3254, 817, 1097, 603, 610, 1322,
    2044, 1864, 384, 2114, 3193, 1218, 1994, 2455, 220, 2142, 1670, 2144, 1799, 2051, 794, 1819,
    2475, 2459, 478, 3221, 3021, 996, 991, 958, 1869, 1522, 1628,
];

#[derive(Clone, Copy)]
struct Poly {
    coeffs: [i16; N],
}

impl Poly {
    fn zero() -> Self {
        Poly { coeffs: [0i16; N] }
    }
}

#[derive(Clone, Copy)]
struct Polyvec {
    vec: [Poly; K],
}

impl Polyvec {
    fn zero() -> Self {
        Polyvec { vec: [Poly::zero(); K] }
    }
}

fn montgomery_reduce(a: i32) -> i16 {
    let t = ((a as u16 as u32).wrapping_mul(62209u32)) as i16;
    ((a.wrapping_sub((t as i32).wrapping_mul(Q))) >> 16) as i16
}

fn barrett_reduce(a: i16) -> i16 {
    let t = (((a as i32).wrapping_mul(20159)) >> 26) as i16;
    (a as i32).wrapping_sub((t as i32).wrapping_mul(Q)) as i16
}

fn csubq(mut a: i16) -> i16 {
    a = a.wrapping_sub(Q as i16);
    a = a.wrapping_add((a >> 15) & (Q as i16));
    a
}

fn poly_ntt(r: &mut Poly) {
    let mut k: usize = 1;
    let mut len = 128usize;
    while len >= 2 {
        let mut start = 0usize;
        while start < N {
            let zeta = ZETAS[k];
            k += 1;
            let mut j = start;
            while j < start + len {
                let t = montgomery_reduce((zeta as i32).wrapping_mul(r.coeffs[j + len] as i32));
                r.coeffs[j + len] = r.coeffs[j].wrapping_sub(t);
                r.coeffs[j] = r.coeffs[j].wrapping_add(t);
                j += 1;
            }
            start = j + len;
        }
        len >>= 1;
    }
}

fn poly_invntt(r: &mut Poly) {
    let f: i16 = 1441;
    let mut k: i32 = 127;
    let mut len = 2usize;
    while len <= 128 {
        let mut start = 0usize;
        while start < N {
            let zeta = ZETAS[k as usize];
            k -= 1;
            let mut j = start;
            while j < start + len {
                let t = r.coeffs[j];
                r.coeffs[j] = barrett_reduce(t.wrapping_add(r.coeffs[j + len]));
                r.coeffs[j + len] = montgomery_reduce(
                    (zeta as i32).wrapping_mul(r.coeffs[j + len].wrapping_sub(t) as i32),
                );
                j += 1;
            }
            start = j + len;
        }
        len <<= 1;
    }
    for j in 0..N {
        r.coeffs[j] = montgomery_reduce((f as i32).wrapping_mul(r.coeffs[j] as i32));
    }
}

fn poly_basemul(r: &mut Poly, a: &Poly, b: &Poly) {
    for i in 0..(N / 4) {
        let zeta = ZETAS[64 + i];

        let mut c0 = montgomery_reduce((a.coeffs[4 * i + 1] as i32).wrapping_mul(b.coeffs[4 * i + 1] as i32));
        c0 = montgomery_reduce((c0 as i32).wrapping_mul(zeta as i32));
        c0 = c0.wrapping_add(montgomery_reduce((a.coeffs[4 * i] as i32).wrapping_mul(b.coeffs[4 * i] as i32)));
        r.coeffs[4 * i] = c0;

        let mut c1 = montgomery_reduce((a.coeffs[4 * i] as i32).wrapping_mul(b.coeffs[4 * i + 1] as i32));
        c1 = c1.wrapping_add(montgomery_reduce((a.coeffs[4 * i + 1] as i32).wrapping_mul(b.coeffs[4 * i] as i32)));
        r.coeffs[4 * i + 1] = c1;

        let mut c2 = montgomery_reduce((a.coeffs[4 * i + 3] as i32).wrapping_mul(b.coeffs[4 * i + 3] as i32));
        c2 = montgomery_reduce((c2 as i32).wrapping_mul(-(zeta as i32)));
        c2 = c2.wrapping_add(montgomery_reduce((a.coeffs[4 * i + 2] as i32).wrapping_mul(b.coeffs[4 * i + 2] as i32)));
        r.coeffs[4 * i + 2] = c2;

        let mut c3 = montgomery_reduce((a.coeffs[4 * i + 2] as i32).wrapping_mul(b.coeffs[4 * i + 3] as i32));
        c3 = c3.wrapping_add(montgomery_reduce((a.coeffs[4 * i + 3] as i32).wrapping_mul(b.coeffs[4 * i + 2] as i32)));
        r.coeffs[4 * i + 3] = c3;
    }
}

fn poly_tomont(r: &mut Poly) {
    let f: i16 = 1353;
    for i in 0..N {
        r.coeffs[i] = montgomery_reduce((f as i32).wrapping_mul(r.coeffs[i] as i32));
    }
}

fn poly_reduce(r: &mut Poly) {
    for i in 0..N {
        r.coeffs[i] = barrett_reduce(r.coeffs[i]);
    }
}

fn poly_add(r: &mut Poly, a: &Poly, b: &Poly) {
    for i in 0..N {
        r.coeffs[i] = a.coeffs[i].wrapping_add(b.coeffs[i]);
    }
}

fn poly_sub(r: &mut Poly, a: &Poly, b: &Poly) {
    for i in 0..N {
        r.coeffs[i] = a.coeffs[i].wrapping_sub(b.coeffs[i]);
    }
}

fn poly_csubq(r: &mut Poly) {
    for i in 0..N {
        r.coeffs[i] = csubq(r.coeffs[i]);
    }
}

fn poly_cbd_eta2(r: &mut Poly, buf: &[u8]) {
    for i in 0..(N / 8) {
        let t = (buf[4 * i] as u32)
            | ((buf[4 * i + 1] as u32) << 8)
            | ((buf[4 * i + 2] as u32) << 16)
            | ((buf[4 * i + 3] as u32) << 24);

        let mut d = t & 0x55555555;
        d = d.wrapping_add((t >> 1) & 0x55555555);

        for j in 0..8 {
            let a = ((d >> (4 * j)) & 0x3) as i16;
            let b = ((d >> (4 * j + 2)) & 0x3) as i16;
            r.coeffs[8 * i + j] = a.wrapping_sub(b);
        }
    }
}

unsafe fn poly_getnoise_eta2(r: &mut Poly, seed: *const u8, nonce: u8) {
    let mut buf = [0u8; ETA2 * N / 4];
    let mut state = XofState::new();
    let sp = &mut state as *mut _ as *mut c_void;
    let mut extseed = [0u8; 33];

    core::ptr::copy_nonoverlapping(seed, extseed.as_mut_ptr(), 32);
    extseed[32] = nonce;

    crypto_xof_shake256_init(sp);
    crypto_xof_shake256_update(sp, extseed.as_ptr(), 33);
    crypto_xof_shake256_squeeze(sp, buf.as_mut_ptr(), buf.len());

    poly_cbd_eta2(r, &buf);
    sodium_memzero(sp, core::mem::size_of::<XofState>());
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
}

fn poly_frombytes(r: &mut Poly, a: &[u8]) {
    for i in 0..(N / 2) {
        r.coeffs[2 * i] = (((a[3 * i] as u16) >> 0) | ((a[3 * i + 1] as u16) << 8)) as i16 & 0xFFF;
        r.coeffs[2 * i + 1] = (((a[3 * i + 1] as u16) >> 4) | ((a[3 * i + 2] as u16) << 4)) as i16 & 0xFFF;
    }
}

fn poly_tobytes(r: &mut [u8], a: &Poly) {
    for i in 0..(N / 2) {
        let t0 = a.coeffs[2 * i] as u16;
        let t1 = a.coeffs[2 * i + 1] as u16;
        r[3 * i] = (t0 >> 0) as u8;
        r[3 * i + 1] = ((t0 >> 8) | (t1 << 4)) as u8;
        r[3 * i + 2] = (t1 >> 4) as u8;
    }
}

fn poly_frommsg(r: &mut Poly, msg: &[u8]) {
    for i in 0..(N / 8) {
        for j in 0..8 {
            let mask = (0i16).wrapping_sub(((msg[i] >> j) & 1) as i16);
            r.coeffs[8 * i + j] = mask & (((Q + 1) / 2) as i16);
        }
    }
}

fn poly_tomsg(msg: &mut [u8], a: &Poly) {
    for i in 0..(N / 8) {
        msg[i] = 0;
        for j in 0..8 {
            let mut t = a.coeffs[8 * i + j] as u32;
            t = t.wrapping_add((((a.coeffs[8 * i + j] as i32) >> 15) & Q) as u32);
            t = ((t << 1).wrapping_add((Q / 2) as u32)).wrapping_mul(80635) >> 28;
            t &= 1;
            msg[i] |= (t << j) as u8;
        }
    }
}

fn poly_compress_du(r: &mut [u8], a: &Poly) {
    let mut t = [0u32; 4];
    for i in 0..(N / 4) {
        for j in 0..4 {
            t[j] = a.coeffs[4 * i + j] as u32;
            t[j] = t[j].wrapping_add((((a.coeffs[4 * i + j] as i32) >> 15) & Q) as u32);
            t[j] = ((((t[j] as u64) << 10).wrapping_add((Q / 2) as u64)).wrapping_mul(161271u64) >> 29) as u32;
            t[j] &= 0x3ff;
        }

        r[5 * i] = (t[0] >> 0) as u8;
        r[5 * i + 1] = ((t[0] >> 8) | (t[1] << 2)) as u8;
        r[5 * i + 2] = ((t[1] >> 6) | (t[2] << 4)) as u8;
        r[5 * i + 3] = ((t[2] >> 4) | (t[3] << 6)) as u8;
        r[5 * i + 4] = (t[3] >> 2) as u8;
    }
}

fn poly_decompress_du(r: &mut Poly, a: &[u8]) {
    let mut t = [0u16; 4];
    for i in 0..(N / 4) {
        t[0] = ((a[5 * i] as u16) >> 0) | ((a[5 * i + 1] as u16) << 8);
        t[1] = ((a[5 * i + 1] as u16) >> 2) | ((a[5 * i + 2] as u16) << 6);
        t[2] = ((a[5 * i + 2] as u16) >> 4) | ((a[5 * i + 3] as u16) << 4);
        t[3] = ((a[5 * i + 3] as u16) >> 6) | ((a[5 * i + 4] as u16) << 2);

        for j in 0..4 {
            r.coeffs[4 * i + j] =
                ((((t[j] & 0x3FF) as u32).wrapping_mul(Q as u32).wrapping_add(512)) >> 10) as i16;
        }
    }
}

fn poly_compress_dv(r: &mut [u8], a: &Poly) {
    let mut t = [0u32; 8];
    for i in 0..(N / 8) {
        for j in 0..8 {
            t[j] = a.coeffs[8 * i + j] as u32;
            t[j] = t[j].wrapping_add((((a.coeffs[8 * i + j] as i32) >> 15) & Q) as u32);
            t[j] = ((((t[j] as u64) << 4).wrapping_add((Q / 2) as u64)).wrapping_mul(161271u64) >> 29) as u32;
            t[j] &= 0xf;
        }

        r[4 * i] = (t[0] | (t[1] << 4)) as u8;
        r[4 * i + 1] = (t[2] | (t[3] << 4)) as u8;
        r[4 * i + 2] = (t[4] | (t[5] << 4)) as u8;
        r[4 * i + 3] = (t[6] | (t[7] << 4)) as u8;
    }
}

fn poly_decompress_dv(r: &mut Poly, a: &[u8]) {
    for i in 0..(N / 2) {
        r.coeffs[2 * i] = ((((a[i] & 15) as u16).wrapping_mul(Q as u16).wrapping_add(8)) >> 4) as i16;
        r.coeffs[2 * i + 1] = ((((a[i] >> 4) as u16).wrapping_mul(Q as u16).wrapping_add(8)) >> 4) as i16;
    }
}

fn polyvec_ntt(r: &mut Polyvec) {
    for i in 0..K {
        poly_ntt(&mut r.vec[i]);
    }
}

fn polyvec_invntt(r: &mut Polyvec) {
    for i in 0..K {
        poly_invntt(&mut r.vec[i]);
    }
}

fn polyvec_basemul_acc(r: &mut Poly, a: &Polyvec, b: &Polyvec) {
    let mut t = Poly::zero();
    poly_basemul(r, &a.vec[0], &b.vec[0]);
    for i in 1..K {
        poly_basemul(&mut t, &a.vec[i], &b.vec[i]);
        let rc = *r;
        poly_add(r, &rc, &t);
    }
    poly_reduce(r);
}

fn polyvec_reduce(r: &mut Polyvec) {
    for i in 0..K {
        poly_reduce(&mut r.vec[i]);
    }
}

fn polyvec_csubq(r: &mut Polyvec) {
    for i in 0..K {
        poly_csubq(&mut r.vec[i]);
    }
}

fn polyvec_add(r: &mut Polyvec, a: &Polyvec, b: &Polyvec) {
    for i in 0..K {
        let (ai, bi) = (a.vec[i], b.vec[i]);
        poly_add(&mut r.vec[i], &ai, &bi);
    }
}

fn polyvec_tobytes(r: &mut [u8], a: &Polyvec) {
    for i in 0..K {
        poly_tobytes(&mut r[i * POLYBYTES..], &a.vec[i]);
    }
}

fn polyvec_frombytes(r: &mut Polyvec, a: &[u8]) {
    for i in 0..K {
        poly_frombytes(&mut r.vec[i], &a[i * POLYBYTES..]);
    }
}

fn polyvec_is_canonical(a: &Polyvec) -> c_int {
    for i in 0..K {
        for j in 0..N {
            if (a.vec[i].coeffs[j] as u16) >= (Q as u16) {
                return 0;
            }
        }
    }
    1
}

fn polyvec_compress(r: &mut [u8], a: &Polyvec) {
    for i in 0..K {
        poly_compress_du(&mut r[i * POLYCOMPRESSEDBYTES_DU..], &a.vec[i]);
    }
}

fn polyvec_decompress(r: &mut Polyvec, a: &[u8]) {
    for i in 0..K {
        poly_decompress_du(&mut r.vec[i], &a[i * POLYCOMPRESSEDBYTES_DU..]);
    }
}

fn rej_uniform(r: &mut [i16], len: usize, buf: &[u8], buflen: usize) -> usize {
    let mut ctr = 0usize;
    let mut pos = 0usize;
    while ctr < len && pos + 3 <= buflen {
        let val0 = (((buf[pos] as u16) >> 0) | ((buf[pos + 1] as u16) << 8)) & 0xFFF;
        let val1 = (((buf[pos + 1] as u16) >> 4) | ((buf[pos + 2] as u16) << 4)) & 0xFFF;
        pos += 3;

        if (val0 as i32) < Q {
            r[ctr] = val0 as i16;
            ctr += 1;
        }
        if ctr < len && (val1 as i32) < Q {
            r[ctr] = val1 as i16;
            ctr += 1;
        }
    }
    ctr
}

// GEN_MATRIX_NBLOCKS = ((12*256/8*4096/3329 + 168) / 168) = 3
const GEN_MATRIX_NBLOCKS: usize = 3;

unsafe fn gen_matrix(a: &mut [Polyvec; K], seed: &[u8], transposed: bool) {
    let mut buf = [0u8; GEN_MATRIX_NBLOCKS * SHAKE128_BLOCKBYTES + 2];
    let mut extseed = [0u8; 34];

    core::ptr::copy_nonoverlapping(seed.as_ptr(), extseed.as_mut_ptr(), 32);

    let mut state = XofState::new();
    let sp = &mut state as *mut _ as *mut c_void;

    for i in 0..K {
        for j in 0..K {
            if transposed {
                extseed[32] = i as u8;
                extseed[33] = j as u8;
            } else {
                extseed[32] = j as u8;
                extseed[33] = i as u8;
            }

            crypto_xof_shake128_init(sp);
            crypto_xof_shake128_update(sp, extseed.as_ptr(), 34);

            let buflen = GEN_MATRIX_NBLOCKS * SHAKE128_BLOCKBYTES;
            crypto_xof_shake128_squeeze(sp, buf.as_mut_ptr(), buflen);

            let mut ctr = rej_uniform(&mut a[i].vec[j].coeffs, N, &buf, buflen);

            while ctr < N {
                crypto_xof_shake128_squeeze(sp, buf.as_mut_ptr(), SHAKE128_BLOCKBYTES);
                ctr += rej_uniform(&mut a[i].vec[j].coeffs[ctr..], N - ctr, &buf, SHAKE128_BLOCKBYTES);
            }
        }
    }
}

unsafe fn indcpa_keypair(pk: &mut [u8], sk: &mut [u8], seed: &[u8]) {
    let mut a = [Polyvec::zero(); K];
    let mut e = Polyvec::zero();
    let mut pkpv = Polyvec::zero();
    let mut skpv = Polyvec::zero();
    let mut buf = [0u8; 64];
    let mut nonce: u8 = 0;

    crypto_hash_sha3512(buf.as_mut_ptr(), seed.as_ptr(), 33);

    // publicseed = buf[0..32], noiseseed = buf[32..64]
    let (publicseed, noiseseed) = buf.split_at(32);
    let publicseed = publicseed.to_vec();
    let noiseseed = noiseseed.to_vec();

    gen_matrix(&mut a, &publicseed, false);

    for i in 0..K {
        poly_getnoise_eta2(&mut skpv.vec[i], noiseseed.as_ptr(), nonce);
        nonce = nonce.wrapping_add(1);
    }
    for i in 0..K {
        poly_getnoise_eta2(&mut e.vec[i], noiseseed.as_ptr(), nonce);
        nonce = nonce.wrapping_add(1);
    }

    polyvec_ntt(&mut skpv);
    polyvec_ntt(&mut e);

    for i in 0..K {
        polyvec_basemul_acc(&mut pkpv.vec[i], &a[i], &skpv);
        poly_tomont(&mut pkpv.vec[i]);
    }

    let pkpv_copy = pkpv;
    polyvec_add(&mut pkpv, &pkpv_copy, &e);
    polyvec_reduce(&mut pkpv);
    polyvec_csubq(&mut pkpv);
    polyvec_reduce(&mut skpv);
    polyvec_csubq(&mut skpv);

    polyvec_tobytes(sk, &skpv);
    polyvec_tobytes(pk, &pkpv);
    pk[POLYVECBYTES..POLYVECBYTES + 32].copy_from_slice(&publicseed);

    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(&mut skpv as *mut _ as *mut c_void, core::mem::size_of::<Polyvec>());
    sodium_memzero(&mut e as *mut _ as *mut c_void, core::mem::size_of::<Polyvec>());
}

unsafe fn indcpa_enc(ct: &mut [u8], m: &[u8], pk: &[u8], coins: *const u8) {
    let mut sp = Polyvec::zero();
    let mut pkpv = Polyvec::zero();
    let mut ep = Polyvec::zero();
    let mut at = [Polyvec::zero(); K];
    let mut b = Polyvec::zero();
    let mut v = Poly::zero();
    let mut k = Poly::zero();
    let mut epp = Poly::zero();
    let mut seed = [0u8; 32];
    let mut nonce: u8 = 0;

    seed.copy_from_slice(&pk[POLYVECBYTES..POLYVECBYTES + 32]);

    polyvec_frombytes(&mut pkpv, pk);

    poly_frommsg(&mut k, m);

    gen_matrix(&mut at, &seed, true);

    for i in 0..K {
        poly_getnoise_eta2(&mut sp.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
    }
    for i in 0..K {
        poly_getnoise_eta2(&mut ep.vec[i], coins, nonce);
        nonce = nonce.wrapping_add(1);
    }
    poly_getnoise_eta2(&mut epp, coins, nonce);
    nonce = nonce.wrapping_add(1);
    let _ = nonce;

    polyvec_ntt(&mut sp);
    polyvec_reduce(&mut sp);

    for i in 0..K {
        polyvec_basemul_acc(&mut b.vec[i], &at[i], &sp);
    }

    polyvec_basemul_acc(&mut v, &pkpv, &sp);

    polyvec_invntt(&mut b);
    poly_invntt(&mut v);

    let b_copy = b;
    polyvec_add(&mut b, &b_copy, &ep);
    let v_copy = v;
    poly_add(&mut v, &v_copy, &epp);
    let v_copy = v;
    poly_add(&mut v, &v_copy, &k);

    polyvec_reduce(&mut b);
    poly_reduce(&mut v);
    polyvec_csubq(&mut b);
    poly_csubq(&mut v);

    polyvec_compress(ct, &b);
    poly_compress_dv(&mut ct[POLYVECCOMPRESSEDBYTES_DU..], &v);

    sodium_memzero(&mut sp as *mut _ as *mut c_void, core::mem::size_of::<Polyvec>());
    sodium_memzero(&mut ep as *mut _ as *mut c_void, core::mem::size_of::<Polyvec>());
    sodium_memzero(&mut epp as *mut _ as *mut c_void, core::mem::size_of::<Poly>());
    sodium_memzero(&mut k as *mut _ as *mut c_void, core::mem::size_of::<Poly>());
}

unsafe fn indcpa_dec(m: &mut [u8], ct: &[u8], sk: &[u8]) {
    let mut b = Polyvec::zero();
    let mut skpv = Polyvec::zero();
    let mut v = Poly::zero();
    let mut mp = Poly::zero();

    polyvec_decompress(&mut b, ct);
    poly_decompress_dv(&mut v, &ct[POLYVECCOMPRESSEDBYTES_DU..]);

    polyvec_frombytes(&mut skpv, sk);

    polyvec_ntt(&mut b);
    polyvec_reduce(&mut b);
    polyvec_basemul_acc(&mut mp, &skpv, &b);
    poly_invntt(&mut mp);

    let v_ref = v;
    let mp_copy = mp;
    poly_sub(&mut mp, &v_ref, &mp_copy);
    poly_reduce(&mut mp);
    poly_csubq(&mut mp);

    poly_tomsg(m, &mp);

    sodium_memzero(&mut skpv as *mut _ as *mut c_void, core::mem::size_of::<Polyvec>());
    sodium_memzero(&mut mp as *mut _ as *mut c_void, core::mem::size_of::<Poly>());
}

unsafe fn cmov(r: *mut u8, x: *const u8, len: usize, b: u8) {
    let mask = (0i32.wrapping_sub(b as i32)) as u8;
    for i in 0..len {
        *r.add(i) ^= mask & (*r.add(i) ^ *x.add(i));
    }
}

// ---------------- exported ref functions ----------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    let mut indseed = [0u8; 33];
    core::ptr::copy_nonoverlapping(seed, indseed.as_mut_ptr(), 32);
    indseed[32] = K as u8;

    let pk_slice = core::slice::from_raw_parts_mut(pk, PUBLICKEYBYTES);
    let sk_slice = core::slice::from_raw_parts_mut(sk, SECRETKEYBYTES);

    indcpa_keypair(pk_slice, sk_slice, &indseed);
    sodium_memzero(indseed.as_mut_ptr() as *mut c_void, indseed.len());

    // sk[POLYVECBYTES..] = pk
    core::ptr::copy_nonoverlapping(pk, sk.add(POLYVECBYTES), PUBLICKEYBYTES);
    crypto_hash_sha3256(
        sk.add(POLYVECBYTES + PUBLICKEYBYTES),
        pk,
        PUBLICKEYBYTES as u64,
    );
    core::ptr::copy_nonoverlapping(seed.add(32), sk.add(POLYVECBYTES + PUBLICKEYBYTES + 32), 32);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    let mut seed = [0u8; SEEDBYTES];
    randombytes_buf(seed.as_mut_ptr() as *mut c_void, SEEDBYTES);
    let ret = _sodium_mlkem768_ref_seed_keypair(pk, sk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());
    ret
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    let mut pkpv = Polyvec::zero();
    let mut buf = [0u8; 64];
    let mut kr = [0u8; 64];

    let pk_slice = core::slice::from_raw_parts(pk, PUBLICKEYBYTES);
    polyvec_frombytes(&mut pkpv, pk_slice);
    if polyvec_is_canonical(&pkpv) == 0 {
        return -1;
    }

    core::ptr::copy_nonoverlapping(seed, buf.as_mut_ptr(), 32);
    crypto_hash_sha3256(buf.as_mut_ptr().add(32), pk, PUBLICKEYBYTES as u64);

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    let ct_slice = core::slice::from_raw_parts_mut(ct, CIPHERTEXTBYTES);
    indcpa_enc(ct_slice, &buf[..32], pk_slice, kr.as_ptr().add(32));

    core::ptr::copy_nonoverlapping(kr.as_ptr(), ss, SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, kr.len());

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _sodium_mlkem768_ref_enc(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
) -> c_int {
    let mut seed = [0u8; 32];
    randombytes_buf(seed.as_mut_ptr() as *mut c_void, 32);
    let ret = _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed.as_ptr());
    sodium_memzero(seed.as_mut_ptr() as *mut c_void, seed.len());
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
    let mut k_bar = [0u8; SHAREDSECRETBYTES];
    let mut cmp = [0u8; CIPHERTEXTBYTES];

    let pk = sk.add(POLYVECBYTES);
    let hpk = sk.add(POLYVECBYTES + PUBLICKEYBYTES);
    let z = sk.add(POLYVECBYTES + PUBLICKEYBYTES + 32);

    let ct_slice = core::slice::from_raw_parts(ct, CIPHERTEXTBYTES);
    let sk_slice = core::slice::from_raw_parts(sk, POLYVECBYTES);

    indcpa_dec(&mut buf[..32], ct_slice, sk_slice);

    core::ptr::copy_nonoverlapping(hpk, buf.as_mut_ptr().add(32), 32);

    crypto_hash_sha3512(kr.as_mut_ptr(), buf.as_ptr(), 64);

    let pk_slice = core::slice::from_raw_parts(pk, PUBLICKEYBYTES);
    indcpa_enc(&mut cmp, &buf[..32], pk_slice, kr.as_ptr().add(32));

    let fail = sodium_memcmp(ct as *const c_void, cmp.as_ptr() as *const c_void, CIPHERTEXTBYTES);
    let mut fail_mask = fail as u32;
    fail_mask >>= (core::mem::size_of::<u32>() * 8) as u32 - 1;

    let mut state = XofState::new();
    let sp = &mut state as *mut _ as *mut c_void;
    crypto_xof_shake256_init(sp);
    crypto_xof_shake256_update(sp, z, 32);
    crypto_xof_shake256_update(sp, ct, CIPHERTEXTBYTES as u64);
    crypto_xof_shake256_squeeze(sp, k_bar.as_mut_ptr(), SHAREDSECRETBYTES);

    cmov(kr.as_mut_ptr(), k_bar.as_ptr(), SHAREDSECRETBYTES, fail_mask as u8);

    core::ptr::copy_nonoverlapping(kr.as_ptr(), ss, SHAREDSECRETBYTES);
    sodium_memzero(buf.as_mut_ptr() as *mut c_void, buf.len());
    sodium_memzero(kr.as_mut_ptr() as *mut c_void, kr.len());
    sodium_memzero(k_bar.as_mut_ptr() as *mut c_void, k_bar.len());
    sodium_memzero(cmp.as_mut_ptr() as *mut c_void, cmp.len());
    sodium_memzero(sp, core::mem::size_of::<XofState>());

    0
}

// ---------------- kem_mlkem768.c accessors + dispatch ----------------

#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_mlkem768_publickeybytes() -> usize {
    PUBLICKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_mlkem768_secretkeybytes() -> usize {
    SECRETKEYBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_mlkem768_ciphertextbytes() -> usize {
    CIPHERTEXTBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_mlkem768_sharedsecretbytes() -> usize {
    SHAREDSECRETBYTES
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_kem_mlkem768_seedbytes() -> usize {
    SEEDBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_seed_keypair(
    pk: *mut u8,
    sk: *mut u8,
    seed: *const u8,
) -> c_int {
    _sodium_mlkem768_ref_seed_keypair(pk, sk, seed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_keypair(pk: *mut u8, sk: *mut u8) -> c_int {
    _sodium_mlkem768_ref_keypair(pk, sk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc(ct: *mut u8, ss: *mut u8, pk: *const u8) -> c_int {
    _sodium_mlkem768_ref_enc(ct, ss, pk)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_enc_deterministic(
    ct: *mut u8,
    ss: *mut u8,
    pk: *const u8,
    seed: *const u8,
) -> c_int {
    _sodium_mlkem768_ref_enc_deterministic(ct, ss, pk, seed)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_kem_mlkem768_dec(ss: *mut u8, ct: *const u8, sk: *const u8) -> c_int {
    _sodium_mlkem768_ref_dec(ss, ct, sk)
}
