//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input the C checks for and asserts
//! that both `.so`s return the *same* sentinel (`-1` / `-2` / `-3` / `0`), not
//! merely that both "failed".

mod common;

use common::*;
use std::os::raw::{c_int, c_uint, c_ulong};

const RNG_SUCCESS: c_int = 0;
const RNG_BAD_MAXLEN: c_int = -1;
const RNG_BAD_OUTBUF: c_int = -2;
const RNG_BAD_REQ_LEN: c_int = -3;

type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> c_int;
type SignatureFn = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> c_int;
type VerifyFn = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> c_int;
type SignFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type OpenFn = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> c_int;
type RandomBytesInitFn = unsafe extern "C" fn(*mut u8, *mut u8);
type RandomBytesFn = unsafe extern "C" fn(*mut u8, u64) -> c_int;
type DrbgUpdateFn = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type SeedExpanderInitFn =
    unsafe extern "C" fn(*mut AesXofStruct, *mut u8, *mut u8, c_ulong) -> c_int;
type SeedExpanderFn = unsafe extern "C" fn(*mut AesXofStruct, *mut u8, c_ulong) -> c_int;

/// A valid (pk, sk, m, sig) tuple built identically on both sides.
#[allow(dead_code)]
struct Fixture {
    pk: Vec<u8>,
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

fn fixture(libs: &Libs, mlen: usize) -> Fixture {
    let (ckp, rkp) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cinit, rinit) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let (cs, rs) = pair!(libs, "crypto_sign_signature", SignatureFn);
    let mut rng = Rng::new(901);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let m = rng.bytes(mlen.max(1));

    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = vec![0u8; SPX_PK_BYTES];
    let mut sk2 = vec![0u8; SPX_SK_BYTES];
    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = (i as u8).wrapping_mul(13);
    }
    let mut sig = vec![0u8; SPX_BYTES];
    let mut sig2 = vec![0u8; SPX_BYTES];
    let mut l1 = 0usize;
    let mut l2 = 0usize;
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
        eq_bytes("fixture pk", &pk, &pk2);
        eq_bytes("fixture sk", &sk, &sk2);
        if !RAND_URANDOM {
            let mut ec = entropy;
            let mut er = entropy;
            cinit(ec.as_mut_ptr(), std::ptr::null_mut());
            rinit(er.as_mut_ptr(), std::ptr::null_mut());
        }
        cs(sig.as_mut_ptr(), &mut l1, m.as_ptr(), mlen, sk.as_ptr());
        rs(sig2.as_mut_ptr(), &mut l2, m.as_ptr(), mlen, sk.as_ptr());
        eq("fixture siglen", l1, l2);
        if !RAND_URANDOM {
            // With the `urandom` provider `optrand` is not reproducible, so the
            // C signature (which both sides must still verify identically) is
            // used as the fixture.
            eq_bytes("fixture sig", &sig, &sig2);
        }
    }
    Fixture { pk, sk, m, sig }
}

// ---------------------------------------------------------------------------
// Row 1 — crypto_sign_verify: siglen != SPX_BYTES  (sign.c:179)
// ---------------------------------------------------------------------------

#[test]
fn row01_verify_wrong_siglen() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_verify", VerifyFn);
    let f = fixture(&libs, 33);

    for &sl in &[
        0usize,
        1,
        SPX_N,
        SPX_BYTES - 1,
        SPX_BYTES + 1,
        SPX_BYTES * 2,
        usize::MAX,
    ] {
        let (cv, rv) = unsafe {
            (
                c(f.sig.as_ptr(), sl, f.m.as_ptr(), f.m.len(), f.pk.as_ptr()),
                r(f.sig.as_ptr(), sl, f.m.as_ptr(), f.m.len(), f.pk.as_ptr()),
            )
        };
        eq(&format!("verify(siglen={sl}) ret"), cv, rv);
        eq(&format!("verify(siglen={sl}) rejects with -1"), cv, -1);
    }
    // and the accepted length still works
    let (cv, rv) = unsafe {
        (
            c(f.sig.as_ptr(), SPX_BYTES, f.m.as_ptr(), f.m.len(), f.pk.as_ptr()),
            r(f.sig.as_ptr(), SPX_BYTES, f.m.as_ptr(), f.m.len(), f.pk.as_ptr()),
        )
    };
    eq("verify(siglen=SPX_BYTES) ret", cv, rv);
    eq("verify(siglen=SPX_BYTES) accepts", cv, 0);
}

// ---------------------------------------------------------------------------
// Row 2 — crypto_sign_verify: root mismatch  (sign.c:235)
// ---------------------------------------------------------------------------

#[test]
fn row02_verify_root_mismatch() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_verify", VerifyFn);
    const MLEN: usize = 1024;
    let f = fixture(&libs, MLEN);
    let mut rng = Rng::new(902);

    // How many bytes of `m` actually reach the message digest.
    //
    // `lib/blake/src/blake256.c`'s `blake256_update(S, data, datalen)` takes
    // `datalen` in BITS (`blake256()` itself calls it with `inlen*8`), but
    // `lib/blake/src/hash_blake.c`'s `hash_message()` passes `mlen` — a byte
    // count.  So the blake backend only absorbs `mlen / 8` message bytes.  That
    // is what the C does, so the Rust must do it too, and a differential test
    // may only demand *rejection* for corruptions the C actually observes.
    let observed = if IS_BLAKE { MLEN / 8 } else { MLEN };
    assert!(observed > 0);

    // Corrupt a signature byte, a public-key byte, or an *observed* message
    // byte: all three must be rejected with -1 by both implementations.
    for which in 0..3 {
        for _ in 0..25 {
            let mut sig = f.sig.clone();
            let mut m = f.m.clone();
            let mut pk = f.pk.clone();
            match which {
                0 => {
                    let i = rng.below(SPX_BYTES as u32) as usize;
                    sig[i] ^= 1 << (rng.below(8) as u8);
                }
                1 => {
                    let i = rng.below(observed as u32) as usize;
                    m[i] ^= 1 << (rng.below(8) as u8);
                }
                _ => {
                    let i = rng.below(SPX_PK_BYTES as u32) as usize;
                    pk[i] ^= 1 << (rng.below(8) as u8);
                }
            }
            let (cv, rv) = unsafe {
                (
                    c(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()),
                    r(sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), pk.as_ptr()),
                )
            };
            eq(&format!("verify(corrupt {which}) ret"), cv, rv);
            assert_eq!(cv, -1, "verify(corrupt {which}) unexpectedly accepted");
        }
    }

    // Corrupting a message byte the backend does *not* read must produce the
    // same (accepting) answer on both sides — this is where the blake bit/byte
    // quirk shows up, and it must be reproduced, not "fixed".
    for _ in 0..40 {
        let mut m = f.m.clone();
        let i = observed + rng.below((MLEN - observed).max(1) as u32) as usize;
        if i >= MLEN {
            continue;
        }
        m[i] ^= 1 << (rng.below(8) as u8);
        let (cv, rv) = unsafe {
            (
                c(f.sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), f.pk.as_ptr()),
                r(f.sig.as_ptr(), SPX_BYTES, m.as_ptr(), m.len(), f.pk.as_ptr()),
            )
        };
        eq("verify(corrupt unobserved message byte) ret", cv, rv);
        if !IS_BLAKE {
            assert_eq!(cv, -1, "non-blake backends read the whole message");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 3 / 4 / 21 — crypto_sign_open
// ---------------------------------------------------------------------------

#[test]
fn row03_open_smlen_too_small() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_open", OpenFn);
    let f = fixture(&libs, 33);
    let mut sm = f.sig.clone();
    sm.extend_from_slice(&f.m);

    for &sl in &[0u64, 1, 16, (SPX_N as u64), (SPX_BYTES - 1) as u64] {
        // `crypto_sign_open` does memset(m, 0, smlen) on this path, so `m` must
        // have at least `smlen` bytes.
        let n = sl as usize + 8;
        let mut cm = vec![0xAAu8; n];
        let mut rm = vec![0xAAu8; n];
        let mut cl: u64 = 0xDEAD;
        let mut rl: u64 = 0xDEAD;
        let (cv, rv) = unsafe {
            (
                c(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), sl, f.pk.as_ptr()),
                r(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), sl, f.pk.as_ptr()),
            )
        };
        eq(&format!("open(smlen={sl}) ret"), cv, rv);
        eq(&format!("open(smlen={sl}) rejects with -1"), cv, -1);
        eq(&format!("open(smlen={sl}) *mlen"), cl, rl);
        eq(&format!("open(smlen={sl}) *mlen == 0"), cl, 0);
        eq_bytes(&format!("open(smlen={sl}) m buffer"), &cm, &rm);
        // C does memset(m, 0, smlen); the guard bytes past it stay 0xAA.
        assert!(cm[..sl as usize].iter().all(|&b| b == 0));
        assert!(cm[sl as usize..].iter().all(|&b| b == 0xAA));
    }
}

#[test]
fn row04_open_bad_signature() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_open", OpenFn);
    let f = fixture(&libs, 33);
    let mut rng = Rng::new(903);

    for which in 0..2 {
        for _ in 0..12 {
            let mut sm = f.sig.clone();
            sm.extend_from_slice(&f.m);
            let mut pk = f.pk.clone();
            if which == 0 {
                let i = rng.below(sm.len() as u32) as usize;
                sm[i] ^= 1 << (rng.below(8) as u8);
            } else {
                let i = rng.below(SPX_PK_BYTES as u32) as usize;
                pk[i] ^= 1 << (rng.below(8) as u8);
            }
            let smlen = sm.len() as u64;
            let mut cm = vec![0xAAu8; smlen as usize + 8];
            let mut rm = vec![0xAAu8; smlen as usize + 8];
            let mut cl: u64 = 0xDEAD;
            let mut rl: u64 = 0xDEAD;
            let (cv, rv) = unsafe {
                (
                    c(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), smlen, pk.as_ptr()),
                    r(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), smlen, pk.as_ptr()),
                )
            };
            eq("open(bad sig) ret", cv, rv);
            eq("open(bad sig) rejects with -1", cv, -1);
            eq("open(bad sig) *mlen", cl, rl);
            eq("open(bad sig) *mlen == 0", cl, 0);
            eq_bytes("open(bad sig) m buffer", &cm, &rm);
            // sign.c:278 memsets `smlen` bytes, NOT `*mlen` bytes.
            assert!(cm[..smlen as usize].iter().all(|&b| b == 0));
            assert!(cm[smlen as usize..].iter().all(|&b| b == 0xAA));
        }
    }
}

#[test]
fn row21_open_empty_message() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "crypto_sign_open", OpenFn);
    let (cs, rs) = pair!(libs, "crypto_sign", SignFn);
    let (ckp, _) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cinit, rinit) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let mut rng = Rng::new(904);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    unsafe { ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = i as u8;
    }
    let mut ec = entropy;
    let mut er = entropy;
    let mut c_sm = vec![0u8; SPX_BYTES];
    let mut r_sm = vec![0u8; SPX_BYTES];
    let mut c_smlen = 0u64;
    let mut r_smlen = 0u64;
    let dummy = [0u8; 1];
    unsafe {
        if !RAND_URANDOM {
            cinit(ec.as_mut_ptr(), std::ptr::null_mut());
            rinit(er.as_mut_ptr(), std::ptr::null_mut());
        }
        cs(c_sm.as_mut_ptr(), &mut c_smlen, dummy.as_ptr(), 0, sk.as_ptr());
        rs(r_sm.as_mut_ptr(), &mut r_smlen, dummy.as_ptr(), 0, sk.as_ptr());
    }
    eq("crypto_sign(mlen=0) smlen", c_smlen, r_smlen);
    eq("crypto_sign(mlen=0) smlen == SPX_BYTES", c_smlen, SPX_BYTES as u64);
    if !RAND_URANDOM {
        eq_bytes("crypto_sign(mlen=0) sm", &c_sm, &r_sm);
    }

    let mut cm = vec![0xAAu8; SPX_BYTES + 8];
    let mut rm = vec![0xAAu8; SPX_BYTES + 8];
    let mut cl = 0xDEADu64;
    let mut rl = 0xDEADu64;
    let (cv, rv) = unsafe {
        (
            c(cm.as_mut_ptr(), &mut cl, c_sm.as_ptr(), c_smlen, pk.as_ptr()),
            r(rm.as_mut_ptr(), &mut rl, r_sm.as_ptr(), r_smlen, pk.as_ptr()),
        )
    };
    eq("open(smlen == SPX_BYTES) ret", cv, rv);
    eq("open(smlen == SPX_BYTES) accepts", cv, 0);
    eq("open(smlen == SPX_BYTES) *mlen", cl, rl);
    eq("open(smlen == SPX_BYTES) *mlen == 0", cl, 0);
    if !RAND_URANDOM {
        eq_bytes("open(smlen == SPX_BYTES) m", &cm, &rm);
    }
}

// ---------------------------------------------------------------------------
// Rows 5 / 6 — seedexpander_init
// ---------------------------------------------------------------------------

#[test]
fn row05_seedexpander_init_maxlen_too_large() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander_init", SeedExpanderInitFn);
    let mut rng = Rng::new(905);
    for &maxlen in &[
        0x1_0000_0000u64,
        0x1_0000_0001,
        0x2_0000_0000,
        u64::MAX,
    ] {
        let seed = rng.bytes(32);
        let div = rng.bytes(8);
        // Pre-fill the contexts with a recognisable pattern: on this path the C
        // returns before touching `ctx`, so it must come back unchanged.
        let mut cs = AesXofStruct::zeroed();
        let mut rs = AesXofStruct::zeroed();
        cs.buffer = [0x5A; 16];
        rs.buffer = [0x5A; 16];
        cs.buffer_pos = 0x1122_3344;
        rs.buffer_pos = 0x1122_3344;
        cs.length_remaining = 0x5566_7788;
        rs.length_remaining = 0x5566_7788;
        cs.key = [0x3C; 32];
        rs.key = [0x3C; 32];
        cs.ctr = [0xC3; 16];
        rs.ctr = [0xC3; 16];
        let before = cs;

        let mut cseed = seed.clone();
        let mut rseed = seed.clone();
        let mut cdiv = div.clone();
        let mut rdiv = div.clone();
        let (cv, rv) = unsafe {
            (
                c(&mut cs, cseed.as_mut_ptr(), cdiv.as_mut_ptr(), maxlen as c_ulong),
                r(&mut rs, rseed.as_mut_ptr(), rdiv.as_mut_ptr(), maxlen as c_ulong),
            )
        };
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ret"), cv, rv);
        eq(
            &format!("seedexpander_init(maxlen={maxlen:#x}) == RNG_BAD_MAXLEN"),
            cv,
            RNG_BAD_MAXLEN,
        );
        eq_bytes(
            "seedexpander_init rejected: ctx unchanged (C)",
            before.as_bytes(),
            cs.as_bytes(),
        );
        eq_bytes(
            "seedexpander_init rejected: ctx unchanged (Rust)",
            before.as_bytes(),
            rs.as_bytes(),
        );
    }
}

#[test]
fn row06_seedexpander_init_maxlen_boundary() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander_init", SeedExpanderInitFn);
    let mut rng = Rng::new(906);
    // 0xFFFFFFFF is the largest accepted value (one below the `>=` check).
    for &maxlen in &[0xFFFF_FFFFu64, 0xFFFF_FFFE, 0x1_0000_0000 - 1] {
        for _ in 0..10 {
            let seed = rng.bytes(32);
            let div = rng.bytes(8);
            let mut cs = AesXofStruct::zeroed();
            let mut rs = AesXofStruct::zeroed();
            let mut cseed = seed.clone();
            let mut rseed = seed.clone();
            let mut cdiv = div.clone();
            let mut rdiv = div.clone();
            let (cv, rv) = unsafe {
                (
                    c(&mut cs, cseed.as_mut_ptr(), cdiv.as_mut_ptr(), maxlen as c_ulong),
                    r(&mut rs, rseed.as_mut_ptr(), rdiv.as_mut_ptr(), maxlen as c_ulong),
                )
            };
            eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ret"), cv, rv);
            eq(
                &format!("seedexpander_init(maxlen={maxlen:#x}) == RNG_SUCCESS"),
                cv,
                RNG_SUCCESS,
            );
            eq_bytes("seedexpander_init accepted ctx", cs.as_bytes(), rs.as_bytes());
            eq("length_remaining", cs.length_remaining, maxlen);
            eq("buffer_pos == 16", cs.buffer_pos, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7 / 8 / 9 / 10 / 11 — seedexpander
// ---------------------------------------------------------------------------

fn init_expander(libs: &Libs, maxlen: u64, seed: &[u8], div: &[u8]) -> (AesXofStruct, AesXofStruct) {
    let (c, r) = pair!(libs, "seedexpander_init", SeedExpanderInitFn);
    let mut cs = AesXofStruct::zeroed();
    let mut rs = AesXofStruct::zeroed();
    let mut cseed = seed.to_vec();
    let mut rseed = seed.to_vec();
    let mut cdiv = div.to_vec();
    let mut rdiv = div.to_vec();
    unsafe {
        let a = c(&mut cs, cseed.as_mut_ptr(), cdiv.as_mut_ptr(), maxlen as c_ulong);
        let b = r(&mut rs, rseed.as_mut_ptr(), rdiv.as_mut_ptr(), maxlen as c_ulong);
        eq("init_expander ret", a, b);
    }
    eq_bytes("init_expander ctx", cs.as_bytes(), rs.as_bytes());
    (cs, rs)
}

#[test]
fn row07_seedexpander_null_output() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander", SeedExpanderFn);
    let mut rng = Rng::new(907);
    for &xlen in &[0u64, 1, 16, 100, 0xFFFF_FFFF] {
        let (mut cs, mut rs) = init_expander(&libs, 1024, &rng.bytes(32), &rng.bytes(8));
        let before = cs;
        let (cv, rv) = unsafe {
            (
                c(&mut cs, std::ptr::null_mut(), xlen as c_ulong),
                r(&mut rs, std::ptr::null_mut(), xlen as c_ulong),
            )
        };
        eq(&format!("seedexpander(x=NULL, xlen={xlen}) ret"), cv, rv);
        eq(
            &format!("seedexpander(x=NULL, xlen={xlen}) == RNG_BAD_OUTBUF"),
            cv,
            RNG_BAD_OUTBUF,
        );
        eq_bytes("seedexpander(NULL) ctx unchanged (C)", before.as_bytes(), cs.as_bytes());
        eq_bytes(
            "seedexpander(NULL) ctx unchanged (Rust)",
            before.as_bytes(),
            rs.as_bytes(),
        );
    }
}

#[test]
fn row08_row09_seedexpander_req_len() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander", SeedExpanderFn);
    let mut rng = Rng::new(908);

    for &maxlen in &[1u64, 16, 17, 64, 1024] {
        // Row 8: `xlen >= ctx->length_remaining` is rejected — note that
        // requesting *exactly* the remaining length fails.
        for &xlen in &[maxlen, maxlen + 1, maxlen * 2, 0xFFFF_FFFF] {
            let (mut cs, mut rs) = init_expander(&libs, maxlen, &rng.bytes(32), &rng.bytes(8));
            let before = cs;
            let mut cb = vec![0xAAu8; xlen.min(4096) as usize + 8];
            let mut rb = vec![0xAAu8; xlen.min(4096) as usize + 8];
            let (cv, rv) = unsafe {
                (
                    c(&mut cs, cb.as_mut_ptr(), xlen as c_ulong),
                    r(&mut rs, rb.as_mut_ptr(), xlen as c_ulong),
                )
            };
            eq(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"), cv, rv);
            eq(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) == RNG_BAD_REQ_LEN"),
                cv,
                RNG_BAD_REQ_LEN,
            );
            eq_bytes("rejected: nothing written (C vs Rust)", &cb, &rb);
            assert!(cb.iter().all(|&b| b == 0xAA), "buffer was written on rejection");
            eq_bytes("rejected: ctx unchanged (C)", before.as_bytes(), cs.as_bytes());
            eq_bytes("rejected: ctx unchanged (Rust)", before.as_bytes(), rs.as_bytes());
        }

        // Row 9: `length_remaining - 1` is the largest accepted request.
        if maxlen >= 1 {
            let xlen = maxlen - 1;
            let (mut cs, mut rs) = init_expander(&libs, maxlen, &rng.bytes(32), &rng.bytes(8));
            let mut cb = vec![0xAAu8; xlen as usize + 8];
            let mut rb = vec![0xAAu8; xlen as usize + 8];
            let (cv, rv) = unsafe {
                (
                    c(&mut cs, cb.as_mut_ptr(), xlen as c_ulong),
                    r(&mut rs, rb.as_mut_ptr(), xlen as c_ulong),
                )
            };
            eq(&format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ret"), cv, rv);
            eq(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) == RNG_SUCCESS"),
                cv,
                RNG_SUCCESS,
            );
            eq_bytes("accepted output", &cb, &rb);
            eq_bytes("accepted ctx", cs.as_bytes(), rs.as_bytes());
            eq("length_remaining decremented", cs.length_remaining, maxlen - xlen);
        }
    }
}

#[test]
fn row10_seedexpander_zero_len() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander", SeedExpanderFn);
    let mut rng = Rng::new(909);
    for &maxlen in &[1u64, 16, 1024] {
        let (mut cs, mut rs) = init_expander(&libs, maxlen, &rng.bytes(32), &rng.bytes(8));
        let mut cb = vec![0xAAu8; 8];
        let mut rb = vec![0xAAu8; 8];
        let (cv, rv) = unsafe {
            (
                c(&mut cs, cb.as_mut_ptr(), 0),
                r(&mut rs, rb.as_mut_ptr(), 0),
            )
        };
        eq(&format!("seedexpander(maxlen={maxlen}, xlen=0) ret"), cv, rv);
        // maxlen == 0 would make `0 >= length_remaining` true; for maxlen >= 1
        // the request is accepted and the while loop simply never runs.
        eq(
            &format!("seedexpander(maxlen={maxlen}, xlen=0) == RNG_SUCCESS"),
            cv,
            RNG_SUCCESS,
        );
        eq_bytes("xlen=0 writes nothing", &cb, &rb);
        assert!(cb.iter().all(|&b| b == 0xAA));
        eq_bytes("xlen=0 ctx", cs.as_bytes(), rs.as_bytes());
    }
}

#[test]
fn row11_seedexpander_maxlen_zero() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "seedexpander", SeedExpanderFn);
    let mut rng = Rng::new(910);
    let (mut cs, mut rs) = init_expander(&libs, 0, &rng.bytes(32), &rng.bytes(8));
    eq("maxlen=0 -> length_remaining 0", cs.length_remaining, 0);
    for &xlen in &[0u64, 1, 16, 100] {
        let mut cb = vec![0xAAu8; xlen as usize + 8];
        let mut rb = vec![0xAAu8; xlen as usize + 8];
        let (cv, rv) = unsafe {
            (
                c(&mut cs, cb.as_mut_ptr(), xlen as c_ulong),
                r(&mut rs, rb.as_mut_ptr(), xlen as c_ulong),
            )
        };
        eq(&format!("seedexpander(maxlen=0, xlen={xlen}) ret"), cv, rv);
        eq(
            &format!("seedexpander(maxlen=0, xlen={xlen}) == RNG_BAD_REQ_LEN"),
            cv,
            RNG_BAD_REQ_LEN,
        );
        eq_bytes("maxlen=0 nothing written", &cb, &rb);
    }
}

// ---------------------------------------------------------------------------
// Rows 12 / 13 / 14 — randombytes / randombytes_init
// ---------------------------------------------------------------------------

/// Rows 12–14 concern `rng.c`'s DRBG `randombytes`.  With the `urandom`
/// feature the exported `randombytes` is `randombytes.c`'s instead; see
/// `diff_f_urandom.rs` for that provider.
#[cfg(rand_drbg)]
#[test]
fn row12_row13_row14_randombytes() {
    let libs = Libs::load();
    let (cinit, rinit) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let (crb, rrb) = pair!(libs, "randombytes", RandomBytesFn);
    let c_drbg = libs.c::<*const u8>("DRBG_ctx").into_raw() as *const u8;
    let r_drbg = libs.r::<*const u8>("DRBG_ctx").into_raw() as *const u8;
    let snap = |p: *const u8| unsafe { std::slice::from_raw_parts(p, 52).to_vec() };

    let mut rng = Rng::new(911);
    let entropy: [u8; 48] = rng.bytes(48).try_into().unwrap();
    let pers: [u8; 48] = rng.bytes(48).try_into().unwrap();

    // Row 13: personalization_string == NULL.
    let mut ec = entropy;
    let mut er = entropy;
    unsafe {
        cinit(ec.as_mut_ptr(), std::ptr::null_mut());
        rinit(er.as_mut_ptr(), std::ptr::null_mut());
    }
    let null_state = snap(c_drbg);
    eq_bytes("randombytes_init(NULL pers) DRBG_ctx", &null_state, &snap(r_drbg));

    // Row 12: xlen == 0 still runs AES256_CTR_DRBG_Update and bumps
    // reseed_counter, so the state must advance identically.
    let mut cb = [0xAAu8; 8];
    let mut rb = [0xAAu8; 8];
    let (cv, rv) = unsafe { (crb(cb.as_mut_ptr(), 0), rrb(rb.as_mut_ptr(), 0)) };
    eq("randombytes(xlen=0) ret", cv, rv);
    eq("randombytes(xlen=0) == RNG_SUCCESS", cv, RNG_SUCCESS);
    eq_bytes("randombytes(xlen=0) writes nothing", &cb, &rb);
    assert!(cb.iter().all(|&b| b == 0xAA));
    let after0 = snap(c_drbg);
    eq_bytes("randombytes(xlen=0) DRBG_ctx", &after0, &snap(r_drbg));
    assert_ne!(after0, null_state, "state must advance even for xlen == 0");

    // Row 14: non-NULL personalization_string XORs all 48 bytes.
    let mut ec = entropy;
    let mut er = entropy;
    let mut pc = pers;
    let mut pr = pers;
    unsafe {
        cinit(ec.as_mut_ptr(), pc.as_mut_ptr());
        rinit(er.as_mut_ptr(), pr.as_mut_ptr());
    }
    let pers_state = snap(c_drbg);
    eq_bytes("randombytes_init(pers) DRBG_ctx", &pers_state, &snap(r_drbg));
    assert_ne!(pers_state, null_state, "personalization must change the state");
    eq_bytes("randombytes_init leaves entropy_input alone", &ec, &er);
    eq_bytes("randombytes_init leaves pers alone", &pc, &pr);
}

// ---------------------------------------------------------------------------
// Row 22 — AES256_CTR_DRBG_Update with provided_data == NULL
// ---------------------------------------------------------------------------

#[test]
fn row22_drbg_update_null_provided_data() {
    let libs = Libs::load();
    let (c, r) = pair!(libs, "AES256_CTR_DRBG_Update", DrbgUpdateFn);
    let mut rng = Rng::new(912);
    for i in 0..80 {
        let key = if i == 0 { vec![0u8; 32] } else { rng.bytes(32) };
        let v = if i == 0 { vec![0u8; 16] } else { rng.bytes(16) };

        let mut ck = key.clone();
        let mut rk = key.clone();
        let mut cv = v.clone();
        let mut rv = v.clone();
        unsafe {
            c(std::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            r(std::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(NULL) Key", &ck, &rk);
        eq_bytes("DRBG_Update(NULL) V", &cv, &rv);

        // and the non-NULL path must differ from the NULL path
        let pd = rng.bytes(48);
        let mut ck2 = key.clone();
        let mut rk2 = key.clone();
        let mut cv2 = v.clone();
        let mut rv2 = v.clone();
        let mut cpd = pd.clone();
        let mut rpd = pd.clone();
        unsafe {
            c(cpd.as_mut_ptr(), ck2.as_mut_ptr(), cv2.as_mut_ptr());
            r(rpd.as_mut_ptr(), rk2.as_mut_ptr(), rv2.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(pd) Key", &ck2, &rk2);
        eq_bytes("DRBG_Update(pd) V", &cv2, &rv2);
        eq_bytes("DRBG_Update(pd) provided_data untouched", &cpd, &rpd);
    }
}

// ---------------------------------------------------------------------------
// Rows 15 / 16 — out-of-range address-field values (C truncates, no validation)
// ---------------------------------------------------------------------------

#[test]
fn row15_row16_out_of_range_address_fields() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u32, u32);
    let mut rng = Rng::new(913);

    // Row 15: `set_type` with values that are not any SPX_ADDR_TYPE_*.
    // Row 16: the other single-byte setters with values above 255.
    for name in [
        "SPX_set_type",
        "SPX_set_layer_addr",
        "SPX_set_chain_addr",
        "SPX_set_hash_addr",
        "SPX_set_tree_height",
    ] {
        let c: libloading::os::unix::Symbol<F> = libs.c(name);
        let r: libloading::os::unix::Symbol<F> = libs.r(name);
        for &v in &[
            7u32, 8, 9, 100, 254, 255, 256, 257, 511, 512, 1000, 0x1_0000, 0x00FF_FF00,
            0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF,
        ] {
            for _ in 0..20 {
                let base = rng.addr();
                let mut ca = base;
                let mut ra = base;
                unsafe {
                    c(ca.as_mut_ptr(), v);
                    r(ra.as_mut_ptr(), v);
                }
                eq_bytes(
                    &format!("{name}({v:#x})"),
                    &u32s_to_bytes(&ca),
                    &u32s_to_bytes(&ra),
                );
                // The C casts to `unsigned char`, i.e. truncates mod 256.
                let off = match name {
                    "SPX_set_type" => OFF_TYPE,
                    "SPX_set_layer_addr" => OFF_LAYER,
                    "SPX_set_chain_addr" => OFF_CHAIN_ADDR,
                    "SPX_set_hash_addr" => OFF_HASH_ADDR,
                    _ => OFF_TREE_HGT,
                };
                assert_eq!(u32s_to_bytes(&ca)[off], (v & 0xFF) as u8);
            }
        }
    }
}

/// The out-of-range type must also propagate identically through the functions
/// that consume the address.
#[test]
fn row15_out_of_range_type_through_prf_and_thash() {
    let libs = Libs::load();
    type SetTypeFn = unsafe extern "C" fn(*mut u32, u32);
    type PrfFn = unsafe extern "C" fn(*mut u8, *const u8, *const u32);
    type ThashFn = unsafe extern "C" fn(*mut u8, *const u8, c_uint, *const u8, *mut u32);
    let (cset, rset) = pair!(libs, "SPX_set_type", SetTypeFn);
    let (cprf, rprf) = pair!(libs, "SPX_prf_addr", PrfFn);
    let (cth, rth) = pair!(libs, "SPX_thash", ThashFn);
    let mut rng = Rng::new(914);
    let ps = rng.bytes(SPX_N);
    let sk = rng.bytes(SPX_N);
    let (cc, rc) = init_ctx_pair(&libs, &ps, &sk);

    for &ty in &[7u32, 42, 255, 256, 259, 1000, 0xFFFF_FFFF] {
        for _ in 0..10 {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                cset(ca.as_mut_ptr(), ty);
                rset(ra.as_mut_ptr(), ty);
            }
            let mut cp = vec![0xAAu8; SPX_N + 8];
            let mut rp = vec![0xAAu8; SPX_N + 8];
            unsafe {
                cprf(cp.as_mut_ptr(), cc.as_ptr(), ca.as_ptr());
                rprf(rp.as_mut_ptr(), rc.as_ptr(), ra.as_ptr());
            }
            eq_bytes(&format!("prf_addr(type={ty})"), &cp, &rp);

            for &nb in &[1usize, 2, SPX_WOTS_LEN] {
                let inp = rng.bytes(nb * SPX_N);
                let mut ca2 = ca;
                let mut ra2 = ra;
                let mut co = vec![0xAAu8; SPX_N + 8];
                let mut ro = vec![0xAAu8; SPX_N + 8];
                unsafe {
                    cth(co.as_mut_ptr(), inp.as_ptr(), nb as c_uint, cc.as_ptr(), ca2.as_mut_ptr());
                    rth(ro.as_mut_ptr(), inp.as_ptr(), nb as c_uint, rc.as_ptr(), ra2.as_mut_ptr());
                }
                eq_bytes(&format!("thash(type={ty}, inblocks={nb})"), &co, &ro);
                eq_bytes(
                    &format!("thash(type={ty}) addr"),
                    &u32s_to_bytes(&ca2),
                    &u32s_to_bytes(&ra2),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17 / 18 / 19 — degenerate lengths in utils.c
// ---------------------------------------------------------------------------

#[test]
fn row17_ull_to_bytes_zero_outlen() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*mut u8, c_uint, u64);
    let (c, r) = pair!(libs, "SPX_ull_to_bytes", F);
    let mut rng = Rng::new(915);
    for i in 0..200 {
        let v = if i == 0 { 0 } else { rng.next_u64() };
        let mut cb = [0xAAu8; 16];
        let mut rb = [0xAAu8; 16];
        unsafe {
            c(cb.as_mut_ptr(), 0, v);
            r(rb.as_mut_ptr(), 0, v);
        }
        eq_bytes("ull_to_bytes(outlen=0)", &cb, &rb);
        // `for (i = (signed int)outlen - 1; i >= 0; i--)` never runs.
        assert!(cb.iter().all(|&b| b == 0xAA), "outlen=0 must write nothing");
    }
}

#[test]
fn row18_row19_bytes_to_ull_degenerate() {
    let libs = Libs::load();
    type F = unsafe extern "C" fn(*const u8, c_uint) -> u64;
    let (c, r) = pair!(libs, "SPX_bytes_to_ull", F);
    let mut rng = Rng::new(916);

    // Row 18: inlen == 0 returns 0.
    for _ in 0..50 {
        let buf = rng.bytes(16);
        let (cv, rv) = unsafe { (c(buf.as_ptr(), 0), r(buf.as_ptr(), 0)) };
        eq("bytes_to_ull(inlen=0)", cv, rv);
        eq("bytes_to_ull(inlen=0) == 0", cv, 0);
    }
    // Row 19: inlen > 8, where the C shift amount 8*(inlen-1-i) exceeds 63.
    for &inlen in &[9u32, 10, 12, 16, 24, 32] {
        for i in 0..80 {
            let buf: Vec<u8> = match i {
                0 => vec![0u8; inlen as usize],
                1 => vec![0xFFu8; inlen as usize],
                2 => (0..inlen).map(|k| k as u8).collect(),
                _ => rng.bytes(inlen as usize),
            };
            let (cv, rv) = unsafe { (c(buf.as_ptr(), inlen), r(buf.as_ptr(), inlen)) };
            eq(&format!("bytes_to_ull(inlen={inlen}, in={:02x?})", &buf), cv, rv);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20 — zero-length message on the signing path
// ---------------------------------------------------------------------------

#[test]
fn row20_zero_length_message() {
    let libs = Libs::load();
    let (ckp, rkp) = pair!(libs, "crypto_sign_seed_keypair", SeedKeypairFn);
    let (cinit, rinit) = pair!(libs, "randombytes_init", RandomBytesInitFn);
    let (cs, rs) = pair!(libs, "crypto_sign_signature", SignatureFn);
    let (cv, rv) = pair!(libs, "crypto_sign_verify", VerifyFn);
    let mut rng = Rng::new(917);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = vec![0u8; SPX_PK_BYTES];
    let mut sk2 = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
    }
    eq_bytes("row20 pk", &pk, &pk2);

    let mut entropy = [0u8; 48];
    for i in 0..48 {
        entropy[i] = (i as u8) ^ 0x33;
    }
    let mut ec = entropy;
    let mut er = entropy;
    if !RAND_URANDOM {
        unsafe {
            cinit(ec.as_mut_ptr(), std::ptr::null_mut());
            rinit(er.as_mut_ptr(), std::ptr::null_mut());
        }
    }

    let dummy = [0u8; 1];
    let mut c_sig = vec![0xAAu8; SPX_BYTES + 8];
    let mut r_sig = vec![0xAAu8; SPX_BYTES + 8];
    let mut cl = 0xDEADusize;
    let mut rl = 0xDEADusize;
    let (cr, rr) = unsafe {
        (
            cs(c_sig.as_mut_ptr(), &mut cl, dummy.as_ptr(), 0, sk.as_ptr()),
            rs(r_sig.as_mut_ptr(), &mut rl, dummy.as_ptr(), 0, sk.as_ptr()),
        )
    };
    eq("crypto_sign_signature(mlen=0) ret", cr, rr);
    eq("crypto_sign_signature(mlen=0) ret == 0", cr, 0);
    eq("crypto_sign_signature(mlen=0) siglen", cl, rl);
    eq("crypto_sign_signature(mlen=0) siglen == SPX_BYTES", cl, SPX_BYTES);
    if !RAND_URANDOM {
        eq_bytes("crypto_sign_signature(mlen=0) sig", &c_sig, &r_sig);
    }

    let (cvr, rvr) = unsafe {
        (
            cv(c_sig.as_ptr(), cl, dummy.as_ptr(), 0, pk.as_ptr()),
            rv(r_sig.as_ptr(), rl, dummy.as_ptr(), 0, pk.as_ptr()),
        )
    };
    eq("crypto_sign_verify(mlen=0) ret", cvr, rvr);
    eq("crypto_sign_verify(mlen=0) accepts", cvr, 0);
}
