//! CONFIGS.md group E — the public `api.h` surface plus the NIST DRBG in
//! `rng.c`, end to end through both `.so`s.

mod common;

use common::*;
use std::os::raw::{c_int, c_ulong};

type SizeFn = unsafe extern "C" fn() -> u64;
type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type KeypairFn = unsafe extern "C" fn(*mut u8, *mut u8) -> c_int;
type SignatureFn = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type OpenFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type RandomBytesInitFn = unsafe extern "C" fn(*mut u8, *mut u8);
type RandomBytesFn = unsafe extern "C" fn(*mut u8, u64) -> c_int;
type Aes256EcbFn = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type DrbgUpdateFn = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type SeedExpanderInitFn = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, c_ulong) -> c_int;
type SeedExpanderFn = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, c_ulong) -> c_int;

/// Seed both DRBGs with the same entropy so that every `randombytes()`
/// consumer (`crypto_sign_keypair`, the `optrand` in `crypto_sign_signature`)
/// produces identical values on both sides.
fn seed_both(libs: &Libs, entropy: &[u8; 48]) {
    if RAND_URANDOM {
        // `randombytes.c` reads /dev/urandom and has no seeding hook, so the
        // signature's `optrand` cannot be made reproducible in this build.
        return;
    }
    let (c, r) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let mut ec = *entropy;
    let mut er = *entropy;
    unsafe {
        c(ec.as_mut_ptr(), std::ptr::null_mut());
        r(er.as_mut_ptr(), std::ptr::null_mut());
    }
}

#[test]
fn e1_size_functions() {
    let libs = Libs::load();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let c: libloading::os::unix::Symbol<SizeFn> = libs.c(name);
        let r: libloading::os::unix::Symbol<SizeFn> = libs.r(name);
        let (cv, rv) = unsafe { (c(), r()) };
        eq(name, cv, rv);
        eq(&format!("{name} vs params header"), cv, expect);
    }
}

#[test]
fn e2_e3_seed_keypair() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let mut rng = Rng::new(701);
    for i in 0..12 {
        let seed: Vec<u8> = match i {
            0 => vec![0u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            2 => (0..CRYPTO_SEEDBYTES).map(|k| k as u8).collect(),
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let (cr, rr) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()),
            )
        };
        eq("crypto_sign_seed_keypair ret", cr, rr);
        eq("crypto_sign_seed_keypair ret == 0", cr, 0);
        eq_bytes("crypto_sign_seed_keypair pk", &cpk, &rpk);
        eq_bytes("crypto_sign_seed_keypair sk", &csk, &rsk);
    }
}

/// `crypto_sign_keypair` draws its seed from `randombytes()`, so it is only
/// reproducible with the deterministic DRBG provider.
#[cfg(rand_drbg)]
#[test]
fn e4_keypair_via_drbg() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_keypair", KeypairFn);
    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = i as u8;
    }
    seed_both(&libs, &entropy);
    for round in 0..4 {
        let mut cpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut rpk = vec![0xAAu8; SPX_PK_BYTES + 8];
        let mut csk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let mut rsk = vec![0xAAu8; SPX_SK_BYTES + 8];
        let (cr, rr) = unsafe {
            (
                c(cpk.as_mut_ptr(), csk.as_mut_ptr()),
                r(rpk.as_mut_ptr(), rsk.as_mut_ptr()),
            )
        };
        eq("crypto_sign_keypair ret", cr, rr);
        eq_bytes(&format!("crypto_sign_keypair pk (round {round})"), &cpk, &rpk);
        eq_bytes(&format!("crypto_sign_keypair sk (round {round})"), &csk, &rsk);
    }
}

fn api_msg_lens() -> Vec<usize> {
    let mut v: Vec<usize> = vec![
        0, 1, 2, 32, 33, 63, 64, 65, 111, 112, 127, 128, 129, 135, 136, 137, 1000, 5000,
    ];
    let split = SHAX_BLOCK_BYTES.saturating_sub(SPX_N);
    v.extend([split.saturating_sub(1), split, split + 1]);
    let hsplit = (SHA2_INBLOCKS * SHAX_BLOCK_BYTES).saturating_sub(SPX_N + SPX_PK_BYTES);
    v.extend([hsplit.saturating_sub(1), hsplit, hsplit + 1]);
    v.sort_unstable();
    v.dedup();
    v
}

#[test]
fn e5_e7_signature_and_verify() {
    let libs = Libs::load();
    let (ckp, rkp) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cs, rs) = pair!(libs, "crypto_sign_signature", SignatureFn);
    let (cv, rv) = pair!(libs, "crypto_sign_verify", VerifyFn);
    let mut rng = Rng::new(702);
    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = (i as u8).wrapping_mul(7);
    }

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = vec![0u8; SPX_PK_BYTES];
    let mut sk2 = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
    }
    eq_bytes("keypair pk", &pk, &pk2);
    eq_bytes("keypair sk", &sk, &sk2);

    for mlen in api_msg_lens() {
        let m = rng.bytes(mlen.max(1));
        // Re-seed both DRBGs so the internal randombytes(optrand, SPX_N) draws
        // agree, making the signature deterministic across implementations.
        seed_both(&libs, &entropy);
        let mut c_sig = vec![0xAAu8; SPX_BYTES + 8];
        let mut r_sig = vec![0xAAu8; SPX_BYTES + 8];
        let mut c_len: usize = 0;
        let mut r_len: usize = 0;
        let (cr, rr) = unsafe {
            (
                cs(c_sig.as_mut_ptr(), &mut c_len, m.as_ptr(), mlen, sk.as_ptr()),
                rs(r_sig.as_mut_ptr(), &mut r_len, m.as_ptr(), mlen, sk.as_ptr()),
            )
        };
        eq(&format!("crypto_sign_signature(mlen={mlen}) ret"), cr, rr);
        eq(&format!("crypto_sign_signature(mlen={mlen}) siglen"), c_len, r_len);
        eq(&format!("siglen == SPX_BYTES (mlen={mlen})"), c_len, SPX_BYTES);
        if !RAND_URANDOM {
            // Byte equality requires the reproducible `optrand`.
            eq_bytes(&format!("crypto_sign_signature(mlen={mlen}) sig"), &c_sig, &r_sig);
        }

        let (cvr, rvr) = unsafe {
            (
                cv(c_sig.as_ptr(), c_len, m.as_ptr(), mlen, pk.as_ptr()),
                rv(r_sig.as_ptr(), r_len, m.as_ptr(), mlen, pk.as_ptr()),
            )
        };
        eq(&format!("crypto_sign_verify(mlen={mlen}) ret"), cvr, rvr);
        eq(&format!("crypto_sign_verify(mlen={mlen}) accepts"), cvr, 0);
        unsafe {
            eq(
                &format!("Rust verifies C signature (mlen={mlen})"),
                rv(c_sig.as_ptr(), c_len, m.as_ptr(), mlen, pk.as_ptr()),
                0,
            );
            eq(
                &format!("C verifies Rust signature (mlen={mlen})"),
                cv(r_sig.as_ptr(), r_len, m.as_ptr(), mlen, pk.as_ptr()),
                0,
            );
        }
    }
}

#[test]
fn e8_sign_open_roundtrip() {
    let libs = Libs::load();
    let (ckp, rkp) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cs, rs) = pair!(libs, "crypto_sign", SignFn);
    let (co, ro) = pair!(libs, "crypto_sign_open", OpenFn);
    let mut rng = Rng::new(703);
    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = (i as u8).wrapping_mul(11).wrapping_add(3);
    }

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        let mut p2 = vec![0u8; SPX_PK_BYTES];
        let mut s2 = vec![0u8; SPX_SK_BYTES];
        rkp(p2.as_mut_ptr(), s2.as_mut_ptr(), seed.as_ptr());
        eq_bytes("keypair pk", &pk, &p2);
        eq_bytes("keypair sk", &sk, &s2);
    }

    for mlen in [0usize, 1, 33, 64, 136, 1000] {
        let m = rng.bytes(mlen.max(1));
        seed_both(&libs, &entropy);
        let mut c_sm = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut r_sm = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut c_smlen: u64 = 0;
        let mut r_smlen: u64 = 0;
        let (cr, rr) = unsafe {
            (
                cs(c_sm.as_mut_ptr(), &mut c_smlen, m.as_ptr(), mlen as u64, sk.as_ptr()),
                rs(r_sm.as_mut_ptr(), &mut r_smlen, m.as_ptr(), mlen as u64, sk.as_ptr()),
            )
        };
        eq(&format!("crypto_sign(mlen={mlen}) ret"), cr, rr);
        eq(&format!("crypto_sign(mlen={mlen}) smlen"), c_smlen, r_smlen);
        eq(
            &format!("smlen == SPX_BYTES + mlen (mlen={mlen})"),
            c_smlen,
            (SPX_BYTES + mlen) as u64,
        );
        if !RAND_URANDOM {
            eq_bytes(&format!("crypto_sign(mlen={mlen}) sm"), &c_sm, &r_sm);
        }

        let mut c_m = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut r_m = vec![0xAAu8; SPX_BYTES + mlen + 8];
        let mut c_mlen: u64 = 0xDEAD;
        let mut r_mlen: u64 = 0xDEAD;
        let (cor, ror) = unsafe {
            (
                co(c_m.as_mut_ptr(), &mut c_mlen, c_sm.as_ptr(), c_smlen, pk.as_ptr()),
                ro(r_m.as_mut_ptr(), &mut r_mlen, r_sm.as_ptr(), r_smlen, pk.as_ptr()),
            )
        };
        eq(&format!("crypto_sign_open(mlen={mlen}) ret"), cor, ror);
        eq(&format!("crypto_sign_open(mlen={mlen}) ret == 0"), cor, 0);
        eq(&format!("crypto_sign_open(mlen={mlen}) mlen"), c_mlen, r_mlen);
        eq(&format!("recovered mlen (mlen={mlen})"), c_mlen, mlen as u64);
        if !RAND_URANDOM {
            eq_bytes(&format!("crypto_sign_open(mlen={mlen}) m"), &c_m, &r_m);
        }
        eq_bytes("recovered message (C)", &c_m[..mlen], &m[..mlen]);
        eq_bytes("recovered message (Rust)", &r_m[..mlen], &m[..mlen]);
    }
}

#[test]
fn e9_cross_verify() {
    let libs = Libs::load();
    let (ckp, _rkp) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cs, rs) = pair!(libs, "crypto_sign_signature", SignatureFn);
    let (cv, rv) = pair!(libs, "crypto_sign_verify", VerifyFn);
    let mut rng = Rng::new(704);
    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = (i as u8) ^ 0x5A;
    }

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }

    for mlen in [0usize, 1, 64, 137, 1000] {
        let m = rng.bytes(mlen.max(1));
        seed_both(&libs, &entropy);
        let mut c_sig = vec![0u8; SPX_BYTES];
        let mut r_sig = vec![0u8; SPX_BYTES];
        let mut cl = 0usize;
        let mut rl = 0usize;
        unsafe {
            cs(c_sig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr());
            rs(r_sig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr());
            // C's signature verified by Rust, and vice versa.
            eq(
                "Rust verifies C signature",
                rv(c_sig.as_ptr(), cl, m.as_ptr(), mlen, pk.as_ptr()),
                0,
            );
            eq(
                "C verifies Rust signature",
                cv(r_sig.as_ptr(), rl, m.as_ptr(), mlen, pk.as_ptr()),
                0,
            );
        }
    }
}

/// The DRBG `randombytes` from `rng.c`; with the `urandom` feature the exported
/// `randombytes` is the `/dev/urandom` one instead (covered in
/// `diff_f_urandom.rs`).
#[cfg(rand_drbg)]
#[test]
fn e10_e11_randombytes_drbg() {
    let libs = Libs::load();
    let (cinit, rinit) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let (crb, rrb) = pair!(libs, "randombytes", RandomBytesFn);
    let c_drbg: libloading::os::unix::Symbol<*const u8> = libs.c("DRBG_ctx");
    let r_drbg: libloading::os::unix::Symbol<*const u8> = libs.r("DRBG_ctx");
    let c_drbg = c_drbg.into_raw() as *const u8;
    let r_drbg = r_drbg.into_raw() as *const u8;
    // sizeof(AES256_CTR_DRBG_struct) == 32 + 16 + 4 (+4 padding)
    let drbg_len = 52usize;
    let snap = |p: *const u8| unsafe { std::slice::from_raw_parts(p, drbg_len).to_vec() };

    let mut rng = Rng::new(705);
    for round in 0..8 {
        let mut entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
        // E11: alternate between a NULL and a non-NULL personalization string.
        let mut pers: [u8; 48] = rng.bytes(48).try_into().unwrap();
        let mut ec = entropy;
        let mut er = entropy;
        unsafe {
            if round % 2 == 0 {
                cinit(ec.as_mut_ptr(), std::ptr::null_mut());
                rinit(er.as_mut_ptr(), std::ptr::null_mut());
            } else {
                let mut pc = pers;
                let mut pr = pers;
                cinit(ec.as_mut_ptr(), pc.as_mut_ptr());
                rinit(er.as_mut_ptr(), pr.as_mut_ptr());
                eq_bytes("randombytes_init personalization untouched", &pc, &pr);
            }
        }
        eq_bytes("DRBG_ctx after randombytes_init", &snap(c_drbg), &snap(r_drbg));
        entropy[0] ^= 0;
        pers[0] ^= 0;

        for &xlen in &[0usize, 1, 15, 16, 17, 31, 32, 48, 63, 64, 65, 1000] {
            let mut cb = vec![0xAAu8; xlen + 8];
            let mut rb = vec![0xAAu8; xlen + 8];
            let (cr, rr) = unsafe {
                (
                    crb(cb.as_mut_ptr(), xlen as u64),
                    rrb(rb.as_mut_ptr(), xlen as u64),
                )
            };
            eq(&format!("randombytes(xlen={xlen}) ret"), cr, rr);
            eq(&format!("randombytes(xlen={xlen}) ret == RNG_SUCCESS"), cr, 0);
            eq_bytes(&format!("randombytes(xlen={xlen}) output"), &cb, &rb);
            eq_bytes(
                &format!("DRBG_ctx after randombytes(xlen={xlen})"),
                &snap(c_drbg),
                &snap(r_drbg),
            );
        }
    }
}

#[test]
fn e12_aes256_ecb() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "AES256_ECB", Aes256EcbFn);
    let mut rng = Rng::new(706);
    for i in 0..500 {
        let key: Vec<u8> = match i {
            0 => vec![0u8; 32],
            1 => vec![0xFFu8; 32],
            _ => rng.bytes(32),
        };
        let ctr: Vec<u8> = match i {
            0 => vec![0u8; 16],
            1 => vec![0xFFu8; 16],
            _ => rng.bytes(16),
        };
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cc = ctr.clone();
        let mut rc = ctr.clone();
        let mut cb = vec![0xAAu8; 16 + 8];
        let mut rb = vec![0xAAu8; 16 + 8];
        unsafe {
            c(ck.as_mut_ptr(), cc.as_mut_ptr(), cb.as_mut_ptr());
            r(rk.as_mut_ptr(), rc.as_mut_ptr(), rb.as_mut_ptr());
        }
        eq_bytes("AES256_ECB buffer", &cb, &rb);
        eq_bytes("AES256_ECB key untouched", &ck, &rk);
        eq_bytes("AES256_ECB ctr untouched", &cc, &rc);
    }
}

#[test]
fn e13_drbg_update() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "AES256_CTR_DRBG_Update", DrbgUpdateFn);
    let mut rng = Rng::new(707);
    for i in 0..400 {
        let key = rng.bytes(32);
        let v = rng.bytes(16);
        let pd = rng.bytes(48);
        let use_pd = i % 2 == 0;
        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cv = v.clone();
        let mut rv = v.clone();
        let mut cpd = pd.clone();
        let mut rpd = pd.clone();
        unsafe {
            if use_pd {
                c(cpd.as_mut_ptr(), ck.as_mut_ptr(), cv.as_mut_ptr());
                r(rpd.as_mut_ptr(), rk.as_mut_ptr(), rv.as_mut_ptr());
                eq_bytes("AES256_CTR_DRBG_Update provided_data untouched", &cpd, &rpd);
            } else {
                c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
                r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
            }
        }
        eq_bytes(
            &format!("AES256_CTR_DRBG_Update Key (pd={use_pd})"),
            &ck,
            &rk,
        );
        eq_bytes(&format!("AES256_CTR_DRBG_Update V (pd={use_pd})"), &cv, &rv);
    }
}

#[test]
fn e14_seedexpander() {
    let libs = Libs::load();
    let (ci, ri) = pair!(libs, "seedexpander_init", SeedExpanderInitFn);
    let (ce, re) = pair!(libs, "seedexpander", SeedExpanderFn);
    let mut rng = Rng::new(708);

    for &maxlen in &[1u64, 16, 17, 64, 1024, 0xFFFF_FFFF] {
        for _ in 0..20 {
            let seed = rng.bytes(32);
            let div = rng.bytes(8);
            let mut cs = AesXofStruct::zeroed();
            let mut rs = AesXofStruct::zeroed();
            let mut cseed = seed.clone();
            let mut rseed = seed.clone();
            let mut cdiv = div.clone();
            let mut rdiv = div.clone();
            let (cr, rr) = unsafe {
                (
                    ci(&mut cs, cseed.as_mut_ptr(), cdiv.as_mut_ptr(), maxlen as c_ulong),
                    ri(&mut rs, rseed.as_mut_ptr(), rdiv.as_mut_ptr(), maxlen as c_ulong),
                )
            };
            eq(&format!("seedexpander_init(maxlen={maxlen}) ret"), cr, rr);
            eq_bytes(
                &format!("seedexpander_init(maxlen={maxlen}) ctx"),
                cs.as_bytes(),
                rs.as_bytes(),
            );

            for &xlen in &[0u64, 1, 15, 16, 17, 100] {
                let mut cb = vec![0xAAu8; xlen as usize + 8];
                let mut rb = vec![0xAAu8; xlen as usize + 8];
                let (cr, rr) = unsafe {
                    (
                        ce(&mut cs, cb.as_mut_ptr(), xlen as c_ulong),
                        re(&mut rs, rb.as_mut_ptr(), xlen as c_ulong),
                    )
                };
                eq(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"),
                    cr,
                    rr,
                );
                eq_bytes(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) out"),
                    &cb,
                    &rb,
                );
                eq_bytes(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ctx"),
                    cs.as_bytes(),
                    rs.as_bytes(),
                );
            }
        }
    }
}
