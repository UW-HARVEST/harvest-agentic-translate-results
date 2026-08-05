//! Differential tests for the PUBLIC-KEY family:
//!   crypto_scalarmult (curve25519 / ed25519 / ristretto255 + frontend)
//!   crypto_sign_ed25519 (keypair/sign/open/detached/verify + sk_to_seed/pk + curve conv)
//!   crypto_box (curve25519xsalsa20poly1305 + curve25519xchacha20poly1305)
//!   crypto_kx
//!   crypto_core_ed25519 / crypto_core_ristretto255
//!
//! Every call goes through BOTH the C `.so` and the Rust `.so` via loaded
//! symbols; C is ground truth, and we assert return codes AND output buffers
//! match byte-for-byte. Keys are derived from fixed seeds so both libraries
//! produce identical material and we can compare deterministically.

#[macro_use]
mod common;
use common::{libs, Rng};

// ---- constants (from headers) ----
const SM_BYTES: usize = 32; // all scalarmult BYTES/SCALARBYTES == 32
const SIGN_BYTES: usize = 64;
const SIGN_SEEDBYTES: usize = 32;
const SIGN_PK: usize = 32;
const SIGN_SK: usize = 64;
const BOX_SEED: usize = 32;
const BOX_PK: usize = 32;
const BOX_SK: usize = 32;
const BOX_BEFORENM: usize = 32;
const BOX_NONCE: usize = 24;
const BOX_MAC: usize = 16;
const BOX_SEAL: usize = BOX_PK + BOX_MAC; // 48
const KX_PK: usize = 32;
const KX_SK: usize = 32;
const KX_SEED: usize = 32;
const KX_SESSION: usize = 32;
const CORE_BYTES: usize = 32;
const CORE_SCALAR: usize = 32;
const CORE_NONREDUCED: usize = 64;
const CORE_HASHBYTES: usize = 64;

// ---- fn-pointer type aliases ----
type Fqn = unsafe extern "C" fn(*mut u8, *const u8) -> i32; // (q, n)
type Fqnp = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32; // (q, n, p) or (r, p, q)
type Vss = unsafe extern "C" fn(*mut u8, *const u8); // void (out, in)
type Vsss = unsafe extern "C" fn(*mut u8, *const u8, *const u8); // void (z, x, y)
type Fis = unsafe extern "C" fn(*const u8) -> i32; // int (in)
type Fstring = unsafe extern "C" fn(*mut u8, *const u8, usize, *const u8, usize, i32) -> i32;

// ============================================================
// SCALARMULT — curve25519
// ============================================================

#[test]
fn scalarmult_curve25519_valid() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_curve25519", Fqnp);
    let (cb, rb) = sympair!(l, b"crypto_scalarmult_curve25519_base", Fqn);
    let mut rng = Rng::new(0x5ca1_ab1e);
    for _ in 0..400 {
        let n = rng.vec(SM_BYTES);
        // derive p as a base point of a random scalar so it's a real x25519 pubkey
        let m = rng.vec(SM_BYTES);
        let (mut cp, mut rp) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
        unsafe {
            let rcb = cb(cp.as_mut_ptr(), m.as_ptr());
            let rrb = rb(rp.as_mut_ptr(), m.as_ptr());
            assert_eq!(rcb, rrb, "curve25519_base rc");
            assert_eq!(cp, rp, "curve25519_base out");
        }
        let (mut cq, mut rq) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
        unsafe {
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), cp.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), rp.as_ptr());
            assert_eq!(rc, rr, "curve25519 rc");
            assert_eq!(cq, rq, "curve25519 out");
        }
    }
}

#[test]
fn scalarmult_frontend_valid() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult", Fqnp);
    let (cb, rb) = sympair!(l, b"crypto_scalarmult_base", Fqn);
    let mut rng = Rng::new(0xf00d_1234);
    for _ in 0..300 {
        let n = rng.vec(SM_BYTES);
        let m = rng.vec(SM_BYTES);
        let (mut cp, mut rp) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
        unsafe {
            assert_eq!(cb(cp.as_mut_ptr(), m.as_ptr()), rb(rp.as_mut_ptr(), m.as_ptr()));
            assert_eq!(cp, rp);
            let (mut cq, mut rq) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), cp.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), rp.as_ptr());
            assert_eq!(rc, rr);
            assert_eq!(cq, rq);
        }
    }
}

#[test]
fn scalarmult_curve25519_allzero_output_err() {
    // p of small order (all-zero point) -> output all zero -> both return -1.
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_curve25519", Fqnp);
    let mut rng = Rng::new(7);
    for _ in 0..50 {
        let n = rng.vec(SM_BYTES);
        let p = [0u8; SM_BYTES]; // small-order / identity -> zero result
        let (mut cq, mut rq) = ([0xAAu8; SM_BYTES], [0xAAu8; SM_BYTES]);
        unsafe {
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, -1, "C should reject all-zero output");
            assert_eq!(rc, rr, "curve25519 zero-output rc parity");
            assert_eq!(cq, rq, "zero-output buffer parity");
        }
    }
}

// ============================================================
// SCALARMULT — ed25519 (clamp + noclamp, base + base_noclamp)
// ============================================================

// helper: produce a valid ed25519 main-subgroup point via C base scalarmult
fn ed25519_valid_point(rng: &mut Rng) -> [u8; SM_BYTES] {
    let l = libs();
    let (cb, _rb) = sympair!(l, b"crypto_scalarmult_ed25519_base", Fqn);
    loop {
        let n = rng.vec(SM_BYTES);
        let mut p = [0u8; SM_BYTES];
        unsafe {
            if cb(p.as_mut_ptr(), n.as_ptr()) == 0 {
                return p;
            }
        }
    }
}

#[test]
fn scalarmult_ed25519_valid() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_ed25519", Fqnp);
    let (cnc, rnc) = sympair!(l, b"crypto_scalarmult_ed25519_noclamp", Fqnp);
    let mut rng = Rng::new(0xed25519);
    let mut oks = 0;
    for _ in 0..200 {
        let p = ed25519_valid_point(&mut rng);
        let n = rng.vec(SM_BYTES);
        unsafe {
            let (mut cq, mut rq) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ed25519 rc");
            assert_eq!(cq, rq, "ed25519 out");
            if rc == 0 {
                oks += 1;
            }
            // noclamp variant on same inputs
            let (mut cq2, mut rq2) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            let rc2 = cnc(cq2.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr2 = rnc(rq2.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc2, rr2, "ed25519 noclamp rc");
            assert_eq!(cq2, rq2, "ed25519 noclamp out");
        }
    }
    assert!(oks > 100, "expected many successful ed25519 scalarmults, got {oks}");
}

#[test]
fn scalarmult_ed25519_base_valid() {
    let l = libs();
    let (cb, rb) = sympair!(l, b"crypto_scalarmult_ed25519_base", Fqn);
    let (cbn, rbn) = sympair!(l, b"crypto_scalarmult_ed25519_base_noclamp", Fqn);
    let mut rng = Rng::new(0xba5e);
    for _ in 0..300 {
        let n = rng.vec(SM_BYTES);
        unsafe {
            let (mut cq, mut rq) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            assert_eq!(cb(cq.as_mut_ptr(), n.as_ptr()), rb(rq.as_mut_ptr(), n.as_ptr()));
            assert_eq!(cq, rq, "ed25519_base out");
            let (mut cq2, mut rq2) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            assert_eq!(cbn(cq2.as_mut_ptr(), n.as_ptr()), rbn(rq2.as_mut_ptr(), n.as_ptr()));
            assert_eq!(cq2, rq2, "ed25519_base_noclamp out");
        }
    }
}

#[test]
fn scalarmult_ed25519_invalid_point_err() {
    // random 32 bytes are almost never a valid ed25519 point -> -1 on both
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_ed25519", Fqnp);
    let mut rng = Rng::new(0xdead);
    let mut rejected = 0;
    for _ in 0..200 {
        let p = rng.vec(SM_BYTES);
        let n = rng.vec(SM_BYTES);
        unsafe {
            let (mut cq, mut rq) = ([1u8; SM_BYTES], [1u8; SM_BYTES]);
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ed25519 invalid-point rc parity");
            assert_eq!(cq, rq, "ed25519 invalid-point out parity");
            if rc == -1 {
                rejected += 1;
            }
        }
    }
    assert!(rejected > 150, "expected most random points rejected, got {rejected}");
}

#[test]
fn scalarmult_ed25519_zero_scalar_err() {
    // valid point but zero scalar -> is_inf/sodium_is_zero -> -1 on both
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_ed25519", Fqnp);
    let (cbn, rbn) = sympair!(l, b"crypto_scalarmult_ed25519_base_noclamp", Fqn);
    let mut rng = Rng::new(0);
    let p = ed25519_valid_point(&mut rng);
    let n = [0u8; SM_BYTES];
    unsafe {
        let (mut cq, mut rq) = ([9u8; SM_BYTES], [9u8; SM_BYTES]);
        let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
        let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
        assert_eq!(rc, -1);
        assert_eq!(rc, rr);
        assert_eq!(cq, rq);
        // base_noclamp with zero scalar -> -1
        let (mut cq2, mut rq2) = ([9u8; SM_BYTES], [9u8; SM_BYTES]);
        let rc2 = cbn(cq2.as_mut_ptr(), n.as_ptr());
        let rr2 = rbn(rq2.as_mut_ptr(), n.as_ptr());
        assert_eq!(rc2, -1);
        assert_eq!(rc2, rr2);
        assert_eq!(cq2, rq2);
    }
}

// ============================================================
// SCALARMULT — ristretto255
// ============================================================

fn ristretto_valid_point(rng: &mut Rng) -> [u8; SM_BYTES] {
    let l = libs();
    let (cb, _rb) = sympair!(l, b"crypto_scalarmult_ristretto255_base", Fqn);
    loop {
        let n = rng.vec(SM_BYTES);
        let mut p = [0u8; SM_BYTES];
        unsafe {
            if cb(p.as_mut_ptr(), n.as_ptr()) == 0 {
                return p;
            }
        }
    }
}

#[test]
fn scalarmult_ristretto255_valid() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_ristretto255", Fqnp);
    let (cb, rb) = sympair!(l, b"crypto_scalarmult_ristretto255_base", Fqn);
    let mut rng = Rng::new(0x715_7e77);
    for _ in 0..200 {
        let n = rng.vec(SM_BYTES);
        unsafe {
            let (mut cbp, mut rbp) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            assert_eq!(cb(cbp.as_mut_ptr(), n.as_ptr()), rb(rbp.as_mut_ptr(), n.as_ptr()));
            assert_eq!(cbp, rbp, "ristretto base out");
        }
        let p = ristretto_valid_point(&mut rng);
        unsafe {
            let (mut cq, mut rq) = ([0u8; SM_BYTES], [0u8; SM_BYTES]);
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto rc");
            assert_eq!(cq, rq, "ristretto out");
        }
    }
}

#[test]
fn scalarmult_ristretto255_invalid_point_err() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_scalarmult_ristretto255", Fqnp);
    let mut rng = Rng::new(0xbad_1257);
    let mut rejected = 0;
    for _ in 0..200 {
        let p = rng.vec(SM_BYTES);
        let n = rng.vec(SM_BYTES);
        unsafe {
            let (mut cq, mut rq) = ([3u8; SM_BYTES], [3u8; SM_BYTES]);
            let rc = cf(cq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            let rr = rf(rq.as_mut_ptr(), n.as_ptr(), p.as_ptr());
            assert_eq!(rc, rr, "ristretto invalid rc parity");
            assert_eq!(cq, rq, "ristretto invalid out parity");
            if rc == -1 {
                rejected += 1;
            }
        }
    }
    assert!(rejected > 150, "expected most random ristretto points rejected, got {rejected}");
}

// ============================================================
// SIGN — ed25519
// ============================================================

fn sign_keypair(seed: &[u8]) -> ([u8; SIGN_PK], [u8; SIGN_SK], [u8; SIGN_PK], [u8; SIGN_SK]) {
    let l = libs();
    let (ck, rk) = sympair!(
        l,
        b"crypto_sign_ed25519_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32
    );
    let (mut cpk, mut csk) = ([0u8; SIGN_PK], [0u8; SIGN_SK]);
    let (mut rpk, mut rsk) = ([0u8; SIGN_PK], [0u8; SIGN_SK]);
    unsafe {
        assert_eq!(ck(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!(rk(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()), 0);
    }
    (cpk, csk, rpk, rsk)
}

#[test]
fn sign_seed_keypair_deterministic() {
    let mut rng = Rng::new(0x519_0);
    for _ in 0..100 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, rpk, rsk) = sign_keypair(&seed);
        assert_eq!(cpk, rpk, "sign pk");
        assert_eq!(csk, rsk, "sign sk");
    }
}

#[test]
fn sign_and_open_roundtrip() {
    let l = libs();
    let (cs, rs) = sympair!(
        l,
        b"crypto_sign_ed25519",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let (co, ro) = sympair!(
        l,
        b"crypto_sign_ed25519_open",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_1);
    for _ in 0..200 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, rpk, rsk) = sign_keypair(&seed);
        assert_eq!(csk, rsk);
        let mlen = rng.range(300);
        let m = rng.vec(mlen);
        let (mut csm, mut rsm) = (vec![0u8; mlen + SIGN_BYTES], vec![0u8; mlen + SIGN_BYTES]);
        let (mut cslen, mut rslen) = (0u64, 0u64);
        unsafe {
            let rc = cs(csm.as_mut_ptr(), &mut cslen, m.as_ptr(), mlen as u64, csk.as_ptr());
            let rr = rs(rsm.as_mut_ptr(), &mut rslen, m.as_ptr(), mlen as u64, rsk.as_ptr());
            assert_eq!(rc, rr, "sign rc");
            assert_eq!(cslen, rslen, "sign smlen");
            assert_eq!(csm, rsm, "sign sm");
            // open with matching pk
            let (mut cm, mut rm) = (vec![0u8; mlen], vec![0u8; mlen]);
            let (mut cml, mut rml) = (0u64, 0u64);
            let orc = co(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), cslen, cpk.as_ptr());
            let orr = ro(rm.as_mut_ptr(), &mut rml, rsm.as_ptr(), rslen, rpk.as_ptr());
            assert_eq!(orc, orr, "open rc");
            assert_eq!(cml, rml, "open mlen");
            assert_eq!(cm, rm, "open m");
            assert_eq!(orc, 0);
            assert_eq!(&cm[..], &m[..], "recovered plaintext");
        }
    }
}

#[test]
fn sign_detached_and_verify() {
    let l = libs();
    let (cd, rd) = sympair!(
        l,
        b"crypto_sign_ed25519_detached",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let (cv, rv) = sympair!(
        l,
        b"crypto_sign_ed25519_verify_detached",
        unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_2);
    for _ in 0..200 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, rpk, rsk) = sign_keypair(&seed);
        let mlen = rng.range(256);
        let m = rng.vec(mlen);
        let (mut csig, mut rsig) = ([0u8; SIGN_BYTES], [0u8; SIGN_BYTES]);
        let (mut cl, mut rl) = (0u64, 0u64);
        unsafe {
            assert_eq!(
                cd(csig.as_mut_ptr(), &mut cl, m.as_ptr(), mlen as u64, csk.as_ptr()),
                rd(rsig.as_mut_ptr(), &mut rl, m.as_ptr(), mlen as u64, rsk.as_ptr())
            );
            assert_eq!(cl, rl, "sig len");
            assert_eq!(csig, rsig, "detached sig");
            // verify good
            let vc = cv(csig.as_ptr(), m.as_ptr(), mlen as u64, cpk.as_ptr());
            let vr = rv(rsig.as_ptr(), m.as_ptr(), mlen as u64, rpk.as_ptr());
            assert_eq!(vc, 0);
            assert_eq!(vc, vr, "verify good rc");
        }
    }
}

#[test]
fn sign_verify_bad_signature_err() {
    let l = libs();
    let (cd, _rd) = sympair!(
        l,
        b"crypto_sign_ed25519_detached",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let (cv, rv) = sympair!(
        l,
        b"crypto_sign_ed25519_verify_detached",
        unsafe extern "C" fn(*const u8, *const u8, u64, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_3);
    for _ in 0..150 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, rpk, _rsk) = sign_keypair(&seed);
        let mlen = rng.range(128) + 1;
        let m = rng.vec(mlen);
        let mut sig = [0u8; SIGN_BYTES];
        let mut sl = 0u64;
        unsafe {
            assert_eq!(cd(sig.as_mut_ptr(), &mut sl, m.as_ptr(), mlen as u64, csk.as_ptr()), 0);
        }
        // tamper one byte of the signature
        let idx = rng.range(SIGN_BYTES);
        sig[idx] ^= 0x01 << (rng.range(8));
        unsafe {
            let vc = cv(sig.as_ptr(), m.as_ptr(), mlen as u64, cpk.as_ptr());
            let vr = rv(sig.as_ptr(), m.as_ptr(), mlen as u64, rpk.as_ptr());
            assert_eq!(vc, -1, "tampered sig must fail on C");
            assert_eq!(vc, vr, "tampered sig rc parity");
        }
        // also tamper the message instead
        let mut m2 = m.clone();
        let mi = rng.range(mlen);
        m2[mi] ^= 0x80;
        let mut sig2 = [0u8; SIGN_BYTES];
        let mut sl2 = 0u64;
        unsafe {
            assert_eq!(cd(sig2.as_mut_ptr(), &mut sl2, m.as_ptr(), mlen as u64, csk.as_ptr()), 0);
            let vc = cv(sig2.as_ptr(), m2.as_ptr(), mlen as u64, cpk.as_ptr());
            let vr = rv(sig2.as_ptr(), m2.as_ptr(), mlen as u64, rpk.as_ptr());
            assert_eq!(vc, -1);
            assert_eq!(vc, vr);
        }
    }
}

#[test]
fn sign_open_bad_and_short_err() {
    let l = libs();
    let (co, ro) = sympair!(
        l,
        b"crypto_sign_ed25519_open",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let (cs, _rs) = sympair!(
        l,
        b"crypto_sign_ed25519",
        unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_4);
    for _ in 0..100 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, rpk, _rsk) = sign_keypair(&seed);
        let mlen = rng.range(100) + 1;
        let m = rng.vec(mlen);
        let mut sm = vec![0u8; mlen + SIGN_BYTES];
        let mut sl = 0u64;
        unsafe {
            assert_eq!(cs(sm.as_mut_ptr(), &mut sl, m.as_ptr(), mlen as u64, csk.as_ptr()), 0);
        }
        // tamper signed message
        let ti = rng.range(sm.len());
        sm[ti] ^= 0x11;
        let (mut cm, mut rm) = (vec![9u8; mlen], vec![9u8; mlen]);
        let (mut cl, mut rl) = (123u64, 123u64);
        unsafe {
            let rc = co(cm.as_mut_ptr(), &mut cl, sm.as_ptr(), sl, cpk.as_ptr());
            let rr = ro(rm.as_mut_ptr(), &mut rl, sm.as_ptr(), sl, rpk.as_ptr());
            assert_eq!(rc, -1, "tampered open must fail");
            assert_eq!(rc, rr, "tampered open rc parity");
            assert_eq!(cl, rl, "open mlen parity (0)");
            assert_eq!(cm, rm, "open buffer zeroed parity");
        }
        // too-short signed message (< 64) -> -1
        let shl = rng.range(63);
        let short = rng.vec(shl);
        let slen = short.len() as u64;
        let (mut cm2, mut rm2) = (vec![0u8; 64], vec![0u8; 64]);
        let (mut cl2, mut rl2) = (7u64, 7u64);
        unsafe {
            let rc = co(cm2.as_mut_ptr(), &mut cl2, short.as_ptr(), slen, cpk.as_ptr());
            let rr = ro(rm2.as_mut_ptr(), &mut rl2, short.as_ptr(), slen, rpk.as_ptr());
            assert_eq!(rc, -1);
            assert_eq!(rc, rr);
            assert_eq!(cl2, rl2);
        }
    }
}

#[test]
fn sign_sk_to_seed_and_pk() {
    let l = libs();
    let (cts, rts) = sympair!(
        l,
        b"crypto_sign_ed25519_sk_to_seed",
        unsafe extern "C" fn(*mut u8, *const u8) -> i32
    );
    let (ctp, rtp) = sympair!(
        l,
        b"crypto_sign_ed25519_sk_to_pk",
        unsafe extern "C" fn(*mut u8, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_5);
    for _ in 0..100 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (cpk, csk, _rpk, rsk) = sign_keypair(&seed);
        assert_eq!(csk, rsk);
        unsafe {
            let (mut cse, mut rse) = ([0u8; SIGN_SEEDBYTES], [0u8; SIGN_SEEDBYTES]);
            assert_eq!(cts(cse.as_mut_ptr(), csk.as_ptr()), rts(rse.as_mut_ptr(), rsk.as_ptr()));
            assert_eq!(cse, rse, "sk_to_seed");
            assert_eq!(&cse[..], &seed[..], "recovered seed");
            let (mut cpk2, mut rpk2) = ([0u8; SIGN_PK], [0u8; SIGN_PK]);
            assert_eq!(ctp(cpk2.as_mut_ptr(), csk.as_ptr()), rtp(rpk2.as_mut_ptr(), rsk.as_ptr()));
            assert_eq!(cpk2, rpk2, "sk_to_pk");
            assert_eq!(cpk2, cpk, "recovered pk matches keypair");
        }
    }
}

#[test]
fn sign_sk_pk_to_curve25519() {
    let l = libs();
    let (csk, rsk) = sympair!(
        l,
        b"crypto_sign_ed25519_sk_to_curve25519",
        unsafe extern "C" fn(*mut u8, *const u8) -> i32
    );
    let (cpk, rpk) = sympair!(
        l,
        b"crypto_sign_ed25519_pk_to_curve25519",
        unsafe extern "C" fn(*mut u8, *const u8) -> i32
    );
    let mut rng = Rng::new(0x519_6);
    for _ in 0..100 {
        let seed = rng.vec(SIGN_SEEDBYTES);
        let (ed_pk, ed_sk, _rpk, ed_rsk) = sign_keypair(&seed);
        assert_eq!(ed_sk, ed_rsk);
        unsafe {
            let (mut ccsk, mut rcsk) = ([0u8; 32], [0u8; 32]);
            assert_eq!(csk(ccsk.as_mut_ptr(), ed_sk.as_ptr()), rsk(rcsk.as_mut_ptr(), ed_rsk.as_ptr()));
            assert_eq!(ccsk, rcsk, "sk_to_curve25519");
            let (mut ccpk, mut rcpk) = ([0u8; 32], [0u8; 32]);
            let rc = cpk(ccpk.as_mut_ptr(), ed_pk.as_ptr());
            let rr = rpk(rcpk.as_mut_ptr(), ed_pk.as_ptr());
            assert_eq!(rc, rr, "pk_to_curve25519 rc");
            assert_eq!(ccpk, rcpk, "pk_to_curve25519 out");
        }
    }
}

// ============================================================
// BOX — both primitives
// ============================================================

struct BoxApi {
    seed_keypair: &'static [u8],
    beforenm: &'static [u8],
    easy: &'static [u8],
    open_easy: &'static [u8],
    detached: &'static [u8],
    open_detached: &'static [u8],
    easy_afternm: &'static [u8],
    open_easy_afternm: &'static [u8],
    seal: &'static [u8],
    seal_open: &'static [u8],
}

fn box_keypair(seed_sym: &[u8], seed: &[u8]) -> ([u8; BOX_PK], [u8; BOX_SK], [u8; BOX_PK], [u8; BOX_SK]) {
    let l = libs();
    let (ck, rk) = sympair!(
        l,
        seed_sym,
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32
    );
    let (mut cpk, mut csk) = ([0u8; BOX_PK], [0u8; BOX_SK]);
    let (mut rpk, mut rsk) = ([0u8; BOX_PK], [0u8; BOX_SK]);
    unsafe {
        assert_eq!(ck(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()), 0);
        assert_eq!(rk(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()), 0);
    }
    (cpk, csk, rpk, rsk)
}

fn run_box_family(api: &BoxApi, base_seed: u64) {
    let l = libs();
    let mut rng = Rng::new(base_seed);
    let (cbe, rbe) = sympair!(
        l,
        api.beforenm,
        unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32
    );
    let (ce, re) = sympair!(
        l,
        api.easy,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32
    );
    let (coe, roe) = sympair!(
        l,
        api.open_easy,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32
    );
    let (cd, rd) = sympair!(
        l,
        api.detached,
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32
    );
    let (cod, rod) = sympair!(
        l,
        api.open_detached,
        unsafe extern "C" fn(*mut u8, *const u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32
    );
    let (cea, rea) = sympair!(
        l,
        api.easy_afternm,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32
    );
    let (coea, roea) = sympair!(
        l,
        api.open_easy_afternm,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32
    );
    let (cseal, rseal) = sympair!(
        l,
        api.seal,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8) -> i32
    );
    let (cso, rso) = sympair!(
        l,
        api.seal_open,
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8) -> i32
    );

    for _ in 0..150 {
        let sseed = rng.vec(BOX_SEED);
        let rseed = rng.vec(BOX_SEED);
        let (spk_c, ssk_c, spk_r, ssk_r) = box_keypair(api.seed_keypair, &sseed);
        let (rpk_c, rsk_c, rpk_r, rsk_r) = box_keypair(api.seed_keypair, &rseed);
        assert_eq!(spk_c, spk_r, "box sender pk");
        assert_eq!(ssk_c, ssk_r, "box sender sk");
        assert_eq!(rpk_c, rpk_r, "box recip pk");

        let nonce = rng.vec(BOX_NONCE);
        let mlen = rng.range(256);
        let m = rng.vec(mlen);

        // beforenm shared key parity
        let (mut ck, mut rk) = ([0u8; BOX_BEFORENM], [0u8; BOX_BEFORENM]);
        unsafe {
            let rcbn = cbe(ck.as_mut_ptr(), rpk_c.as_ptr(), ssk_c.as_ptr());
            let rrbn = rbe(rk.as_mut_ptr(), rpk_r.as_ptr(), ssk_r.as_ptr());
            assert_eq!(rcbn, rrbn, "beforenm rc");
            assert_eq!(ck, rk, "beforenm key");
        }

        // box_easy / open_easy
        let clen = mlen + BOX_MAC;
        let (mut cc, mut rc_) = (vec![0u8; clen], vec![0u8; clen]);
        unsafe {
            let a = ce(cc.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), rpk_c.as_ptr(), ssk_c.as_ptr());
            let b = re(rc_.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), rpk_r.as_ptr(), ssk_r.as_ptr());
            assert_eq!(a, b, "box_easy rc");
            assert_eq!(cc, rc_, "box_easy ct");
            // open with recipient's sk and sender pk
            let (mut cm, mut rm) = (vec![0u8; mlen], vec![0u8; mlen]);
            let oa = coe(cm.as_mut_ptr(), cc.as_ptr(), clen as u64, nonce.as_ptr(), spk_c.as_ptr(), rsk_c.as_ptr());
            let ob = roe(rm.as_mut_ptr(), rc_.as_ptr(), clen as u64, nonce.as_ptr(), spk_r.as_ptr(), rsk_r.as_ptr());
            assert_eq!(oa, ob, "open_easy rc");
            assert_eq!(oa, 0);
            assert_eq!(cm, rm, "open_easy pt");
            assert_eq!(&cm[..], &m[..], "box roundtrip plaintext");

            // tampered ciphertext -> -1 on both
            if clen > 0 {
                let mut ct = cc.clone();
                let ti = rng.range(clen);
                ct[ti] ^= 0x40;
                let (mut cm2, mut rm2) = (vec![5u8; mlen], vec![5u8; mlen]);
                let ta = coe(cm2.as_mut_ptr(), ct.as_ptr(), clen as u64, nonce.as_ptr(), spk_c.as_ptr(), rsk_c.as_ptr());
                let tb = roe(rm2.as_mut_ptr(), ct.as_ptr(), clen as u64, nonce.as_ptr(), spk_r.as_ptr(), rsk_r.as_ptr());
                assert_eq!(ta, -1, "tampered box must fail");
                assert_eq!(ta, tb, "tampered box rc parity");
            }
        }

        // easy_afternm / open_easy_afternm using shared key ck
        unsafe {
            let (mut cc2, mut rc2) = (vec![0u8; clen], vec![0u8; clen]);
            let a = cea(cc2.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), ck.as_ptr());
            let b = rea(rc2.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), rk.as_ptr());
            assert_eq!(a, b, "easy_afternm rc");
            assert_eq!(cc2, rc2, "easy_afternm ct");
            let (mut cm, mut rm) = (vec![0u8; mlen], vec![0u8; mlen]);
            let oa = coea(cm.as_mut_ptr(), cc2.as_ptr(), clen as u64, nonce.as_ptr(), ck.as_ptr());
            let ob = roea(rm.as_mut_ptr(), rc2.as_ptr(), clen as u64, nonce.as_ptr(), rk.as_ptr());
            assert_eq!(oa, ob, "open_easy_afternm rc");
            assert_eq!(cm, rm, "open_easy_afternm pt");
        }

        // detached / open_detached
        unsafe {
            let (mut cc3, mut rc3) = (vec![0u8; mlen], vec![0u8; mlen]);
            let (mut cmac, mut rmac) = ([0u8; BOX_MAC], [0u8; BOX_MAC]);
            let a = cd(cc3.as_mut_ptr(), cmac.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), rpk_c.as_ptr(), ssk_c.as_ptr());
            let b = rd(rc3.as_mut_ptr(), rmac.as_mut_ptr(), m.as_ptr(), mlen as u64, nonce.as_ptr(), rpk_r.as_ptr(), ssk_r.as_ptr());
            assert_eq!(a, b, "detached rc");
            assert_eq!(cc3, rc3, "detached ct");
            assert_eq!(cmac, rmac, "detached mac");
            let (mut cm, mut rm) = (vec![0u8; mlen], vec![0u8; mlen]);
            let oa = cod(cm.as_mut_ptr(), cc3.as_ptr(), cmac.as_ptr(), mlen as u64, nonce.as_ptr(), spk_c.as_ptr(), rsk_c.as_ptr());
            let ob = rod(rm.as_mut_ptr(), rc3.as_ptr(), rmac.as_ptr(), mlen as u64, nonce.as_ptr(), spk_r.as_ptr(), rsk_r.as_ptr());
            assert_eq!(oa, ob, "open_detached rc");
            assert_eq!(oa, 0);
            assert_eq!(cm, rm, "open_detached pt");
            assert_eq!(&cm[..], &m[..]);
        }

        // seal / seal_open (anonymous, to recipient pk)
        unsafe {
            let sealen = mlen + BOX_SEAL;
            let (mut csc, mut rsc) = (vec![0u8; sealen], vec![0u8; sealen]);
            // seal uses ephemeral randomness -> ciphertext differs; just check rc + roundtrip
            let a = cseal(csc.as_mut_ptr(), m.as_ptr(), mlen as u64, rpk_c.as_ptr());
            let b = rseal(rsc.as_mut_ptr(), m.as_ptr(), mlen as u64, rpk_r.as_ptr());
            assert_eq!(a, b, "seal rc");
            assert_eq!(a, 0);
            // open each library's own sealed box with the recipient keypair
            let (mut cm, mut rm) = (vec![0u8; mlen], vec![0u8; mlen]);
            let oa = cso(cm.as_mut_ptr(), csc.as_ptr(), sealen as u64, rpk_c.as_ptr(), rsk_c.as_ptr());
            let ob = rso(rm.as_mut_ptr(), rsc.as_ptr(), sealen as u64, rpk_r.as_ptr(), rsk_r.as_ptr());
            assert_eq!(oa, ob, "seal_open rc");
            assert_eq!(oa, 0);
            assert_eq!(&cm[..], &m[..], "C seal roundtrip");
            assert_eq!(&rm[..], &m[..], "Rust seal roundtrip");
            // cross-open: open Rust's sealed box with C library must also work (interop)
            let mut xm = vec![0u8; mlen];
            let xa = cso(xm.as_mut_ptr(), rsc.as_ptr(), sealen as u64, rpk_c.as_ptr(), rsk_c.as_ptr());
            assert_eq!(xa, 0, "C opens Rust seal");
            assert_eq!(&xm[..], &m[..]);
        }
    }
}

#[test]
fn box_xsalsa20poly1305_family() {
    let api = BoxApi {
        seed_keypair: b"crypto_box_curve25519xsalsa20poly1305_seed_keypair",
        beforenm: b"crypto_box_beforenm",
        easy: b"crypto_box_easy",
        open_easy: b"crypto_box_open_easy",
        detached: b"crypto_box_detached",
        open_detached: b"crypto_box_open_detached",
        easy_afternm: b"crypto_box_easy_afternm",
        open_easy_afternm: b"crypto_box_open_easy_afternm",
        seal: b"crypto_box_seal",
        seal_open: b"crypto_box_seal_open",
    };
    run_box_family(&api, 0xb0_5a15a);
}

#[test]
fn box_xchacha20poly1305_family() {
    let api = BoxApi {
        seed_keypair: b"crypto_box_curve25519xchacha20poly1305_seed_keypair",
        beforenm: b"crypto_box_curve25519xchacha20poly1305_beforenm",
        easy: b"crypto_box_curve25519xchacha20poly1305_easy",
        open_easy: b"crypto_box_curve25519xchacha20poly1305_open_easy",
        detached: b"crypto_box_curve25519xchacha20poly1305_detached",
        open_detached: b"crypto_box_curve25519xchacha20poly1305_open_detached",
        easy_afternm: b"crypto_box_curve25519xchacha20poly1305_easy_afternm",
        open_easy_afternm: b"crypto_box_curve25519xchacha20poly1305_open_easy_afternm",
        seal: b"crypto_box_curve25519xchacha20poly1305_seal",
        seal_open: b"crypto_box_curve25519xchacha20poly1305_seal_open",
    };
    run_box_family(&api, 0xc4_1c4a);
}

#[test]
fn box_open_easy_too_short_err() {
    let l = libs();
    let (coe, roe) = sympair!(
        l,
        b"crypto_box_open_easy",
        unsafe extern "C" fn(*mut u8, *const u8, u64, *const u8, *const u8, *const u8) -> i32
    );
    let mut rng = Rng::new(0x5417);
    // sender + recipient keypairs (identical across both libs from same seeds)
    let (spk, _ssk_c, _spk_r, _ssk_r) = box_keypair(b"crypto_box_curve25519xsalsa20poly1305_seed_keypair", &rng.vec(BOX_SEED));
    let (_rpk, rsk_c, _rpk_r, rsk_r) = box_keypair(b"crypto_box_curve25519xsalsa20poly1305_seed_keypair", &rng.vec(BOX_SEED));
    let nonce = rng.vec(BOX_NONCE);
    for clen in 0..BOX_MAC {
        let c = rng.vec(clen);
        let (mut cm, mut rm) = (vec![0u8; 8], vec![0u8; 8]);
        unsafe {
            let a = coe(cm.as_mut_ptr(), c.as_ptr(), clen as u64, nonce.as_ptr(), spk.as_ptr(), rsk_c.as_ptr());
            let b = roe(rm.as_mut_ptr(), c.as_ptr(), clen as u64, nonce.as_ptr(), spk.as_ptr(), rsk_r.as_ptr());
            assert_eq!(a, -1, "short ciphertext must fail");
            assert_eq!(a, b, "short ciphertext rc parity");
        }
    }
}

// ============================================================
// KX
// ============================================================

#[test]
fn kx_seed_keypair_and_sessions() {
    let l = libs();
    let (ck, rk) = sympair!(
        l,
        b"crypto_kx_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32
    );
    let (ccs, rcs) = sympair!(
        l,
        b"crypto_kx_client_session_keys",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32
    );
    let (css, rss) = sympair!(
        l,
        b"crypto_kx_server_session_keys",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32
    );
    let mut rng = Rng::new(0x4b_78);
    for _ in 0..150 {
        let cseed = rng.vec(KX_SEED);
        let sseed = rng.vec(KX_SEED);
        // client keypair
        let (mut c_cpk, mut c_csk) = ([0u8; KX_PK], [0u8; KX_SK]);
        let (mut r_cpk, mut r_csk) = ([0u8; KX_PK], [0u8; KX_SK]);
        // server keypair
        let (mut c_spk, mut c_ssk) = ([0u8; KX_PK], [0u8; KX_SK]);
        let (mut r_spk, mut r_ssk) = ([0u8; KX_PK], [0u8; KX_SK]);
        unsafe {
            assert_eq!(ck(c_cpk.as_mut_ptr(), c_csk.as_mut_ptr(), cseed.as_ptr()), 0);
            assert_eq!(rk(r_cpk.as_mut_ptr(), r_csk.as_mut_ptr(), cseed.as_ptr()), 0);
            assert_eq!(ck(c_spk.as_mut_ptr(), c_ssk.as_mut_ptr(), sseed.as_ptr()), 0);
            assert_eq!(rk(r_spk.as_mut_ptr(), r_ssk.as_mut_ptr(), sseed.as_ptr()), 0);
        }
        assert_eq!(c_cpk, r_cpk, "kx client pk");
        assert_eq!(c_csk, r_csk, "kx client sk");
        assert_eq!(c_spk, r_spk, "kx server pk");
        assert_eq!(c_ssk, r_ssk, "kx server sk");

        // client session keys
        let (mut c_rx, mut c_tx) = ([0u8; KX_SESSION], [0u8; KX_SESSION]);
        let (mut r_rx, mut r_tx) = ([0u8; KX_SESSION], [0u8; KX_SESSION]);
        unsafe {
            let a = ccs(c_rx.as_mut_ptr(), c_tx.as_mut_ptr(), c_cpk.as_ptr(), c_csk.as_ptr(), c_spk.as_ptr());
            let b = rcs(r_rx.as_mut_ptr(), r_tx.as_mut_ptr(), r_cpk.as_ptr(), r_csk.as_ptr(), r_spk.as_ptr());
            assert_eq!(a, b, "client_session rc");
            assert_eq!(a, 0);
            assert_eq!(c_rx, r_rx, "client rx");
            assert_eq!(c_tx, r_tx, "client tx");
        }
        // server session keys
        let (mut c_srx, mut c_stx) = ([0u8; KX_SESSION], [0u8; KX_SESSION]);
        let (mut r_srx, mut r_stx) = ([0u8; KX_SESSION], [0u8; KX_SESSION]);
        unsafe {
            let a = css(c_srx.as_mut_ptr(), c_stx.as_mut_ptr(), c_spk.as_ptr(), c_ssk.as_ptr(), c_cpk.as_ptr());
            let b = rss(r_srx.as_mut_ptr(), r_stx.as_mut_ptr(), r_spk.as_ptr(), r_ssk.as_ptr(), r_cpk.as_ptr());
            assert_eq!(a, b, "server_session rc");
            assert_eq!(a, 0);
            assert_eq!(c_srx, r_srx, "server rx");
            assert_eq!(c_stx, r_stx, "server tx");
        }
        // protocol property: client.rx == server.tx and client.tx == server.rx
        assert_eq!(c_rx, c_stx, "client rx == server tx");
        assert_eq!(c_tx, c_srx, "client tx == server rx");
    }
}

#[test]
fn kx_session_keys_bad_pubkey_err() {
    // A zero/small-order server pubkey makes scalarmult yield zero -> -1 on both.
    let l = libs();
    let (ck, rk) = sympair!(
        l,
        b"crypto_kx_seed_keypair",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32
    );
    let (ccs, rcs) = sympair!(
        l,
        b"crypto_kx_client_session_keys",
        unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8, *const u8) -> i32
    );
    let mut rng = Rng::new(0x4b_99);
    let cseed = rng.vec(KX_SEED);
    let (mut c_cpk, mut c_csk) = ([0u8; KX_PK], [0u8; KX_SK]);
    let (mut r_cpk, mut r_csk) = ([0u8; KX_PK], [0u8; KX_SK]);
    unsafe {
        assert_eq!(ck(c_cpk.as_mut_ptr(), c_csk.as_mut_ptr(), cseed.as_ptr()), 0);
        assert_eq!(rk(r_cpk.as_mut_ptr(), r_csk.as_mut_ptr(), cseed.as_ptr()), 0);
    }
    let bad_spk = [0u8; KX_PK]; // zero pubkey -> zero shared secret
    let (mut c_rx, mut c_tx) = ([1u8; KX_SESSION], [1u8; KX_SESSION]);
    let (mut r_rx, mut r_tx) = ([1u8; KX_SESSION], [1u8; KX_SESSION]);
    unsafe {
        let a = ccs(c_rx.as_mut_ptr(), c_tx.as_mut_ptr(), c_cpk.as_ptr(), c_csk.as_ptr(), bad_spk.as_ptr());
        let b = rcs(r_rx.as_mut_ptr(), r_tx.as_mut_ptr(), r_cpk.as_ptr(), r_csk.as_ptr(), bad_spk.as_ptr());
        assert_eq!(a, -1, "bad server pk must fail");
        assert_eq!(a, b, "kx bad pk rc parity");
    }
}

// ============================================================
// CORE — ed25519
// ============================================================

#[test]
fn core_ed25519_is_valid_point() {
    let l = libs();
    let (cv, rv) = sympair!(l, b"crypto_core_ed25519_is_valid_point", Fis);
    let mut rng = Rng::new(0x0e_d1);
    // valid points
    for _ in 0..100 {
        let p = ed25519_valid_point(&mut rng);
        unsafe {
            let a = cv(p.as_ptr());
            let b = rv(p.as_ptr());
            assert_eq!(a, 1, "valid point should be 1");
            assert_eq!(a, b, "is_valid parity (valid)");
        }
    }
    // random / invalid points
    let mut zeros = 0;
    for _ in 0..200 {
        let p = rng.vec(CORE_BYTES);
        unsafe {
            let a = cv(p.as_ptr());
            let b = rv(p.as_ptr());
            assert_eq!(a, b, "is_valid parity (random)");
            if a == 0 {
                zeros += 1;
            }
        }
    }
    assert!(zeros > 150, "most random points invalid, got {zeros}");
}

#[test]
fn core_ed25519_add_sub() {
    let l = libs();
    let (ca, ra) = sympair!(l, b"crypto_core_ed25519_add", Fqnp);
    let (cs, rs) = sympair!(l, b"crypto_core_ed25519_sub", Fqnp);
    let mut rng = Rng::new(0x0e_d2);
    for _ in 0..150 {
        let p = ed25519_valid_point(&mut rng);
        let q = ed25519_valid_point(&mut rng);
        unsafe {
            let (mut cr, mut rr) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
            let a = ca(cr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            let b = ra(rr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            assert_eq!(a, b, "add rc");
            assert_eq!(a, 0);
            assert_eq!(cr, rr, "add out");
            let (mut cr2, mut rr2) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
            let a2 = cs(cr2.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            let b2 = rs(rr2.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            assert_eq!(a2, b2, "sub rc");
            assert_eq!(cr2, rr2, "sub out");
        }
    }
    // invalid inputs -> -1 on both
    let mut rejected = 0;
    for _ in 0..100 {
        let p = rng.vec(CORE_BYTES);
        let q = rng.vec(CORE_BYTES);
        unsafe {
            let (mut cr, mut rr) = ([7u8; CORE_BYTES], [7u8; CORE_BYTES]);
            let a = ca(cr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            let b = ra(rr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            assert_eq!(a, b, "add invalid rc parity");
            assert_eq!(cr, rr, "add invalid out parity");
            if a == -1 {
                rejected += 1;
            }
        }
    }
    assert!(rejected > 60, "expected many invalid add rejects, got {rejected}");
}

#[test]
fn core_ed25519_scalar_ops() {
    let l = libs();
    let (cadd, radd) = sympair!(l, b"crypto_core_ed25519_scalar_add", Vsss);
    let (csub, rsub) = sympair!(l, b"crypto_core_ed25519_scalar_sub", Vsss);
    let (cmul, rmul) = sympair!(l, b"crypto_core_ed25519_scalar_mul", Vsss);
    let (cneg, rneg) = sympair!(l, b"crypto_core_ed25519_scalar_negate", Vss);
    let (ccmp, rcmp) = sympair!(l, b"crypto_core_ed25519_scalar_complement", Vss);
    let (cred, rred) = sympair!(l, b"crypto_core_ed25519_scalar_reduce", Vss);
    let (cinv, rinv) = sympair!(l, b"crypto_core_ed25519_scalar_invert", unsafe extern "C" fn(*mut u8, *const u8) -> i32);
    let (ccan, rcan) = sympair!(l, b"crypto_core_ed25519_scalar_is_canonical", Fis);
    let mut rng = Rng::new(0x0e_d3);
    for _ in 0..300 {
        let x = rng.vec(CORE_SCALAR);
        let y = rng.vec(CORE_SCALAR);
        unsafe {
            for (cf, rf) in [(&cadd, &radd), (&csub, &rsub), (&cmul, &rmul)] {
                let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
                cf(cz.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                rf(rz.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                assert_eq!(cz, rz, "scalar binop parity");
            }
            for (cf, rf) in [(&cneg, &rneg), (&ccmp, &rcmp)] {
                let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
                cf(cz.as_mut_ptr(), x.as_ptr());
                rf(rz.as_mut_ptr(), x.as_ptr());
                assert_eq!(cz, rz, "scalar unop parity");
            }
            // invert (nonzero x)
            let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
            let a = cinv(cz.as_mut_ptr(), x.as_ptr());
            let b = rinv(rz.as_mut_ptr(), x.as_ptr());
            assert_eq!(a, b, "scalar_invert rc");
            assert_eq!(cz, rz, "scalar_invert out");
            // is_canonical
            assert_eq!(ccan(x.as_ptr()), rcan(x.as_ptr()), "scalar_is_canonical");
            // reduce (64-byte input)
            let big = rng.vec(CORE_NONREDUCED);
            let (mut cr, mut rr) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
            cred(cr.as_mut_ptr(), big.as_ptr());
            rred(rr.as_mut_ptr(), big.as_ptr());
            assert_eq!(cr, rr, "scalar_reduce parity");
        }
    }
    // invert of zero -> -1 on both
    let zero = [0u8; CORE_SCALAR];
    unsafe {
        let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
        let a = cinv(cz.as_mut_ptr(), zero.as_ptr());
        let b = rinv(rz.as_mut_ptr(), zero.as_ptr());
        assert_eq!(a, -1, "invert(0) -> -1");
        assert_eq!(a, b);
        assert_eq!(cz, rz);
    }
}

#[test]
fn core_ed25519_from_string() {
    // hash-to-curve (exercises internal from_uniform/from_hash), deterministic
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_core_ed25519_from_string", Fstring);
    let (csf, rsf) = sympair!(l, b"crypto_core_ed25519_scalar_from_string", Fstring);
    let mut rng = Rng::new(0x0e_d4);
    for alg in [1i32, 2i32] {
        // 1 = SHA256, 2 = SHA512
        for _ in 0..80 {
            let cl = rng.range(20);
            let ctx = rng.vec(cl);
            let ml = rng.range(64);
            let msg = rng.vec(ml);
            unsafe {
                let (mut cp, mut rp) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
                let a = cf(cp.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
                let b = rf(rp.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
                assert_eq!(a, b, "from_string rc");
                assert_eq!(cp, rp, "from_string out");
                let (mut cs2, mut rs2) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
                let a2 = csf(cs2.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
                let b2 = rsf(rs2.as_mut_ptr(), ctx.as_ptr(), ctx.len(), msg.as_ptr(), msg.len(), alg);
                assert_eq!(a2, b2, "scalar_from_string rc");
                assert_eq!(cs2, rs2, "scalar_from_string out");
            }
        }
    }
}

// ============================================================
// CORE — ristretto255
// ============================================================

#[test]
fn core_ristretto255_is_valid_point() {
    let l = libs();
    let (cv, rv) = sympair!(l, b"crypto_core_ristretto255_is_valid_point", Fis);
    let mut rng = Rng::new(0x1517_0);
    for _ in 0..100 {
        let p = ristretto_valid_point(&mut rng);
        unsafe {
            assert_eq!(cv(p.as_ptr()), 1);
            assert_eq!(cv(p.as_ptr()), rv(p.as_ptr()));
        }
    }
    let mut zeros = 0;
    for _ in 0..200 {
        let p = rng.vec(CORE_BYTES);
        unsafe {
            let a = cv(p.as_ptr());
            assert_eq!(a, rv(p.as_ptr()), "ristretto is_valid parity");
            if a == 0 {
                zeros += 1;
            }
        }
    }
    assert!(zeros > 150, "most random ristretto invalid, got {zeros}");
}

#[test]
fn core_ristretto255_add_sub() {
    let l = libs();
    let (ca, ra) = sympair!(l, b"crypto_core_ristretto255_add", Fqnp);
    let (cs, rs) = sympair!(l, b"crypto_core_ristretto255_sub", Fqnp);
    let mut rng = Rng::new(0x1517_1);
    for _ in 0..120 {
        let p = ristretto_valid_point(&mut rng);
        let q = ristretto_valid_point(&mut rng);
        unsafe {
            let (mut cr, mut rr) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
            assert_eq!(ca(cr.as_mut_ptr(), p.as_ptr(), q.as_ptr()), ra(rr.as_mut_ptr(), p.as_ptr(), q.as_ptr()));
            assert_eq!(cr, rr, "ristretto add out");
            let (mut cr2, mut rr2) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
            assert_eq!(cs(cr2.as_mut_ptr(), p.as_ptr(), q.as_ptr()), rs(rr2.as_mut_ptr(), p.as_ptr(), q.as_ptr()));
            assert_eq!(cr2, rr2, "ristretto sub out");
        }
    }
    // invalid -> -1 both
    let mut rejected = 0;
    for _ in 0..100 {
        let p = rng.vec(CORE_BYTES);
        let q = rng.vec(CORE_BYTES);
        unsafe {
            let (mut cr, mut rr) = ([8u8; CORE_BYTES], [8u8; CORE_BYTES]);
            let a = ca(cr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            let b = ra(rr.as_mut_ptr(), p.as_ptr(), q.as_ptr());
            assert_eq!(a, b, "ristretto add invalid parity");
            assert_eq!(cr, rr);
            if a == -1 {
                rejected += 1;
            }
        }
    }
    assert!(rejected > 60, "expected many invalid ristretto add rejects, got {rejected}");
}

#[test]
fn core_ristretto255_from_hash() {
    let l = libs();
    let (cf, rf) = sympair!(l, b"crypto_core_ristretto255_from_hash", unsafe extern "C" fn(*mut u8, *const u8) -> i32);
    let (cv, _rv) = sympair!(l, b"crypto_core_ristretto255_is_valid_point", Fis);
    let mut rng = Rng::new(0x1517_2);
    for _ in 0..200 {
        let r = rng.vec(CORE_HASHBYTES);
        unsafe {
            let (mut cp, mut rp) = ([0u8; CORE_BYTES], [0u8; CORE_BYTES]);
            let a = cf(cp.as_mut_ptr(), r.as_ptr());
            let b = rf(rp.as_mut_ptr(), r.as_ptr());
            assert_eq!(a, b, "from_hash rc");
            assert_eq!(cp, rp, "from_hash out");
            assert_eq!(cv(cp.as_ptr()), 1, "from_hash produces valid point");
        }
    }
}

#[test]
fn core_ristretto255_scalar_ops() {
    let l = libs();
    let (cadd, radd) = sympair!(l, b"crypto_core_ristretto255_scalar_add", Vsss);
    let (csub, rsub) = sympair!(l, b"crypto_core_ristretto255_scalar_sub", Vsss);
    let (cmul, rmul) = sympair!(l, b"crypto_core_ristretto255_scalar_mul", Vsss);
    let (cneg, rneg) = sympair!(l, b"crypto_core_ristretto255_scalar_negate", Vss);
    let (ccmp, rcmp) = sympair!(l, b"crypto_core_ristretto255_scalar_complement", Vss);
    let (cred, rred) = sympair!(l, b"crypto_core_ristretto255_scalar_reduce", Vss);
    let (cinv, rinv) = sympair!(l, b"crypto_core_ristretto255_scalar_invert", unsafe extern "C" fn(*mut u8, *const u8) -> i32);
    let (ccan, rcan) = sympair!(l, b"crypto_core_ristretto255_scalar_is_canonical", Fis);
    let mut rng = Rng::new(0x1517_3);
    for _ in 0..250 {
        let x = rng.vec(CORE_SCALAR);
        let y = rng.vec(CORE_SCALAR);
        unsafe {
            for (cf, rf) in [(&cadd, &radd), (&csub, &rsub), (&cmul, &rmul)] {
                let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
                cf(cz.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                rf(rz.as_mut_ptr(), x.as_ptr(), y.as_ptr());
                assert_eq!(cz, rz, "ristretto scalar binop parity");
            }
            for (cf, rf) in [(&cneg, &rneg), (&ccmp, &rcmp)] {
                let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
                cf(cz.as_mut_ptr(), x.as_ptr());
                rf(rz.as_mut_ptr(), x.as_ptr());
                assert_eq!(cz, rz, "ristretto scalar unop parity");
            }
            let (mut cz, mut rz) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
            assert_eq!(cinv(cz.as_mut_ptr(), x.as_ptr()), rinv(rz.as_mut_ptr(), x.as_ptr()));
            assert_eq!(cz, rz, "ristretto scalar_invert out");
            assert_eq!(ccan(x.as_ptr()), rcan(x.as_ptr()), "ristretto scalar_is_canonical");
            let big = rng.vec(CORE_NONREDUCED);
            let (mut cr, mut rr) = ([0u8; CORE_SCALAR], [0u8; CORE_SCALAR]);
            cred(cr.as_mut_ptr(), big.as_ptr());
            rred(rr.as_mut_ptr(), big.as_ptr());
            assert_eq!(cr, rr, "ristretto scalar_reduce parity");
        }
    }
}
