//! Area 7 — `crypto_kem`: ML-KEM-768 (`crypto_kem_mlkem768_*`), the X-Wing
//! hybrid (`crypto_kem_xwing_*`) and the generic `crypto_kem_*` dispatch
//! (which is a set of thin aliases for xwing).
//!
//! Covers `configs_7.md` rows 7.113–7.128 and `errors_7.md` rows 7.110–7.126.
//!
//! Load-bearing subtleties exercised here:
//!
//! * `mlkem768_ref_dec` has **no** ciphertext validation and **never** returns
//!   `-1`.  On re-encryption mismatch it constant-time-swaps in
//!   `k_bar = SHAKE256(z ‖ ct)[0..32]` (implicit rejection).  Every corruption
//!   test therefore asserts `rc == 0`, `ss != ss_enc`, `ss_C == ss_Rust` *and*
//!   `ss == SHAKE256(z ‖ ct)` computed independently.
//! * Consequently the `mlkem768_dec != 0` branch of `crypto_kem_xwing_dec` is
//!   dead; the only reachable xwing decapsulation failure is a small-order
//!   `ct[1088..1120]`, which is constructed explicitly below.
//! * Seed-length contract mismatch: `crypto_kem_xwing_SEEDBYTES == 32` but
//!   `crypto_kem_xwing_enc_deterministic` consumes `seed[64]`; ML-KEM-768 is
//!   the mirror image (`_SEEDBYTES == 64`, `_enc_deterministic` consumes 32).
//!   Both are pinned by observing which seed bytes influence the output.
//! * Every output buffer is pre-filled with a distinctive pattern and compared
//!   C-vs-Rust byte for byte *after a failure*, not only via the return code.
mod common;
use common::*;
use std::ffi::{c_char, c_int, CStr};

// ------------------------------------------------------------------- sizes

const MLKEM_PK: usize = 1184;
const MLKEM_SK: usize = 2400;
const MLKEM_CT: usize = 1088;
const MLKEM_SEED: usize = 64; // crypto_kem_mlkem768_SEEDBYTES
const MLKEM_ENC_SEED: usize = 32; // what _enc_deterministic actually reads
const SS: usize = 32;

const XWING_PK: usize = 1216;
const XWING_SK: usize = 32;
const XWING_CT: usize = 1120;
const XWING_SEED: usize = 32; // crypto_kem_xwing_SEEDBYTES
const XWING_ENC_SEED: usize = 64; // what _enc_deterministic actually reads

/// `MLKEM768_POLYVECBYTES` — offset of the embedded `pk` inside the ML-KEM `sk`.
const POLYVECBYTES: usize = 1152;

const XWING_LABEL: [u8; 6] = [0x5c, 0x2e, 0x2f, 0x2f, 0x5e, 0x5c];

// ------------------------------------------------------------------- types

type Kp = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type Enc = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type EncD = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> c_int;
type Dec = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;
type SizeFn = unsafe extern "C" fn() -> usize;
type StrFn = unsafe extern "C" fn() -> *const c_char;
type ShakeFn = unsafe extern "C" fn(*mut u8, usize, *const u8, u64) -> c_int;
type Sha3Fn = unsafe extern "C" fn(*mut u8, *const u8, u64) -> c_int;
type BaseFn = unsafe extern "C" fn(*mut u8, *const u8) -> c_int;
type MultFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> c_int;

// ----------------------------------------------------------------- helpers

/// Distinctive prefill so that "output buffer untouched" is observable.
fn pat(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| 0x5Au8.wrapping_add((i as u8).wrapping_mul(31)))
        .collect()
}

fn prefilled(n: usize) -> Vec<u8> {
    let mut v = padded(n);
    v[..n].copy_from_slice(&pat(n));
    v
}

/// SHAKE256 through the *C* library (independent ground truth for expectations).
fn c_shake256(outlen: usize, input: &[u8]) -> Vec<u8> {
    let (c, _r) = both::<ShakeFn>("crypto_xof_shake256");
    let mut out = vec![0u8; outlen];
    let rc = unsafe { (c)(out.as_mut_ptr(), outlen, input.as_ptr(), input.len() as u64) };
    assert_eq!(rc, 0, "crypto_xof_shake256 failed");
    out
}

/// SHA3-256 through the *C* library.
fn c_sha3_256(input: &[u8]) -> Vec<u8> {
    let (c, _r) = both::<Sha3Fn>("crypto_hash_sha3256");
    let mut out = vec![0u8; 32];
    let rc = unsafe { (c)(out.as_mut_ptr(), input.as_ptr(), input.len() as u64) };
    assert_eq!(rc, 0, "crypto_hash_sha3256 failed");
    out
}

fn c_x25519_base(n: &[u8]) -> Vec<u8> {
    let (c, _r) = both::<BaseFn>("crypto_scalarmult_curve25519_base");
    let mut out = vec![0u8; 32];
    let rc = unsafe { (c)(out.as_mut_ptr(), n.as_ptr()) };
    assert_eq!(rc, 0, "curve25519_base failed");
    out
}

fn c_x25519(n: &[u8], p: &[u8]) -> (c_int, Vec<u8>) {
    let (c, _r) = both::<MultFn>("crypto_scalarmult_curve25519");
    let mut out = vec![0u8; 32];
    let rc = unsafe { (c)(out.as_mut_ptr(), n.as_ptr(), p.as_ptr()) };
    (rc, out)
}

// ------------------------------------------------- differential invokers

/// `*_seed_keypair(pk, sk, seed)` in both libraries.  Returns `(rc, pk, sk)`.
#[track_caller]
fn d_seed_keypair(name: &str, pklen: usize, sklen: usize, seed: &[u8]) -> (c_int, Vec<u8>, Vec<u8>) {
    let (c, r) = both::<SKp>(name);
    let (mut pkc, mut pkr) = (prefilled(pklen), prefilled(pklen));
    let (mut skc, mut skr) = (prefilled(sklen), prefilled(sklen));
    let rcc = unsafe { (c)(pkc.as_mut_ptr(), skc.as_mut_ptr(), seed.as_ptr()) };
    let rcr = unsafe { (r)(pkr.as_mut_ptr(), skr.as_mut_ptr(), seed.as_ptr()) };
    eqi(&format!("{name}(seed={}) rc", hex(seed)), rcc, rcr);
    eqb(&format!("{name}(seed={}) pk", hex(seed)), &pkc, &pkr);
    eqb(&format!("{name}(seed={}) sk", hex(seed)), &skc, &skr);
    check_pad(&format!("{name} pk(C)"), &pkc, pklen);
    check_pad(&format!("{name} pk(Rust)"), &pkr, pklen);
    check_pad(&format!("{name} sk(C)"), &skc, sklen);
    check_pad(&format!("{name} sk(Rust)"), &skr, sklen);
    (rcc, pkc[..pklen].to_vec(), skc[..sklen].to_vec())
}

/// `*_keypair(pk, sk)` in both libraries with the RNG streams rewound first.
#[track_caller]
fn d_keypair(name: &str, pklen: usize, sklen: usize, rng_seed: u64) -> (c_int, Vec<u8>, Vec<u8>) {
    let (c, r) = both::<Kp>(name);
    let (mut pkc, mut pkr) = (prefilled(pklen), prefilled(pklen));
    let (mut skc, mut skr) = (prefilled(sklen), prefilled(sklen));
    rng_reseed(rng_seed);
    let rcc = unsafe { (c)(pkc.as_mut_ptr(), skc.as_mut_ptr()) };
    let rcr = unsafe { (r)(pkr.as_mut_ptr(), skr.as_mut_ptr()) };
    eqi(&format!("{name}() rc"), rcc, rcr);
    eqb(&format!("{name}() pk"), &pkc, &pkr);
    eqb(&format!("{name}() sk"), &skc, &skr);
    check_pad(&format!("{name} pk(C)"), &pkc, pklen);
    check_pad(&format!("{name} pk(Rust)"), &pkr, pklen);
    check_pad(&format!("{name} sk(C)"), &skc, sklen);
    check_pad(&format!("{name} sk(Rust)"), &skr, sklen);
    (rcc, pkc[..pklen].to_vec(), skc[..sklen].to_vec())
}

/// `*_enc(ct, ss, pk)` in both libraries with the RNG streams rewound first.
#[track_caller]
fn d_enc(name: &str, ctlen: usize, pk: &[u8], rng_seed: u64) -> (c_int, Vec<u8>, Vec<u8>) {
    let (c, r) = both::<Enc>(name);
    let (mut ctc, mut ctr) = (prefilled(ctlen), prefilled(ctlen));
    let (mut ssc, mut ssr) = (prefilled(SS), prefilled(SS));
    rng_reseed(rng_seed);
    let rcc = unsafe { (c)(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr()) };
    let rcr = unsafe { (r)(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr()) };
    eqi(&format!("{name}() rc"), rcc, rcr);
    eqb(&format!("{name}() ct"), &ctc, &ctr);
    eqb(&format!("{name}() ss"), &ssc, &ssr);
    check_pad(&format!("{name} ct(C)"), &ctc, ctlen);
    check_pad(&format!("{name} ct(Rust)"), &ctr, ctlen);
    check_pad(&format!("{name} ss(C)"), &ssc, SS);
    check_pad(&format!("{name} ss(Rust)"), &ssr, SS);
    (rcc, ctc[..ctlen].to_vec(), ssc[..SS].to_vec())
}

/// `*_enc_deterministic(ct, ss, pk, seed)` in both libraries.
#[track_caller]
fn d_enc_det(name: &str, ctlen: usize, pk: &[u8], seed: &[u8]) -> (c_int, Vec<u8>, Vec<u8>) {
    let (c, r) = both::<EncD>(name);
    let (mut ctc, mut ctr) = (prefilled(ctlen), prefilled(ctlen));
    let (mut ssc, mut ssr) = (prefilled(SS), prefilled(SS));
    let rcc = unsafe { (c)(ctc.as_mut_ptr(), ssc.as_mut_ptr(), pk.as_ptr(), seed.as_ptr()) };
    let rcr = unsafe { (r)(ctr.as_mut_ptr(), ssr.as_mut_ptr(), pk.as_ptr(), seed.as_ptr()) };
    eqi(&format!("{name}(seed={}) rc", hex(seed)), rcc, rcr);
    eqb(&format!("{name}(seed={}) ct", hex(seed)), &ctc, &ctr);
    eqb(&format!("{name}(seed={}) ss", hex(seed)), &ssc, &ssr);
    check_pad(&format!("{name} ct(C)"), &ctc, ctlen);
    check_pad(&format!("{name} ct(Rust)"), &ctr, ctlen);
    check_pad(&format!("{name} ss(C)"), &ssc, SS);
    check_pad(&format!("{name} ss(Rust)"), &ssr, SS);
    (rcc, ctc[..ctlen].to_vec(), ssc[..SS].to_vec())
}

/// `*_dec(ss, ct, sk)` in both libraries.  Also reports the raw C/Rust `ss`
/// buffers (including the guard tail) so that "untouched on failure" is checked.
#[track_caller]
fn d_dec(name: &str, ct: &[u8], sk: &[u8]) -> (c_int, Vec<u8>) {
    let (c, r) = both::<Dec>(name);
    let (mut ssc, mut ssr) = (prefilled(SS), prefilled(SS));
    let rcc = unsafe { (c)(ssc.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
    let rcr = unsafe { (r)(ssr.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) };
    eqi(&format!("{name}() rc"), rcc, rcr);
    eqb(&format!("{name}() ss"), &ssc, &ssr);
    check_pad(&format!("{name} ss(C)"), &ssc, SS);
    check_pad(&format!("{name} ss(Rust)"), &ssr, SS);
    (rcc, ssc[..SS].to_vec())
}

// ------------------------------------------------- ML-KEM pk manipulation

/// Force global coefficient `(poly, coeff)` of the packed polyvec `pk[0..1152]`
/// to `v` (only the low 12 bits matter).
fn set_coeff(pk: &mut [u8], poly: usize, coeff: usize, v: u16) {
    assert!(poly < 3 && coeff < 256);
    let base = poly * 384 + 3 * (coeff / 2);
    let v = v & 0xfff;
    if coeff % 2 == 0 {
        pk[base] = (v & 0xff) as u8;
        pk[base + 1] = (pk[base + 1] & 0xf0) | ((v >> 8) as u8 & 0x0f);
    } else {
        pk[base + 1] = (pk[base + 1] & 0x0f) | (((v & 0x0f) as u8) << 4);
        pk[base + 2] = (v >> 4) as u8;
    }
}

/// The seven blocklisted small-order curve25519 encodings, plus high-bit-set
/// variants (`has_small_order` masks `s[31] & 0x7f`, so those are rejected too).
fn small_order_encodings() -> Vec<[u8; 32]> {
    let mut v: Vec<[u8; 32]> = Vec::new();
    let mut z = [0u8; 32];
    v.push(z); // 0 (order 4)
    z = [0u8; 32];
    z[0] = 1;
    v.push(z); // 1 (order 1)
    v.push([
        0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4,
        0x6a, 0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49,
        0xb8, 0x00,
    ]);
    v.push([
        0x5f, 0x9c, 0x95, 0xbc, 0xa3, 0x50, 0x8c, 0x24, 0xb1, 0xd0, 0xb1, 0x55, 0x9c, 0x83, 0xef,
        0x5b, 0x04, 0x44, 0x5c, 0xc4, 0x58, 0x1c, 0x8e, 0x86, 0xd8, 0x22, 0x4e, 0xdd, 0xd0, 0x9f,
        0x11, 0x57,
    ]);
    let mut ff = [0xffu8; 32];
    ff[0] = 0xec;
    ff[31] = 0x7f;
    v.push(ff); // p-1 (order 2)
    ff[0] = 0xed;
    v.push(ff); // p   (=0, order 4)
    ff[0] = 0xee;
    v.push(ff); // p+1 (=1, order 1)
    // High-bit variants: the guard compares (s[31] & 0x7f), so these are also
    // rejected — an 8th, 9th and 10th case.
    let n = v.len();
    for i in [0usize, 4, n - 1] {
        let mut w = v[i];
        w[31] |= 0x80;
        v.push(w);
    }
    v
}

// =========================================================================
// accessors / dispatch — configs 7.119, 7.126, 7.127, 7.128; errors 7.127
// =========================================================================

#[track_caller]
fn size_eq(name: &str, expect: usize) {
    let (c, r) = both::<SizeFn>(name);
    let (vc, vr) = unsafe { ((c)(), (r)()) };
    assert_eq!(vc, expect, "{name}(): C returned {vc}, expected {expect}");
    assert_eq!(vr, vc, "{name}(): Rust {vr} != C {vc}");
}

#[test]
fn kem_accessors() {
    // configs 7.119
    size_eq("crypto_kem_mlkem768_publickeybytes", MLKEM_PK);
    size_eq("crypto_kem_mlkem768_secretkeybytes", MLKEM_SK);
    size_eq("crypto_kem_mlkem768_ciphertextbytes", MLKEM_CT);
    size_eq("crypto_kem_mlkem768_sharedsecretbytes", SS);
    size_eq("crypto_kem_mlkem768_seedbytes", MLKEM_SEED);
    // configs 7.126 — note SECRETKEYBYTES == SEEDBYTES == 32
    size_eq("crypto_kem_xwing_publickeybytes", XWING_PK);
    size_eq("crypto_kem_xwing_secretkeybytes", XWING_SK);
    size_eq("crypto_kem_xwing_ciphertextbytes", XWING_CT);
    size_eq("crypto_kem_xwing_sharedsecretbytes", SS);
    size_eq("crypto_kem_xwing_seedbytes", XWING_SEED);
    // configs 7.127 — the generic accessors alias xwing
    size_eq("crypto_kem_publickeybytes", XWING_PK);
    size_eq("crypto_kem_secretkeybytes", XWING_SK);
    size_eq("crypto_kem_ciphertextbytes", XWING_CT);
    size_eq("crypto_kem_sharedsecretbytes", SS);
    size_eq("crypto_kem_seedbytes", XWING_SEED);

    // configs 7.128 — the concatenation arithmetic of the hybrid
    assert_eq!(XWING_CT, MLKEM_CT + 32);
    assert_eq!(XWING_PK, MLKEM_PK + 32);

    // primitive string
    let (c, r) = both::<StrFn>("crypto_kem_primitive");
    let (pc, pr) = unsafe { (CStr::from_ptr((c)()), CStr::from_ptr((r)())) };
    assert_eq!(pc.to_bytes(), b"xwing", "C crypto_kem_primitive()");
    assert_eq!(pr.to_bytes(), pc.to_bytes(), "Rust crypto_kem_primitive()");
    // Called twice: must be a pointer into static storage, never NULL.
    let (p1, p2) = unsafe { ((c)(), (r)()) };
    assert!(!p1.is_null() && !p2.is_null());
    assert_eq!(unsafe { (c)() }, p1, "C primitive pointer not stable");
    assert_eq!(unsafe { (r)() }, p2, "Rust primitive pointer not stable");
}

// =========================================================================
// ML-KEM-768 keypair — configs 7.113, 7.114; errors 7.116
// =========================================================================

/// `sk = skpv(1152) ‖ pk(1184) ‖ SHA3-256(pk)(32) ‖ z(32)`
#[track_caller]
fn check_mlkem_sk_layout(what: &str, pk: &[u8], sk: &[u8], seed: Option<&[u8]>) {
    assert_eq!(pk.len(), MLKEM_PK);
    assert_eq!(sk.len(), MLKEM_SK);
    eqb(
        &format!("{what}: sk[1152..2336] == pk"),
        pk,
        &sk[POLYVECBYTES..POLYVECBYTES + MLKEM_PK],
    );
    eqb(
        &format!("{what}: sk[2336..2368] == SHA3-256(pk)"),
        &c_sha3_256(pk),
        &sk[POLYVECBYTES + MLKEM_PK..POLYVECBYTES + MLKEM_PK + 32],
    );
    if let Some(seed) = seed {
        eqb(
            &format!("{what}: sk[2368..2400] == z == seed[32..64]"),
            &seed[32..64],
            &sk[MLKEM_SK - 32..],
        );
    }
}

#[test]
fn mlkem768_seed_keypair_structure() {
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; MLKEM_SEED],
        vec![0xffu8; MLKEM_SEED],
        (0..MLKEM_SEED as u8).collect(),
    ];
    let mut rng = Rng::new(0x7113);
    for _ in 0..24 {
        seeds.push(rng.bytes(MLKEM_SEED));
    }
    for seed in &seeds {
        let (rc, pk, sk) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, seed);
        eqi("mlkem768_seed_keypair rc", rc, 0);
        assert_eq!(rc, 0, "errors 7.116: _seed_keypair can never fail");
        check_mlkem_sk_layout("mlkem768_seed_keypair", &pk, &sk, Some(seed));
        // Deterministic: a second call with the same seed reproduces both halves.
        let (rc2, pk2, sk2) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, seed);
        eqi("rc stability", rc, rc2);
        eqb("pk stability", &pk, &pk2);
        eqb("sk stability", &sk, &sk2);
    }
    // Distinct seeds give distinct public keys.
    let (_, pk_a, _) = d_seed_keypair(
        "crypto_kem_mlkem768_seed_keypair",
        MLKEM_PK,
        MLKEM_SK,
        &vec![0u8; MLKEM_SEED],
    );
    let (_, pk_b, _) = d_seed_keypair(
        "crypto_kem_mlkem768_seed_keypair",
        MLKEM_PK,
        MLKEM_SK,
        &vec![0xffu8; MLKEM_SEED],
    );
    assert_ne!(pk_a, pk_b);
}

#[test]
fn mlkem768_seed_keypair_seed_halves() {
    // errors 7.126 / configs 7.113: seed[0..32] drives indcpa_keypair,
    // seed[32..64] is copied verbatim into sk as `z` and does NOT affect pk.
    let mut rng = Rng::new(0x7113_0002);
    for _ in 0..12 {
        let mut s1 = rng.bytes(MLKEM_SEED);
        let mut s2 = s1.clone();
        rng.fill(&mut s2[32..]);
        if s1[32..] == s2[32..] {
            s2[32] ^= 1;
        }
        let (_, pk1, sk1) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &s1);
        let (_, pk2, sk2) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &s2);
        eqb("pk independent of seed[32..64]", &pk1, &pk2);
        eqb("sk[0..2368] independent of seed[32..64]", &sk1[..MLKEM_SK - 32], &sk2[..MLKEM_SK - 32]);
        assert_ne!(&sk1[MLKEM_SK - 32..], &sk2[MLKEM_SK - 32..], "z must differ");

        // And a change in seed[0..32] does change pk.
        rng.fill(&mut s1[..32]);
        let (_, pk3, _) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &s1);
        assert_ne!(pk1, pk3, "pk must depend on seed[0..32]");
    }
}

#[test]
fn mlkem768_keypair_random() {
    for i in 0..24u64 {
        let (rc, pk, sk) = d_keypair(
            "crypto_kem_mlkem768_keypair",
            MLKEM_PK,
            MLKEM_SK,
            0x7114_0000 + i,
        );
        assert_eq!(rc, 0);
        check_mlkem_sk_layout("mlkem768_keypair", &pk, &sk, None);
        // The 64 random bytes are the seed, so _seed_keypair over the recovered
        // `z` half cannot be reconstructed — but the round trip must still work.
        let mut rng = Rng::new(0x7114_1000 + i);
        let s = rng.bytes(MLKEM_ENC_SEED);
        let (rce, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &s);
        assert_eq!(rce, 0);
        let (rcd, ss2) = d_dec("crypto_kem_mlkem768_dec", &ct, &sk);
        assert_eq!(rcd, 0);
        eqb("mlkem768 keypair round trip", &ss, &ss2);
    }
    // rng_reset() puts both streams at the default seed; the same keypair must
    // come out of both libraries and be reproducible across resets.
    rng_reset();
    let (c, r) = both::<Kp>("crypto_kem_mlkem768_keypair");
    let (mut pkc, mut pkr) = (prefilled(MLKEM_PK), prefilled(MLKEM_PK));
    let (mut skc, mut skr) = (prefilled(MLKEM_SK), prefilled(MLKEM_SK));
    assert_eq!(unsafe { (c)(pkc.as_mut_ptr(), skc.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { (r)(pkr.as_mut_ptr(), skr.as_mut_ptr()) }, 0);
    eqb("rng_reset keypair pk", &pkc, &pkr);
    eqb("rng_reset keypair sk", &skc, &skr);
    check_pad("rng_reset keypair pk(C)", &pkc, MLKEM_PK);
    check_pad("rng_reset keypair sk(C)", &skc, MLKEM_SK);
}

// =========================================================================
// ML-KEM-768 encapsulation — configs 7.115, 7.116, 7.118; errors 7.110–7.112
// =========================================================================

fn mlkem_fixed_keypair() -> (Vec<u8>, Vec<u8>) {
    let seed: Vec<u8> = (0..MLKEM_SEED as u8).map(|i| i.wrapping_mul(3).wrapping_add(7)).collect();
    let (rc, pk, sk) = d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &seed);
    assert_eq!(rc, 0);
    (pk, sk)
}

#[test]
fn mlkem768_enc_deterministic_fixed() {
    let (pk, _sk) = mlkem_fixed_keypair();
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; MLKEM_ENC_SEED],
        vec![0xffu8; MLKEM_ENC_SEED],
        (0..MLKEM_ENC_SEED as u8).collect(),
    ];
    let mut rng = Rng::new(0x7115);
    for _ in 0..24 {
        seeds.push(rng.bytes(MLKEM_ENC_SEED));
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for s in &seeds {
        let (rc, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, s);
        assert_eq!(rc, 0, "canonical pk must be accepted");
        // deterministic
        let (rc2, ct2, ss2) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, s);
        eqi("rc stability", rc, rc2);
        eqb("ct stability", &ct, &ct2);
        eqb("ss stability", &ss, &ss2);
        assert!(!seen.contains(&ct), "distinct seeds must give distinct ct");
        seen.push(ct);
        assert_ne!(ss, vec![0u8; SS]);
    }
}

#[test]
fn mlkem768_enc_deterministic_reads_only_32_seed_bytes() {
    // errors 7.126: `crypto_kem_mlkem768_SEEDBYTES == 64` but
    // `_enc_deterministic` consumes exactly 32 bytes.
    let (pk, _sk) = mlkem_fixed_keypair();
    let mut rng = Rng::new(0x7126_0001);
    for _ in 0..12 {
        let mut long = rng.bytes(64);
        let (rc1, ct1, ss1) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &long);
        rng.fill(&mut long[32..]);
        let (rc2, ct2, ss2) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &long);
        eqi("rc", rc1, rc2);
        eqb("ct independent of seed[32..64]", &ct1, &ct2);
        eqb("ss independent of seed[32..64]", &ss1, &ss2);
        // whereas seed[0..32] is load-bearing
        long[0] ^= 0x01;
        let (_, ct3, _) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &long);
        assert_ne!(ct1, ct3);
    }
}

#[test]
fn mlkem768_enc_random_and_roundtrip() {
    // configs 7.116 — randomised round trip.
    for i in 0..100u64 {
        let mut rng = Rng::new(0x7116_0000 + i);
        let kseed = rng.bytes(MLKEM_SEED);
        let (_, pk, sk) =
            d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &kseed);
        let (rce, ct, ss) = d_enc("crypto_kem_mlkem768_enc", MLKEM_CT, &pk, 0x9000_0000 + i);
        assert_eq!(rce, 0);
        let (rcd, ssd) = d_dec("crypto_kem_mlkem768_dec", &ct, &sk);
        assert_eq!(rcd, 0);
        eqb("mlkem768 _enc/_dec round trip", &ss, &ssd);
    }
}

#[test]
fn mlkem768_interop_both_directions() {
    // A C-generated keypair/ciphertext decapsulated by Rust and vice versa.
    let (ckp, rkp) = both::<SKp>("crypto_kem_mlkem768_seed_keypair");
    let (cenc, renc) = both::<EncD>("crypto_kem_mlkem768_enc_deterministic");
    let (cdec, rdec) = both::<Dec>("crypto_kem_mlkem768_dec");
    let mut rng = Rng::new(0x7116_1234);
    for _ in 0..16 {
        let kseed = rng.bytes(MLKEM_SEED);
        let eseed = rng.bytes(MLKEM_ENC_SEED);

        // keypair from C, encapsulate with Rust, decapsulate with C ... and the
        // mirror image; all four (keygen, encap, decap) library choices.
        for (kp, enc, dec, tag) in [
            (&ckp, &renc, &cdec, "C-kp/Rust-enc/C-dec"),
            (&rkp, &cenc, &rdec, "Rust-kp/C-enc/Rust-dec"),
            (&ckp, &cenc, &rdec, "C-kp/C-enc/Rust-dec"),
            (&rkp, &renc, &cdec, "Rust-kp/Rust-enc/C-dec"),
        ] {
            let mut pk = prefilled(MLKEM_PK);
            let mut sk = prefilled(MLKEM_SK);
            assert_eq!(
                unsafe { (kp)(pk.as_mut_ptr(), sk.as_mut_ptr(), kseed.as_ptr()) },
                0
            );
            let mut ct = prefilled(MLKEM_CT);
            let mut ss1 = prefilled(SS);
            assert_eq!(
                unsafe {
                    (enc)(ct.as_mut_ptr(), ss1.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr())
                },
                0
            );
            let mut ss2 = prefilled(SS);
            assert_eq!(
                unsafe { (dec)(ss2.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) },
                0
            );
            eqb(&format!("mlkem768 interop {tag}"), &ss1[..SS], &ss2[..SS]);
            check_pad(&format!("{tag} ct"), &ct, MLKEM_CT);
            check_pad(&format!("{tag} ss"), &ss2, SS);
        }
    }
}

#[test]
fn mlkem768_enc_zero_and_varied_publicseed() {
    // configs 7.118 / errors 7.112: the all-zero pk is canonical, so it is
    // ACCEPTED, and the unvalidated publicseed tail changes the ciphertext.
    let seed: Vec<u8> = (0..MLKEM_ENC_SEED as u8).collect();

    let zero_pk = vec![0u8; MLKEM_PK];
    let (rc, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &zero_pk, &seed);
    assert_eq!(rc, 0, "errors 7.112: all-zero pk must be accepted");
    assert_ne!(ct, vec![0u8; MLKEM_CT]);
    assert_ne!(ss, vec![0u8; SS]);
    let (rcr, _ctr, _ssr) = d_enc("crypto_kem_mlkem768_enc", MLKEM_CT, &zero_pk, 0x7118_0001);
    assert_eq!(rcr, 0);

    // Vary only pk[1152..1184] (the publicseed): still accepted, different ct.
    let (pk, _sk) = mlkem_fixed_keypair();
    let (_, base_ct, base_ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &seed);
    let mut rng = Rng::new(0x7118);
    let mut seen = vec![base_ct.clone()];
    for _ in 0..16 {
        let mut pk2 = pk.clone();
        rng.fill(&mut pk2[POLYVECBYTES..]);
        let (rc2, ct2, ss2) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk2, &seed);
        assert_eq!(rc2, 0, "publicseed is not validated");
        assert!(!seen.contains(&ct2), "publicseed must change ct");
        assert_ne!(ss2, base_ss, "publicseed is hashed into ss via H(pk)");
        seen.push(ct2);
    }
    // A publicseed of all-0xff is also fine (only pk[0..1152] is validated).
    let mut pk3 = pk.clone();
    for b in pk3[POLYVECBYTES..].iter_mut() {
        *b = 0xff;
    }
    let (rc3, _, _) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk3, &seed);
    assert_eq!(rc3, 0);
}

#[test]
fn mlkem768_enc_noncanonical_pk_rejected() {
    // errors 7.110 / 7.111 — the only real validity check in the whole KEM area.
    let (pk, _sk) = mlkem_fixed_keypair();
    let seed: Vec<u8> = (0..MLKEM_ENC_SEED as u8).map(|i| i ^ 0x5a).collect();
    let (c, r) = both::<EncD>("crypto_kem_mlkem768_enc_deterministic");

    for (poly, coeff) in [
        (0usize, 0usize),
        (0, 1),
        (0, 2),
        (0, 127),
        (0, 254),
        (0, 255),
        (1, 0),
        (1, 7),
        (1, 255),
        (2, 0),
        (2, 128),
        (2, 254),
        (2, 255),
    ] {
        for v in [3329u16, 3330, 4095] {
            let mut bad = pk.clone();
            set_coeff(&mut bad, poly, coeff, v);
            let tag = format!("noncanonical pk poly={poly} coeff={coeff} v={v}");
            let (mut ctc, mut ctr) = (prefilled(MLKEM_CT), prefilled(MLKEM_CT));
            let (mut ssc, mut ssr) = (prefilled(SS), prefilled(SS));
            let rcc = unsafe {
                (c)(ctc.as_mut_ptr(), ssc.as_mut_ptr(), bad.as_ptr(), seed.as_ptr())
            };
            let rcr = unsafe {
                (r)(ctr.as_mut_ptr(), ssr.as_mut_ptr(), bad.as_ptr(), seed.as_ptr())
            };
            eqi(&format!("{tag} rc"), rcc, rcr);
            assert_eq!(rcc, -1, "{tag}: expected -1 from polyvec_is_canonical");
            // ct and ss must be completely untouched.
            eqb(&format!("{tag} ct"), &ctc, &ctr);
            eqb(&format!("{tag} ss"), &ssc, &ssr);
            eqb(&format!("{tag} ct untouched"), &ctc[..MLKEM_CT], &pat(MLKEM_CT));
            eqb(&format!("{tag} ss untouched"), &ssc[..SS], &pat(SS));
            check_pad(&format!("{tag} ct pad"), &ctc, MLKEM_CT);
            check_pad(&format!("{tag} ss pad"), &ssr, SS);
        }
        // 3328 is the largest canonical value: must still be accepted.
        let mut ok = pk.clone();
        set_coeff(&mut ok, poly, coeff, 3328);
        let (rc, _, _) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &ok, &seed);
        assert_eq!(rc, 0, "coefficient 3328 is canonical (poly={poly} coeff={coeff})");
    }

    // all-0xff pk: every coefficient is 0xFFF = 4095 >= 3329.
    let ff = vec![0xffu8; MLKEM_PK];
    let (rc, _, _) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &ff, &seed);
    assert_eq!(rc, -1);

    // errors 7.111: the same rejection propagates through the randomised _enc,
    // which draws (and then zeroes) 32 random bytes first.
    let mut bad = pk.clone();
    set_coeff(&mut bad, 1, 3, 3329);
    let (rce, cte, sse) = d_enc("crypto_kem_mlkem768_enc", MLKEM_CT, &bad, 0x7111);
    assert_eq!(rce, -1);
    eqb("7.111 ct untouched", &cte, &pat(MLKEM_CT));
    eqb("7.111 ss untouched", &sse, &pat(SS));
    let (rcf, _, _) = d_enc("crypto_kem_mlkem768_enc", MLKEM_CT, &ff, 0x7111_0002);
    assert_eq!(rcf, -1);
}

// =========================================================================
// ML-KEM-768 decapsulation / implicit rejection — configs 7.117;
// errors 7.113, 7.114, 7.115
// =========================================================================

/// `k_bar = SHAKE256(z ‖ ct)[0..32]`, the implicit-rejection secret.
fn implicit_secret(sk: &[u8], ct: &[u8]) -> Vec<u8> {
    let z = &sk[MLKEM_SK - 32..];
    let mut inp = Vec::with_capacity(32 + ct.len());
    inp.extend_from_slice(z);
    inp.extend_from_slice(ct);
    c_shake256(SS, &inp)
}

#[test]
fn mlkem768_dec_implicit_rejection_bitflips() {
    // configs 7.117 / errors 7.113: NEVER -1; the shared secret becomes
    // SHAKE256(z ‖ ct) and both libraries must produce the SAME wrong secret.
    let (pk, sk) = mlkem_fixed_keypair();
    let eseed: Vec<u8> = (0..MLKEM_ENC_SEED as u8).map(|i| i.wrapping_mul(5)).collect();
    let (rc, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &eseed);
    assert_eq!(rc, 0);
    let (rcd, ssd) = d_dec("crypto_kem_mlkem768_dec", &ct, &sk);
    assert_eq!(rcd, 0);
    eqb("clean round trip", &ss, &ssd);

    // Byte positions across every structural region of the 1088-byte ct:
    // b (3 x 320 bytes, du-compressed) then v (128 bytes, dv-compressed).
    let positions: Vec<usize> = vec![
        0, 1, 2, 3, 4, 5, 159, 319, 320, 321, 479, 639, 640, 641, 799, 958, 959, 960, 961, 1000,
        1023, 1024, 1063, 1085, 1086, 1087,
    ];
    for (i, &p) in positions.iter().enumerate() {
        let bit = 1u8 << (i % 8);
        let mut bad = ct.clone();
        bad[p] ^= bit;
        let tag = format!("mlkem768_dec ct[{p}] ^= {bit:#04x}");
        let (rcb, ssb) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
        assert_eq!(rcb, 0, "{tag}: mlkem768_dec must NEVER return -1");
        assert_ne!(ssb, ss, "{tag}: implicit rejection must change ss");
        eqb(&format!("{tag} == SHAKE256(z‖ct)"), &implicit_secret(&sk, &bad), &ssb);
        // reproducible
        let (_, ssb2) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
        eqb(&format!("{tag} reproducible"), &ssb, &ssb2);
    }

    // Several bit positions inside one byte, at both ends of the ciphertext.
    for &p in &[0usize, 1087] {
        for b in 0..8u32 {
            let mut bad = ct.clone();
            bad[p] ^= 1u8 << b;
            let (rcb, ssb) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
            assert_eq!(rcb, 0);
            assert_ne!(ssb, ss);
            eqb(
                &format!("mlkem768_dec ct[{p}] bit {b}"),
                &implicit_secret(&sk, &bad),
                &ssb,
            );
        }
    }
}

#[test]
fn mlkem768_dec_extreme_ciphertexts() {
    // errors 7.113: all-zero / all-0xff / fully random ct — always 0.
    let (pk, sk) = mlkem_fixed_keypair();
    let eseed = vec![0x11u8; MLKEM_ENC_SEED];
    let (_, good_ct, good_ss) =
        d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &eseed);

    let mut cts: Vec<Vec<u8>> = vec![vec![0u8; MLKEM_CT], vec![0xffu8; MLKEM_CT]];
    let mut rng = Rng::new(0x7113_0003);
    for _ in 0..24 {
        cts.push(rng.bytes(MLKEM_CT));
    }
    for ct in &cts {
        let (rc, ss) = d_dec("crypto_kem_mlkem768_dec", ct, &sk);
        assert_eq!(rc, 0, "no ciphertext validity check exists");
        assert_ne!(ss, good_ss);
        eqb("extreme ct == SHAKE256(z‖ct)", &implicit_secret(&sk, ct), &ss);
    }
    // sanity: the untouched ciphertext still round-trips
    let (rc, ss) = d_dec("crypto_kem_mlkem768_dec", &good_ct, &sk);
    assert_eq!(rc, 0);
    eqb("good ct still ok", &good_ss, &ss);
}

#[test]
fn mlkem768_dec_wrong_and_corrupt_sk() {
    // errors 7.114 / 7.115
    let (pk_a, sk_a) = mlkem_fixed_keypair();
    let bseed: Vec<u8> = (0..MLKEM_SEED as u8).map(|i| i ^ 0xa5).collect();
    let (_, _pk_b, sk_b) =
        d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, &bseed);
    let eseed = vec![0x42u8; MLKEM_ENC_SEED];
    let (_, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk_a, &eseed);

    // 7.114 — foreign secret key
    let (rc, ssb) = d_dec("crypto_kem_mlkem768_dec", &ct, &sk_b);
    assert_eq!(rc, 0);
    assert_ne!(ssb, ss);
    eqb("foreign sk == SHAKE256(z_b‖ct)", &implicit_secret(&sk_b, &ct), &ssb);

    // 7.115 — structurally corrupted sk.  Note `z` (sk[2368..2400]) is only
    // consulted on the implicit-rejection path, so corrupting it does NOT
    // change the result for a *valid* ciphertext; it is exercised separately
    // below with a corrupted ciphertext.
    for (off, tag) in [
        (0usize, "polyvec"),
        (POLYVECBYTES, "embedded pk"),
        (POLYVECBYTES + MLKEM_PK, "hpk"),
    ] {
        let mut bad = sk_a.clone();
        bad[off] ^= 0x01;
        let (rcc, ssc) = d_dec("crypto_kem_mlkem768_dec", &ct, &bad);
        assert_eq!(rcc, 0, "{tag}: no sk consistency check");
        assert_ne!(ssc, ss, "{tag}: must land on the implicit-rejection path");
        eqb(
            &format!("corrupt sk ({tag}) == SHAKE256(z‖ct)"),
            &implicit_secret(&bad, &ct),
            &ssc,
        );
    }
    // `z` is a pure no-op for a valid ciphertext …
    let mut zbad = sk_a.clone();
    zbad[MLKEM_SK - 32] ^= 0x01;
    let (rcz, ssz) = d_dec("crypto_kem_mlkem768_dec", &ct, &zbad);
    assert_eq!(rcz, 0);
    eqb("corrupt z is a no-op on the success path", &ss, &ssz);
    // … but fully determines the shared secret once the ciphertext is corrupted.
    let mut badct = ct.clone();
    badct[17] ^= 0x20;
    let (rc1, s1) = d_dec("crypto_kem_mlkem768_dec", &badct, &sk_a);
    let (rc2, s2) = d_dec("crypto_kem_mlkem768_dec", &badct, &zbad);
    assert_eq!((rc1, rc2), (0, 0));
    assert_ne!(s1, s2, "z drives the implicit-rejection secret");
    eqb("z=orig", &implicit_secret(&sk_a, &badct), &s1);
    eqb("z=flipped", &implicit_secret(&zbad, &badct), &s2);

    // all-zero and all-0xff secret keys: still 0, no validation whatsoever
    for sk in [vec![0u8; MLKEM_SK], vec![0xffu8; MLKEM_SK]] {
        let (rc, ss2) = d_dec("crypto_kem_mlkem768_dec", &ct, &sk);
        assert_eq!(rc, 0);
        eqb("degenerate sk == SHAKE256(z‖ct)", &implicit_secret(&sk, &ct), &ss2);
    }
}

// =========================================================================
// X-Wing keypair — configs 7.120, 7.121; errors 7.124
// =========================================================================

/// Recompute `expand_decaps_key(seed)` with independent C primitives and check
/// that `pk` / `sk` match the X-Wing layout.
#[track_caller]
fn check_xwing_keypair(what: &str, seed: &[u8], pk: &[u8], sk: &[u8]) {
    assert_eq!(seed.len(), XWING_SEED);
    // `sk` is *exactly* the 32-byte seed.
    eqb(&format!("{what}: sk == seed"), seed, sk);

    let expanded = c_shake256(96, seed);
    let mlkem_seed = &expanded[..64];
    let sk_x = &expanded[64..96];

    let (rc, pk_mlkem, _sk_mlkem) =
        d_seed_keypair("crypto_kem_mlkem768_seed_keypair", MLKEM_PK, MLKEM_SK, mlkem_seed);
    assert_eq!(rc, 0);
    eqb(&format!("{what}: pk[0..1184] == ML-KEM pk"), &pk_mlkem, &pk[..MLKEM_PK]);
    eqb(
        &format!("{what}: pk[1184..1216] == X25519 base(sk_x)"),
        &c_x25519_base(sk_x),
        &pk[MLKEM_PK..],
    );
}

#[test]
fn xwing_seed_keypair_structure() {
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; XWING_SEED],
        vec![0xffu8; XWING_SEED],
        (0..XWING_SEED as u8).collect(),
    ];
    let mut rng = Rng::new(0x7120);
    for _ in 0..24 {
        seeds.push(rng.bytes(XWING_SEED));
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for seed in &seeds {
        let (rc, pk, sk) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, seed);
        assert_eq!(rc, 0, "errors 7.124: xwing _seed_keypair can never fail");
        check_xwing_keypair("xwing_seed_keypair", seed, &pk, &sk);
        // deterministic
        let (_, pk2, sk2) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, seed);
        eqb("pk stability", &pk, &pk2);
        eqb("sk stability", &sk, &sk2);
        assert!(!seen.contains(&pk), "distinct seeds must give distinct pk");
        seen.push(pk);
    }
}

#[test]
fn xwing_keypair_random() {
    for i in 0..24u64 {
        let (rc, pk, sk) = d_keypair("crypto_kem_xwing_keypair", XWING_PK, XWING_SK, 0x7121_0000 + i);
        assert_eq!(rc, 0);
        assert_eq!(sk.len(), 32);
        // configs 7.121: re-deriving from `sk` (which *is* the seed) reproduces pk.
        let (rc2, pk2, sk2) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, &sk);
        assert_eq!(rc2, 0);
        eqb("xwing _keypair == _seed_keypair(sk)", &pk, &pk2);
        eqb("xwing sk idempotent", &sk, &sk2);
        check_xwing_keypair("xwing_keypair", &sk, &pk, &sk2);
    }
    rng_reset();
    let (c, r) = both::<Kp>("crypto_kem_xwing_keypair");
    let (mut pkc, mut pkr) = (prefilled(XWING_PK), prefilled(XWING_PK));
    let (mut skc, mut skr) = (prefilled(XWING_SK), prefilled(XWING_SK));
    assert_eq!(unsafe { (c)(pkc.as_mut_ptr(), skc.as_mut_ptr()) }, 0);
    assert_eq!(unsafe { (r)(pkr.as_mut_ptr(), skr.as_mut_ptr()) }, 0);
    eqb("rng_reset xwing pk", &pkc, &pkr);
    eqb("rng_reset xwing sk", &skc, &skr);
    check_pad("rng_reset xwing pk(C)", &pkc, XWING_PK);
    check_pad("rng_reset xwing sk(Rust)", &skr, XWING_SK);
}

// =========================================================================
// X-Wing encapsulation — configs 7.122, 7.123; errors 7.117–7.119, 7.126
// =========================================================================

fn xwing_fixed_keypair() -> (Vec<u8>, Vec<u8>) {
    let seed: Vec<u8> = (0..XWING_SEED as u8).map(|i| i.wrapping_mul(11).wrapping_add(3)).collect();
    let (rc, pk, sk) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, &seed);
    assert_eq!(rc, 0);
    (pk, sk)
}

/// The X-Wing combiner, recomputed from independent C primitives.
fn combiner(ss_mlkem: &[u8], ss_x: &[u8], ct_x: &[u8], pk_x: &[u8]) -> Vec<u8> {
    let mut inp = Vec::new();
    inp.extend_from_slice(ss_mlkem);
    inp.extend_from_slice(ss_x);
    inp.extend_from_slice(ct_x);
    inp.extend_from_slice(pk_x);
    inp.extend_from_slice(&XWING_LABEL);
    c_sha3_256(&inp)
}

#[test]
fn xwing_enc_deterministic_structure() {
    // configs 7.122 + 7.128: ct = ct_mlkem(1088) ‖ ct_x25519(32),
    // ss = SHA3-256(ss_mlkem ‖ ss_x25519 ‖ ct_x25519 ‖ pk_x25519 ‖ label).
    let (pk, _sk) = xwing_fixed_keypair();
    let mut seeds: Vec<Vec<u8>> = vec![
        vec![0u8; XWING_ENC_SEED],
        vec![0xffu8; XWING_ENC_SEED],
        (0..XWING_ENC_SEED as u8).collect(),
    ];
    let mut rng = Rng::new(0x7122);
    for _ in 0..24 {
        seeds.push(rng.bytes(XWING_ENC_SEED));
    }
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for seed in &seeds {
        let (rc, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, seed);
        assert_eq!(rc, 0);

        // ML-KEM half
        let (rcm, ct_mlkem, ss_mlkem) = d_enc_det(
            "crypto_kem_mlkem768_enc_deterministic",
            MLKEM_CT,
            &pk[..MLKEM_PK],
            &seed[..32],
        );
        assert_eq!(rcm, 0);
        eqb("xwing ct[0..1088] == ML-KEM ct", &ct_mlkem, &ct[..MLKEM_CT]);

        // X25519 half
        let ct_x = c_x25519_base(&seed[32..64]);
        eqb("xwing ct[1088..1120] == base(seed[32..64])", &ct_x, &ct[MLKEM_CT..]);
        let (rcs, ss_x) = c_x25519(&seed[32..64], &pk[MLKEM_PK..]);
        assert_eq!(rcs, 0);

        eqb("xwing ss == combiner(...)", &combiner(&ss_mlkem, &ss_x, &ct_x, &pk[MLKEM_PK..]), &ss);

        // deterministic
        let (_, ct2, ss2) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, seed);
        eqb("ct stability", &ct, &ct2);
        eqb("ss stability", &ss, &ss2);
        assert!(!seen.contains(&ct));
        seen.push(ct);
    }
}

#[test]
fn xwing_enc_deterministic_reads_64_seed_bytes() {
    // errors 7.126: crypto_kem_xwing_SEEDBYTES == 32 for _seed_keypair, yet
    // _enc_deterministic consumes 64 bytes — both halves are load-bearing.
    let (pk, _sk) = xwing_fixed_keypair();
    let mut rng = Rng::new(0x7126_0002);
    for _ in 0..12 {
        let base = rng.bytes(XWING_ENC_SEED);
        let (_, ct0, ss0) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &base);

        let mut s1 = base.clone();
        s1[0] ^= 0x01;
        let (_, ct1, ss1) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &s1);
        assert_ne!(ct0[..MLKEM_CT], ct1[..MLKEM_CT], "seed[0..32] drives the ML-KEM half");
        eqb("seed[0..32] does not move ct_x25519", &ct0[MLKEM_CT..], &ct1[MLKEM_CT..]);
        assert_ne!(ss0, ss1);

        // seed[32..64] is the ephemeral X25519 scalar, which is *clamped*
        // (`t[0] &= 248; t[31] &= 127; t[31] |= 64`), so only the non-masked
        // bits are load-bearing.  Bit 3 of seed[32] survives clamping.
        let mut s2 = base.clone();
        s2[32] ^= 0x08;
        let (_, ct2, ss2) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &s2);
        eqb("seed[32..64] does not move ct_mlkem", &ct0[..MLKEM_CT], &ct2[..MLKEM_CT]);
        assert_ne!(ct0[MLKEM_CT..], ct2[MLKEM_CT..], "seed[32..64] is the ephemeral scalar");
        assert_ne!(ss0, ss2);

        // The clamped-away bits (low 3 of seed[32], bits 6/7 of seed[63]) must
        // leave the ciphertext AND the shared secret bit-identical.
        for (idx, mask) in [(32usize, 0x01u8), (32, 0x02), (32, 0x04), (63, 0x40), (63, 0x80)] {
            let mut s3 = base.clone();
            s3[idx] ^= mask;
            let (_, ct3, ss3) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &s3);
            eqb(&format!("clamped bit seed[{idx}]&{mask:#04x}: ct"), &ct0, &ct3);
            eqb(&format!("clamped bit seed[{idx}]&{mask:#04x}: ss"), &ss0, &ss3);
        }
    }
}

#[test]
fn xwing_enc_random_and_roundtrip() {
    // configs 7.123 — randomised round trip through _enc (draws 64 bytes).
    for i in 0..100u64 {
        let mut rng = Rng::new(0x7123_0000 + i);
        let kseed = rng.bytes(XWING_SEED);
        let (_, pk, sk) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, &kseed);
        let (rce, ct, ss) = d_enc("crypto_kem_xwing_enc", XWING_CT, &pk, 0xA000_0000 + i);
        assert_eq!(rce, 0);
        let (rcd, ssd) = d_dec("crypto_kem_xwing_dec", &ct, &sk);
        assert_eq!(rcd, 0);
        eqb("xwing _enc/_dec round trip", &ss, &ssd);

        // and the deterministic entry point over many seeds
        for _ in 0..3 {
            let es = rng.bytes(XWING_ENC_SEED);
            let (rc2, ct2, ss2) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &es);
            assert_eq!(rc2, 0);
            let (rc3, ss3) = d_dec("crypto_kem_xwing_dec", &ct2, &sk);
            assert_eq!(rc3, 0);
            eqb("xwing _enc_deterministic/_dec round trip", &ss2, &ss3);
        }
    }
}

#[test]
fn xwing_roundtrip_degenerate_seeds() {
    // configs 7.125 / errors 7.124: sk = 32 zero bytes and 32 0xff bytes are
    // perfectly legal seeds; the full round trip must work for each.
    for kseed in [vec![0u8; XWING_SEED], vec![0xffu8; XWING_SEED]] {
        let (rc, pk, sk) = d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, &kseed);
        assert_eq!(rc, 0);
        eqb("sk == seed", &kseed, &sk);
        check_xwing_keypair("degenerate xwing keypair", &kseed, &pk, &sk);
        for eseed in [
            vec![0u8; XWING_ENC_SEED],
            vec![0xffu8; XWING_ENC_SEED],
            (0..XWING_ENC_SEED as u8).collect::<Vec<u8>>(),
        ] {
            let (rce, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &eseed);
            assert_eq!(rce, 0);
            let (rcd, ssd) = d_dec("crypto_kem_xwing_dec", &ct, &sk);
            assert_eq!(rcd, 0);
            eqb("degenerate xwing round trip", &ss, &ssd);
        }
        let (rce, ct, ss) = d_enc("crypto_kem_xwing_enc", XWING_CT, &pk, 0x7125_0001);
        assert_eq!(rce, 0);
        let (rcd, ssd) = d_dec("crypto_kem_xwing_dec", &ct, &sk);
        assert_eq!(rcd, 0);
        eqb("degenerate xwing randomised round trip", &ss, &ssd);
    }
}

#[test]
fn xwing_interop_both_directions() {
    let (ckp, rkp) = both::<SKp>("crypto_kem_xwing_seed_keypair");
    let (cenc, renc) = both::<EncD>("crypto_kem_xwing_enc_deterministic");
    let (cdec, rdec) = both::<Dec>("crypto_kem_xwing_dec");
    let mut rng = Rng::new(0x7123_4321);
    for _ in 0..16 {
        let kseed = rng.bytes(XWING_SEED);
        let eseed = rng.bytes(XWING_ENC_SEED);
        for (kp, enc, dec, tag) in [
            (&ckp, &renc, &cdec, "C-kp/Rust-enc/C-dec"),
            (&rkp, &cenc, &rdec, "Rust-kp/C-enc/Rust-dec"),
            (&ckp, &cenc, &rdec, "C-kp/C-enc/Rust-dec"),
            (&rkp, &renc, &cdec, "Rust-kp/Rust-enc/C-dec"),
        ] {
            let mut pk = prefilled(XWING_PK);
            let mut sk = prefilled(XWING_SK);
            assert_eq!(unsafe { (kp)(pk.as_mut_ptr(), sk.as_mut_ptr(), kseed.as_ptr()) }, 0);
            let mut ct = prefilled(XWING_CT);
            let mut ss1 = prefilled(SS);
            assert_eq!(
                unsafe { (enc)(ct.as_mut_ptr(), ss1.as_mut_ptr(), pk.as_ptr(), eseed.as_ptr()) },
                0
            );
            let mut ss2 = prefilled(SS);
            assert_eq!(unsafe { (dec)(ss2.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()) }, 0);
            eqb(&format!("xwing interop {tag}"), &ss1[..SS], &ss2[..SS]);
            check_pad(&format!("{tag} ct"), &ct, XWING_CT);
            check_pad(&format!("{tag} ss"), &ss2, SS);
        }
    }
}

// =========================================================================
// X-Wing failure surface — errors 7.117, 7.118, 7.119, 7.121, 7.122, 7.123
// =========================================================================

#[test]
fn xwing_enc_noncanonical_mlkem_pk() {
    // errors 7.117: pk[0..1152] non-canonical → -1 from the ML-KEM layer,
    // ct/ss untouched.
    let (pk, _sk) = xwing_fixed_keypair();
    let seed: Vec<u8> = (0..XWING_ENC_SEED as u8).map(|i| i ^ 0x33).collect();
    for (poly, coeff, v) in [(0usize, 0usize, 3329u16), (1, 200, 4095), (2, 255, 3400)] {
        let mut bad = pk.clone();
        set_coeff(&mut bad, poly, coeff, v);
        let (rc, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &bad, &seed);
        assert_eq!(rc, -1, "errors 7.117 (poly={poly} coeff={coeff} v={v})");
        eqb("7.117 ct untouched", &ct, &pat(XWING_CT));
        eqb("7.117 ss untouched", &ss, &pat(SS));
        // errors 7.119: the same failure propagates through the randomised _enc
        let (rce, cte, sse) = d_enc("crypto_kem_xwing_enc", XWING_CT, &bad, 0x7119_0000);
        assert_eq!(rce, -1);
        eqb("7.119 ct untouched", &cte, &pat(XWING_CT));
        eqb("7.119 ss untouched", &sse, &pat(SS));
    }
    // whole-pk 0xff: ML-KEM half non-canonical
    let ff = vec![0xffu8; XWING_PK];
    let (rc, _, _) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &ff, &seed);
    assert_eq!(rc, -1);
}

#[test]
fn xwing_enc_small_order_x25519_pk() {
    // errors 7.118: pk[1184..1216] is a blocklisted small-order encoding →
    // crypto_scalarmult_curve25519 fails → -1, ct/ss untouched.
    let (pk, _sk) = xwing_fixed_keypair();
    let seed: Vec<u8> = (0..XWING_ENC_SEED as u8).map(|i| i.wrapping_mul(7)).collect();
    for (i, so) in small_order_encodings().iter().enumerate() {
        let mut bad = pk.clone();
        bad[MLKEM_PK..].copy_from_slice(so);
        let (rc, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &bad, &seed);
        assert_eq!(rc, -1, "errors 7.118 case {i} ({})", hex(so));
        eqb(&format!("7.118 case {i} ct untouched"), &ct, &pat(XWING_CT));
        eqb(&format!("7.118 case {i} ss untouched"), &ss, &pat(SS));
        // errors 7.119 through the randomised _enc
        let (rce, cte, sse) = d_enc("crypto_kem_xwing_enc", XWING_CT, &bad, 0x7119_1000 + i as u64);
        assert_eq!(rce, -1);
        eqb("7.119 ct untouched", &cte, &pat(XWING_CT));
        eqb("7.119 ss untouched", &sse, &pat(SS));
    }
    // Sanity: a non-canonical-but-not-blocklisted pk_x25519 (e.g. p+2) is
    // ACCEPTED — the guard is a blocklist, not a canonicity check.
    let mut ok = pk.clone();
    let mut p2 = [0xffu8; 32];
    p2[0] = 0xef;
    p2[31] = 0x7f;
    ok[MLKEM_PK..].copy_from_slice(&p2);
    let (rc, _, _) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &ok, &seed);
    assert_eq!(rc, 0, "p+2 is not blocklisted");
}

#[test]
fn xwing_dec_small_order_x25519_ct() {
    // errors 7.121 — the ONLY reachable failure of xwing decapsulation.
    let (pk, sk) = xwing_fixed_keypair();
    let eseed: Vec<u8> = (0..XWING_ENC_SEED as u8).map(|i| i.wrapping_mul(13)).collect();
    let (rc, ct, _ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &eseed);
    assert_eq!(rc, 0);

    for (i, so) in small_order_encodings().iter().enumerate() {
        let mut bad = ct.clone();
        bad[MLKEM_CT..].copy_from_slice(so);
        let (rcd, ssd) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        assert_eq!(rcd, -1, "errors 7.121 case {i} ({})", hex(so));
        eqb(&format!("7.121 case {i} ss untouched"), &ssd, &pat(SS));
        // Also through the generic dispatch (errors 7.125).
        let (rcg, ssg) = d_dec("crypto_kem_dec", &bad, &sk);
        assert_eq!(rcg, -1);
        eqb("7.121 generic ss untouched", &ssg, &pat(SS));
    }
    // A non-blocklisted, non-canonical ct_x25519 is accepted (returns 0 with a
    // different ss) — proving the failure above is the blocklist, not parsing.
    let mut ok = ct.clone();
    let mut p2 = [0xffu8; 32];
    p2[0] = 0xef;
    p2[31] = 0x7f;
    ok[MLKEM_CT..].copy_from_slice(&p2);
    let (rc2, _) = d_dec("crypto_kem_xwing_dec", &ok, &sk);
    assert_eq!(rc2, 0, "p+2 in ct[1088..1120] is not blocklisted");
}

#[test]
fn xwing_dec_corrupt_mlkem_half() {
    // errors 7.122 / configs 7.124: tampering with ct[0..1088] gives 0 with a
    // different shared secret (ML-KEM implicit rejection through the combiner);
    // both libraries must agree on the wrong secret.
    let (pk, sk) = xwing_fixed_keypair();
    let eseed: Vec<u8> = (0..XWING_ENC_SEED as u8).map(|i| i ^ 0x5c).collect();
    let (_, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &eseed);
    let (rcd, ssd) = d_dec("crypto_kem_xwing_dec", &ct, &sk);
    assert_eq!(rcd, 0);
    eqb("clean xwing round trip", &ss, &ssd);

    // Rebuild the ML-KEM secret key so the expected value can be recomputed.
    let expanded = c_shake256(96, &sk);
    let (_, _pk_mlkem, sk_mlkem) = d_seed_keypair(
        "crypto_kem_mlkem768_seed_keypair",
        MLKEM_PK,
        MLKEM_SK,
        &expanded[..64],
    );
    let (rcs, ss_x) = c_x25519(&expanded[64..96], &ct[MLKEM_CT..]);
    assert_eq!(rcs, 0);

    let positions: Vec<usize> = vec![
        0, 1, 2, 3, 100, 319, 320, 500, 639, 640, 900, 959, 960, 961, 1000, 1050, 1086, 1087,
    ];
    for (i, &p) in positions.iter().enumerate() {
        let bit = 1u8 << (i % 8);
        let mut bad = ct.clone();
        bad[p] ^= bit;
        let tag = format!("xwing_dec ct[{p}] ^= {bit:#04x}");
        let (rc, ssb) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        assert_eq!(rc, 0, "{tag}: the mlkem768_dec != 0 branch is dead");
        assert_ne!(ssb, ss, "{tag}: ss must differ");
        // Full independent recomputation: k_bar through the combiner.
        let k_bar = implicit_secret(&sk_mlkem, &bad[..MLKEM_CT]);
        eqb(
            &format!("{tag} == combiner(SHAKE256(z‖ct_mlkem), …)"),
            &combiner(&k_bar, &ss_x, &ct[MLKEM_CT..], &pk[MLKEM_PK..]),
            &ssb,
        );
        // reproducible
        let (_, ssb2) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        eqb(&format!("{tag} reproducible"), &ssb, &ssb2);
    }
    // several bit positions inside one byte
    for b in 0..8u32 {
        let mut bad = ct.clone();
        bad[7] ^= 1u8 << b;
        let (rc, ssb) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        assert_eq!(rc, 0);
        assert_ne!(ssb, ss);
    }
    // Wholly zero / wholly 0xff ML-KEM half with an intact X25519 half.
    for fill in [0x00u8, 0xff] {
        let mut bad = ct.clone();
        for x in bad[..MLKEM_CT].iter_mut() {
            *x = fill;
        }
        let (rc, ssb) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        assert_eq!(rc, 0);
        assert_ne!(ssb, ss);
        let k_bar = implicit_secret(&sk_mlkem, &bad[..MLKEM_CT]);
        eqb(
            "xwing_dec degenerate mlkem half",
            &combiner(&k_bar, &ss_x, &ct[MLKEM_CT..], &pk[MLKEM_PK..]),
            &ssb,
        );
    }
}

#[test]
fn xwing_dec_wrong_sk() {
    // errors 7.123
    let (pk_a, sk_a) = xwing_fixed_keypair();
    let eseed = vec![0x77u8; XWING_ENC_SEED];
    let (_, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk_a, &eseed);
    let (rc, ssd) = d_dec("crypto_kem_xwing_dec", &ct, &sk_a);
    assert_eq!(rc, 0);
    eqb("baseline", &ss, &ssd);

    let mut rng = Rng::new(0x7123_0009);
    for _ in 0..24 {
        let mut sk_b = sk_a.clone();
        rng.fill(&mut sk_b);
        let (rcb, ssb) = d_dec("crypto_kem_xwing_dec", &ct, &sk_b);
        assert_eq!(rcb, 0, "a foreign 32-byte seed is still a valid sk");
        assert_ne!(ssb, ss);
    }
    // single-bit corruption of the seed
    for b in 0..8u32 {
        let mut sk_b = sk_a.clone();
        sk_b[0] ^= 1u8 << b;
        let (rcb, ssb) = d_dec("crypto_kem_xwing_dec", &ct, &sk_b);
        assert_eq!(rcb, 0);
        assert_ne!(ssb, ss);
    }
    // all-zero / all-0xff foreign sk
    for sk_b in [vec![0u8; XWING_SK], vec![0xffu8; XWING_SK]] {
        let (rcb, ssb) = d_dec("crypto_kem_xwing_dec", &ct, &sk_b);
        assert_eq!(rcb, 0);
        assert_ne!(ssb, ss);
    }
}

#[test]
fn xwing_dec_random_ciphertexts() {
    // A wholly random ct is overwhelmingly likely to have a non-blocklisted
    // X25519 half, hence 0 with a pseudorandom ss; both libraries must agree.
    let (_pk, sk) = xwing_fixed_keypair();
    let mut rng = Rng::new(0x7122_0077);
    let mut seen: Vec<Vec<u8>> = Vec::new();
    for _ in 0..24 {
        let ct = rng.bytes(XWING_CT);
        let (rc, ss) = d_dec("crypto_kem_xwing_dec", &ct, &sk);
        assert_eq!(rc, 0);
        assert!(!seen.contains(&ss));
        seen.push(ss);
    }
    // all-0xff ct: the X25519 half is ff…ff, which is NOT blocklisted
    // (`ff…ff` masks to `ff…7f` ≠ `ec/ed/ee ff…7f`), so this succeeds.
    let ff = vec![0xffu8; XWING_CT];
    let (rc, _ss) = d_dec("crypto_kem_xwing_dec", &ff, &sk);
    assert_eq!(rc, 0);
    // all-zero ct: the X25519 half is 32 zero bytes → blocklisted → -1.
    let zero = vec![0u8; XWING_CT];
    let (rcz, ssz) = d_dec("crypto_kem_xwing_dec", &zero, &sk);
    assert_eq!(rcz, -1);
    eqb("all-zero ct leaves ss untouched", &ssz, &pat(SS));
}

// =========================================================================
// generic dispatch — configs 7.127; errors 7.125
// =========================================================================

#[test]
fn generic_dispatch_matches_xwing() {
    let mut rng = Rng::new(0x7127);
    for i in 0..16u64 {
        let kseed = rng.bytes(XWING_SEED);
        let (rcg, pkg, skg) = d_seed_keypair("crypto_kem_seed_keypair", XWING_PK, XWING_SK, &kseed);
        let (rcx, pkx, skx) =
            d_seed_keypair("crypto_kem_xwing_seed_keypair", XWING_PK, XWING_SK, &kseed);
        eqi("seed_keypair rc", rcg, rcx);
        eqb("generic _seed_keypair pk", &pkx, &pkg);
        eqb("generic _seed_keypair sk", &skx, &skg);

        // Randomised keypair: identical RNG stream ⇒ identical output.
        let (rckg, pkkg, skkg) =
            d_keypair("crypto_kem_keypair", XWING_PK, XWING_SK, 0x7127_0000 + i);
        let (rckx, pkkx, skkx) =
            d_keypair("crypto_kem_xwing_keypair", XWING_PK, XWING_SK, 0x7127_0000 + i);
        eqi("keypair rc", rckg, rckx);
        eqb("generic _keypair pk", &pkkx, &pkkg);
        eqb("generic _keypair sk", &skkx, &skkg);

        // enc / enc_deterministic / dec
        let eseed = rng.bytes(XWING_ENC_SEED);
        let (rceg, ctg, ssg) = d_enc("crypto_kem_enc", XWING_CT, &pkg, 0x7127_1000 + i);
        let (rcex, ctx, ssx) = d_enc("crypto_kem_xwing_enc", XWING_CT, &pkg, 0x7127_1000 + i);
        eqi("enc rc", rceg, rcex);
        eqb("generic _enc ct", &ctx, &ctg);
        eqb("generic _enc ss", &ssx, &ssg);

        let (rcdg, ssdg) = d_dec("crypto_kem_dec", &ctg, &skg);
        let (rcdx, ssdx) = d_dec("crypto_kem_xwing_dec", &ctg, &skg);
        eqi("dec rc", rcdg, rcdx);
        eqb("generic _dec ss", &ssdx, &ssdg);
        eqb("generic round trip", &ssg, &ssdg);

        let (_, ct2, ss2) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pkg, &eseed);
        let (rc3, ss3) = d_dec("crypto_kem_dec", &ct2, &skg);
        assert_eq!(rc3, 0);
        eqb("generic _dec of deterministic ct", &ss2, &ss3);
    }

    // errors 7.125: the failure surface is shared too.
    let (pk, _sk) = xwing_fixed_keypair();
    let seed = vec![0x21u8; XWING_ENC_SEED];
    let mut bad = pk.clone();
    bad[MLKEM_PK..].copy_from_slice(&[0u8; 32]); // small-order pk_x25519
    let (rc, ct, ss) = d_enc("crypto_kem_enc", XWING_CT, &bad, 0x7125_2000);
    assert_eq!(rc, -1);
    eqb("generic _enc ct untouched", &ct, &pat(XWING_CT));
    eqb("generic _enc ss untouched", &ss, &pat(SS));
    let mut bad2 = pk.clone();
    set_coeff(&mut bad2, 0, 5, 4095);
    let (rc2, _, _) = d_enc("crypto_kem_enc", XWING_CT, &bad2, 0x7125_2001);
    assert_eq!(rc2, -1);
    // and the deterministic xwing entry point through a generic-sized buffer
    let (rc3, _, _) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &bad2, &seed);
    assert_eq!(rc3, -1);
}

// =========================================================================
// symbol-surface parity
// =========================================================================

#[test]
fn kem_symbol_surface() {
    // Everything the C library exports under crypto_kem_* must exist in the
    // Rust cdylib as well (and nothing here is gated away).
    for s in [
        "crypto_kem_publickeybytes",
        "crypto_kem_secretkeybytes",
        "crypto_kem_ciphertextbytes",
        "crypto_kem_sharedsecretbytes",
        "crypto_kem_seedbytes",
        "crypto_kem_primitive",
        "crypto_kem_seed_keypair",
        "crypto_kem_keypair",
        "crypto_kem_enc",
        "crypto_kem_dec",
        "crypto_kem_mlkem768_publickeybytes",
        "crypto_kem_mlkem768_secretkeybytes",
        "crypto_kem_mlkem768_ciphertextbytes",
        "crypto_kem_mlkem768_sharedsecretbytes",
        "crypto_kem_mlkem768_seedbytes",
        "crypto_kem_mlkem768_seed_keypair",
        "crypto_kem_mlkem768_keypair",
        "crypto_kem_mlkem768_enc",
        "crypto_kem_mlkem768_enc_deterministic",
        "crypto_kem_mlkem768_dec",
        "crypto_kem_xwing_publickeybytes",
        "crypto_kem_xwing_secretkeybytes",
        "crypto_kem_xwing_ciphertextbytes",
        "crypto_kem_xwing_sharedsecretbytes",
        "crypto_kem_xwing_seedbytes",
        "crypto_kem_xwing_seed_keypair",
        "crypto_kem_xwing_keypair",
        "crypto_kem_xwing_enc",
        "crypto_kem_xwing_enc_deterministic",
        "crypto_kem_xwing_dec",
    ] {
        assert!(has(s), "symbol `{s}` missing from one of the two libraries");
    }
    // There is no generic `crypto_kem_enc_deterministic` in libsodium 1.0.23,
    // and neither library may invent one.
    assert!(!has("crypto_kem_enc_deterministic"));
}

// =========================================================================
// dense corruption sweeps
// =========================================================================

#[test]
fn mlkem768_dec_dense_ciphertext_sweep() {
    // errors 7.113 at scale: every 7th byte of the 1088-byte ciphertext, with a
    // rotating bit and with whole-byte substitutions.  `rc` must stay 0 and the
    // two libraries must land on the SAME implicitly-rejected secret.
    let (pk, sk) = mlkem_fixed_keypair();
    let eseed: Vec<u8> = (0..MLKEM_ENC_SEED as u8).map(|i| i ^ 0x9c).collect();
    let (_, ct, ss) = d_enc_det("crypto_kem_mlkem768_enc_deterministic", MLKEM_CT, &pk, &eseed);

    for (i, p) in (0..MLKEM_CT).step_by(7).enumerate() {
        let mut bad = ct.clone();
        bad[p] ^= 1u8 << (i % 8);
        let (rc, ssb) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
        assert_eq!(rc, 0, "ct[{p}]: mlkem768_dec must never fail");
        assert_ne!(ssb, ss, "ct[{p}]: ss must change");
        eqb(&format!("ct[{p}] flip == SHAKE256(z‖ct)"), &implicit_secret(&sk, &bad), &ssb);
    }
    for (i, p) in (0..MLKEM_CT).step_by(53).enumerate() {
        for fill in [0x00u8, 0xff, 0x5a] {
            let mut bad = ct.clone();
            if bad[p] == fill {
                continue;
            }
            bad[p] = fill;
            let (rc, ssb) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
            assert_eq!(rc, 0, "ct[{p}]={fill:#04x} (case {i})");
            eqb(
                &format!("ct[{p}]={fill:#04x} == SHAKE256(z‖ct)"),
                &implicit_secret(&sk, &bad),
                &ssb,
            );
        }
    }
    // Truncation-like corruption: zero out whole regions of the ciphertext.
    for (lo, hi) in [(0usize, 320usize), (320, 640), (640, 960), (960, 1088), (0, 1088)] {
        let mut bad = ct.clone();
        for x in bad[lo..hi].iter_mut() {
            *x = 0;
        }
        let (rc, ssb) = d_dec("crypto_kem_mlkem768_dec", &bad, &sk);
        assert_eq!(rc, 0, "zeroed ct[{lo}..{hi}]");
        eqb(
            &format!("zeroed ct[{lo}..{hi}] == SHAKE256(z‖ct)"),
            &implicit_secret(&sk, &bad),
            &ssb,
        );
    }
}

#[test]
fn xwing_dec_dense_ciphertext_sweep() {
    // ct[0..1088] tampering → 0 with a different ss (errors 7.122);
    // ct[1088..1120] tampering → 0 unless the result is blocklisted
    // (errors 7.121).  Both libraries must always agree.
    let (pk, sk) = xwing_fixed_keypair();
    let eseed: Vec<u8> = (0..XWING_ENC_SEED as u8).map(|i| i ^ 0x2f).collect();
    let (_, ct, ss) = d_enc_det("crypto_kem_xwing_enc_deterministic", XWING_CT, &pk, &eseed);

    // ML-KEM half: dense sweep.
    for (i, p) in (0..MLKEM_CT).step_by(23).enumerate() {
        let mut bad = ct.clone();
        bad[p] ^= 1u8 << (i % 8);
        let (rc, ssb) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
        assert_eq!(rc, 0, "xwing ct[{p}]: the mlkem768_dec != 0 branch is dead");
        assert_ne!(ssb, ss, "xwing ct[{p}]: ss must change");
    }

    // X25519 half: every byte, every bit — never blocklisted for a random-ish
    // point, so always 0 with a different shared secret.
    let mut changed = 0usize;
    for p in MLKEM_CT..XWING_CT {
        for b in 0..8u32 {
            let mut bad = ct.clone();
            bad[p] ^= 1u8 << b;
            let (rc, ssb) = d_dec("crypto_kem_xwing_dec", &bad, &sk);
            assert_eq!(rc, 0, "xwing ct[{p}] bit {b}: not a blocklisted encoding");
            if ssb != ss {
                changed += 1;
            }
        }
    }
    // Only bit 255 of the point is masked away by fe25519_frombytes, so at most
    // one of the 256 flips can leave the secret unchanged.
    assert!(changed >= 255, "expected ≥255 of 256 flips to change ss, got {changed}");
}
