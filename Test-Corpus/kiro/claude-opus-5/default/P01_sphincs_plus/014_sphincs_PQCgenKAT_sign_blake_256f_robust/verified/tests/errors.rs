//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input the C rejects, calls **both**
//! libraries through their `.so` exports, and asserts they return the *same*
//! error code or sentinel — not merely that both failed.

mod common;

use common::*;
use std::ffi::{c_uint, c_ulong, c_ulonglong};

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

/// `AES_XOF_struct` = `buffer[16] ‖ unsigned long buffer_pos ‖
/// unsigned long length_remaining ‖ key[32] ‖ ctr[16]`.
const XOF_STRUCT_BYTES: usize = 16 + 8 + 8 + 32 + 16;
/// `AES256_CTR_DRBG_struct` = `Key[32] ‖ V[16] ‖ int reseed_counter`.
const DRBG_STRUCT_BYTES: usize = 32 + 16 + 4;

fn backend_is(want: &str) -> bool {
    env().1.backend() == want
}

/// A valid (pk, sk, message, signature) quadruple to corrupt.
struct Keyed {
    pk: Vec<u8>,
    #[allow(dead_code)]
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
    sm: Vec<u8>,
}

fn keyed(rng: &mut Rng, mlen: usize) -> Keyed {
    let (l, p) = env();
    unsafe {
        let kp: FnSeedKeypair = *l.c("crypto_sign_seed_keypair");
        let sign: FnSignature = *l.c("crypto_sign_signature");
        let seed = rng.bytes(p.seed_bytes());
        let mut pk = vec![0u8; p.pk_bytes()];
        let mut sk = vec![0u8; p.sk_bytes()];
        assert_eq!(kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()), 0);
        let m = rng.bytes(mlen);
        let mut sig = vec![0u8; p.spx_bytes()];
        let mut sl = 0usize;
        assert_eq!(
            sign(sig.as_mut_ptr(), &mut sl, m.as_ptr(), mlen, sk.as_ptr()),
            0
        );
        assert_eq!(sl, p.spx_bytes());
        let mut sm = sig.clone();
        sm.extend_from_slice(&m);
        Keyed { pk, sk, m, sig, sm }
    }
}

/* ================================================================== */
/* row 1 — crypto_sign_verify: siglen != SPX_BYTES                     */
/* ================================================================== */

#[test]
fn err_verify_wrong_siglen() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE001);
    let k = keyed(&mut rng, 64);
    let n = p.spx_bytes();
    unsafe {
        let cv: FnVerify = *l.c("crypto_sign_verify");
        let rv: FnVerify = *l.r("crypto_sign_verify");
        for siglen in [0usize, 1, 2, n - 2, n - 1, n + 1, n + 2, 2 * n, usize::MAX] {
            let c = cv(k.sig.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            let r = rv(k.sig.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            same_i(&format!("crypto_sign_verify(siglen={siglen})"), c, r);
            assert_eq!(c, -1, "siglen={siglen} must be rejected with -1");
        }
        // and the accepted length still verifies
        let c = cv(k.sig.as_ptr(), n, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
        let r = rv(k.sig.as_ptr(), n, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
        same_i("crypto_sign_verify(siglen=SPX_BYTES)", c, r);
        assert_eq!(c, 0);
    }
}

/* ================================================================== */
/* row 2 — crypto_sign_verify: root mismatch                           */
/* ================================================================== */

#[test]
fn err_verify_bad_signature() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE002);
    let k = keyed(&mut rng, 64);
    let n = p.n_();
    unsafe {
        let cv: FnVerify = *l.c("crypto_sign_verify");
        let rv: FnVerify = *l.r("crypto_sign_verify");
        // `expect_reject = true` only where rejection is structurally certain:
        // any byte at offset >= SPX_N lives in the FORS signature, a WOTS
        // signature or an authentication path, all of which feed directly into
        // the recomputed root.
        //
        // Bytes inside R (offset < SPX_N) are NOT certain: `hash_blake.c` calls
        // `blakeX_update(&S, R, SPX_N)` and `blake*_update` takes its length in
        // *bits*, so the BLAKE backend absorbs only the first SPX_N/8 bytes of
        // R.  That is what the C does, so it is what the Rust must do -- for
        // those cases the C's own answer is the ground truth and the test only
        // requires the two to agree.
        let mut rejected = 0usize;
        let mut check = |what: String, sig: &[u8], m: &[u8], pk: &[u8], expect_reject: bool| {
            let c = cv(sig.as_ptr(), sig.len(), m.as_ptr(), m.len(), pk.as_ptr());
            let r = rv(sig.as_ptr(), sig.len(), m.as_ptr(), m.len(), pk.as_ptr());
            same_i(&what, c, r);
            if expect_reject {
                assert_eq!(c, -1, "{what}: expected rejection");
            }
            if c == -1 {
                rejected += 1;
            }
        };

        // R region: agreement only (see above)
        for pos in 0..n {
            let mut bad = k.sig.clone();
            bad[pos] ^= 0x01;
            check(format!("flip R byte {pos}"), &bad, &k.m, &k.pk, false);
        }
        // FORS signature, WOTS signatures and auth paths: certain rejection
        let mut positions = vec![
            n,
            n + 1,
            n + p.fors_bytes() / 2,
            n + p.fors_bytes() - 1,
            n + p.fors_bytes(),
            p.spx_bytes() / 2,
            p.spx_bytes() - 1,
        ];
        for _ in 0..16 {
            positions.push(n + rng.below(p.spx_bytes() - n));
        }
        for pos in positions {
            for bit in [0x01u8, 0x80] {
                let mut bad = k.sig.clone();
                bad[pos] ^= bit;
                check(format!("flip sig byte {pos} bit {bit:#x}"), &bad, &k.m, &k.pk, true);
            }
        }
        check(
            "zero signature".into(),
            &vec![0u8; p.spx_bytes()],
            &k.m,
            &k.pk,
            true,
        );
        for _ in 0..4 {
            check(
                "random signature".into(),
                &rng.bytes(p.spx_bytes()),
                &k.m,
                &k.pk,
                true,
            );
        }
        // message corruption: for BLAKE only the first mlen/8 bytes are
        // absorbed, so agreement is the invariant, not rejection
        for pos in 0..k.m.len() {
            let mut m = k.m.clone();
            m[pos] ^= 0x80;
            check(format!("flip message byte {pos}"), &k.sig, &m, &k.pk, false);
        }
        check("truncated message".into(), &k.sig, &k.m[..32], &k.pk, false);
        check("empty message".into(), &k.sig, &[], &k.pk, false);
        // public key: pub_seed feeds ctx and the root half is compared directly
        for pos in 0..p.pk_bytes() {
            let mut pk = k.pk.clone();
            pk[pos] ^= 0x01;
            check(format!("flip pk byte {pos}"), &k.sig, &k.m, &pk, true);
        }
        assert!(
            rejected > 20,
            "only {rejected} of the corruptions were rejected -- the test is \
             not actually exercising the rejection path"
        );
    }
}

/* ================================================================== */
/* rows 3-4 — crypto_sign_open                                         */
/* ================================================================== */

#[test]
fn err_open_short_smlen() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE003);
    let k = keyed(&mut rng, 64);
    let n = p.spx_bytes();
    unsafe {
        let co: FnOpen = *l.c("crypto_sign_open");
        let ro: FnOpen = *l.r("crypto_sign_open");
        for smlen in [0usize, 1, 2, n / 2, n - 2, n - 1] {
            // the C memsets `smlen` bytes of m, so give it that much room
            let mut cm = vec![0xA5u8; smlen + 64];
            let mut rm = cm.clone();
            let mut cml: c_ulonglong = 0xDEAD_BEEF;
            let mut rml: c_ulonglong = 0xDEAD_BEEF;
            let c = co(
                cm.as_mut_ptr(),
                &mut cml,
                k.sm.as_ptr(),
                smlen as c_ulonglong,
                k.pk.as_ptr(),
            );
            let r = ro(
                rm.as_mut_ptr(),
                &mut rml,
                k.sm.as_ptr(),
                smlen as c_ulonglong,
                k.pk.as_ptr(),
            );
            same_i(&format!("crypto_sign_open(smlen={smlen}) ret"), c, r);
            assert_eq!(c, -1, "smlen={smlen} < SPX_BYTES must be rejected");
            same_u(&format!("crypto_sign_open(smlen={smlen}) *mlen"), cml, rml);
            assert_eq!(cml, 0, "*mlen must be zeroed on rejection");
            // and m[0..smlen] must be zeroed identically, guard bytes intact
            same(&format!("crypto_sign_open(smlen={smlen}) m"), &cm, &rm);
            assert!(cm[..smlen].iter().all(|&b| b == 0), "m not zeroed");
            assert!(cm[smlen..].iter().all(|&b| b == 0xA5), "m over-written");
        }
    }
}

#[test]
fn err_open_bad_signature() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE004);
    let k = keyed(&mut rng, 64);
    let n = p.spx_bytes();
    unsafe {
        let co: FnOpen = *l.c("crypto_sign_open");
        let ro: FnOpen = *l.r("crypto_sign_open");
        let mut cases: Vec<(String, Vec<u8>, bool)> = Vec::new();
        // smlen == SPX_BYTES exactly (empty message) with a corrupted FORS byte
        let mut only_sig = k.sig.clone();
        only_sig[p.n_()] ^= 1;
        cases.push(("corrupt sig, smlen==SPX_BYTES".into(), only_sig, true));
        // signature region (offset >= SPX_N) -> certain rejection
        for pos in [p.n_(), n / 2, n - 1] {
            let mut sm = k.sm.clone();
            sm[pos] ^= 0x01;
            cases.push((format!("flip sm byte {pos} (signature)"), sm, true));
        }
        // R region and message region: only agreement is guaranteed, because
        // hash_blake.c passes byte counts to the bit-oriented blake*_update
        for pos in [0usize, n - p.spx_bytes() + p.n_() - 1, n, n + 10, k.sm.len() - 1] {
            let mut sm = k.sm.clone();
            sm[pos] ^= 0x01;
            cases.push((format!("flip sm byte {pos}"), sm, false));
        }
        cases.push(("all-zero sm".into(), vec![0u8; k.sm.len()], true));
        for _ in 0..4 {
            cases.push(("random sm".into(), rng.bytes(k.sm.len()), true));
        }

        let mut rejected = 0usize;
        for (what, sm, expect_reject) in cases {
            let smlen = sm.len();
            let mut cm = vec![0xA5u8; smlen + 64];
            let mut rm = cm.clone();
            let mut cml: c_ulonglong = 0xDEAD_BEEF;
            let mut rml: c_ulonglong = 0xDEAD_BEEF;
            let c = co(
                cm.as_mut_ptr(),
                &mut cml,
                sm.as_ptr(),
                smlen as c_ulonglong,
                k.pk.as_ptr(),
            );
            let r = ro(
                rm.as_mut_ptr(),
                &mut rml,
                sm.as_ptr(),
                smlen as c_ulonglong,
                k.pk.as_ptr(),
            );
            same_i(&format!("crypto_sign_open({what}) ret"), c, r);
            same_u(&format!("crypto_sign_open({what}) *mlen"), cml, rml);
            same(&format!("crypto_sign_open({what}) m"), &cm, &rm);
            if expect_reject {
                assert_eq!(c, -1, "{what} must be rejected");
            }
            if c == -1 {
                rejected += 1;
                assert_eq!(cml, 0, "{what}: *mlen must be zeroed on rejection");
                // the C zeroes the *full* smlen, not smlen - SPX_BYTES
                assert!(cm[..smlen].iter().all(|&b| b == 0), "{what}: m not fully zeroed");
                assert!(cm[smlen..].iter().all(|&b| b == 0xA5), "{what}: overrun");
            }
        }
        assert!(rejected >= 8, "only {rejected} rejections -- test is vacuous");
    }
}

/* ================================================================== */
/* rows 5, 9 — seedexpander_init maxlen boundary                        */
/* ================================================================== */

#[test]
fn err_seedexpander_init_maxlen() {
    let (l, _p) = env();
    let mut rng = Rng::new(0xE005);
    unsafe {
        let ci: FnSeedexpanderInit = *l.c("seedexpander_init");
        let ri: FnSeedexpanderInit = *l.r("seedexpander_init");
        // On LP64 `unsigned long` is 64 bits, so 0x100000000 is representable
        // and the `maxlen >= 0x100000000` guard is reachable.
        let rejected: [u64; 5] = [
            0x1_0000_0000,
            0x1_0000_0001,
            0x2_0000_0000,
            0xFFFF_FFFF_FFFF_FFFF,
            0x8000_0000_0000_0000,
        ];
        let accepted: [u64; 6] = [0, 1, 16, 0xFFFF, 0xFFFF_FFFE, 0xFFFF_FFFF];

        for maxlen in rejected {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut s2 = seed.clone();
            let mut d2 = div.clone();
            // pre-fill so "ctx untouched" is observable
            let mut cctx = vec![0x5Au8; XOF_STRUCT_BYTES];
            let mut rctx = cctx.clone();
            let c = ci(
                cctx.as_mut_ptr(),
                seed.as_mut_ptr(),
                div.as_mut_ptr(),
                maxlen as c_ulong,
            );
            let r = ri(
                rctx.as_mut_ptr(),
                s2.as_mut_ptr(),
                d2.as_mut_ptr(),
                maxlen as c_ulong,
            );
            same_i(&format!("seedexpander_init(maxlen={maxlen:#x})"), c, r);
            assert_eq!(c, RNG_BAD_MAXLEN, "maxlen={maxlen:#x} must give RNG_BAD_MAXLEN");
            same(
                &format!("seedexpander_init(maxlen={maxlen:#x}) ctx untouched"),
                &cctx,
                &rctx,
            );
            assert!(cctx.iter().all(|&b| b == 0x5A), "ctx written before the guard");
        }

        for maxlen in accepted {
            let mut seed = rng.bytes(32);
            let mut div = rng.bytes(8);
            let mut s2 = seed.clone();
            let mut d2 = div.clone();
            let mut cctx = vec![0x5Au8; XOF_STRUCT_BYTES];
            let mut rctx = cctx.clone();
            let c = ci(
                cctx.as_mut_ptr(),
                seed.as_mut_ptr(),
                div.as_mut_ptr(),
                maxlen as c_ulong,
            );
            let r = ri(
                rctx.as_mut_ptr(),
                s2.as_mut_ptr(),
                d2.as_mut_ptr(),
                maxlen as c_ulong,
            );
            same_i(&format!("seedexpander_init(maxlen={maxlen:#x})"), c, r);
            assert_eq!(c, RNG_SUCCESS, "maxlen={maxlen:#x} must be accepted");
            same(
                &format!("seedexpander_init(maxlen={maxlen:#x}) ctx"),
                &cctx,
                &rctx,
            );
        }
    }
}

/* ================================================================== */
/* rows 6-7, 10 — seedexpander rejections                              */
/* ================================================================== */

/// Initialises an `AES_XOF_struct` in both libraries with the same inputs.
unsafe fn xof(rng: &mut Rng, maxlen: u64) -> (Vec<u8>, Vec<u8>) {
    let (l, _p) = env();
    let ci: FnSeedexpanderInit = *l.c("seedexpander_init");
    let ri: FnSeedexpanderInit = *l.r("seedexpander_init");
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);
    let mut s2 = seed.clone();
    let mut d2 = div.clone();
    let mut c = vec![0u8; XOF_STRUCT_BYTES];
    let mut r = c.clone();
    assert_eq!(
        ci(c.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as c_ulong),
        0
    );
    assert_eq!(
        ri(r.as_mut_ptr(), s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen as c_ulong),
        0
    );
    same("seedexpander_init ctx", &c, &r);
    (c, r)
}

#[test]
fn err_seedexpander_null_outbuf() {
    let (l, _p) = env();
    let mut rng = Rng::new(0xE006);
    unsafe {
        let cse: FnSeedexpander = *l.c("seedexpander");
        let rse: FnSeedexpander = *l.r("seedexpander");
        // The NULL check precedes the length check, so NULL must win even when
        // the length is *also* invalid.
        for xlen in [0u64, 1, 16, 4096, u64::MAX] {
            let (mut c, mut r) = xof(&mut rng, 4096);
            let before = c.clone();
            let cv = cse(c.as_mut_ptr(), std::ptr::null_mut(), xlen as c_ulong);
            let rv = rse(r.as_mut_ptr(), std::ptr::null_mut(), xlen as c_ulong);
            same_i(&format!("seedexpander(x=NULL, xlen={xlen})"), cv, rv);
            assert_eq!(cv, RNG_BAD_OUTBUF);
            same("seedexpander(x=NULL) ctx untouched", &c, &r);
            same("seedexpander(x=NULL) ctx unchanged", &before, &c);
        }
    }
}

#[test]
fn err_seedexpander_req_len() {
    let (l, _p) = env();
    let mut rng = Rng::new(0xE007);
    unsafe {
        let cse: FnSeedexpander = *l.c("seedexpander");
        let rse: FnSeedexpander = *l.r("seedexpander");
        for maxlen in [0u64, 1, 16, 100] {
            // `xlen >= ctx->length_remaining` is rejected, so xlen == maxlen too
            for xlen in [maxlen, maxlen + 1, maxlen + 1000, u64::MAX] {
                let (mut c, mut r) = xof(&mut rng, maxlen);
                let before = c.clone();
                let mut co = vec![0xA5u8; 1024];
                let mut ro = co.clone();
                let cv = cse(c.as_mut_ptr(), co.as_mut_ptr(), xlen as c_ulong);
                let rv = rse(r.as_mut_ptr(), ro.as_mut_ptr(), xlen as c_ulong);
                same_i(
                    &format!("seedexpander(maxlen={maxlen}, xlen={xlen})"),
                    cv,
                    rv,
                );
                assert_eq!(
                    cv, RNG_BAD_REQ_LEN,
                    "xlen={xlen} >= length_remaining={maxlen} must give RNG_BAD_REQ_LEN"
                );
                same("seedexpander(too long) out untouched", &co, &ro);
                assert!(co.iter().all(|&b| b == 0xA5), "output written on rejection");
                same("seedexpander(too long) ctx", &c, &r);
                same("seedexpander(too long) ctx unchanged", &before, &c);
            }
            // one step inside the range must succeed identically
            if maxlen > 0 {
                let xlen = maxlen - 1;
                let (mut c, mut r) = xof(&mut rng, maxlen);
                let mut co = vec![0xA5u8; xlen as usize + 32];
                let mut ro = co.clone();
                let cv = cse(c.as_mut_ptr(), co.as_mut_ptr(), xlen as c_ulong);
                let rv = rse(r.as_mut_ptr(), ro.as_mut_ptr(), xlen as c_ulong);
                same_i(&format!("seedexpander(xlen={xlen}) accepted"), cv, rv);
                assert_eq!(cv, RNG_SUCCESS);
                same("seedexpander accepted out", &co, &ro);
                same("seedexpander accepted ctx", &c, &r);
            }
        }
    }
}

#[test]
fn ok_seedexpander_stream() {
    // row 10: repeated successful draws, chaining the internal buffer and the
    // ctr[12..16] carry.
    let (l, _p) = env();
    let mut rng = Rng::new(0xE010);
    unsafe {
        let cse: FnSeedexpander = *l.c("seedexpander");
        let rse: FnSeedexpander = *l.r("seedexpander");
        let (mut c, mut r) = xof(&mut rng, 0xFFFF_FFFF);
        for step in 0..64 {
            let xlen = 1 + rng.below(40);
            let mut co = vec![0xA5u8; xlen + 32];
            let mut ro = co.clone();
            let cv = cse(c.as_mut_ptr(), co.as_mut_ptr(), xlen as c_ulong);
            let rv = rse(r.as_mut_ptr(), ro.as_mut_ptr(), xlen as c_ulong);
            same_i(&format!("seedexpander step {step}"), cv, rv);
            assert_eq!(cv, RNG_SUCCESS);
            same(&format!("seedexpander step {step} out"), &co, &ro);
            same(&format!("seedexpander step {step} ctx"), &c, &r);
        }
    }
}

/* ================================================================== */
/* rows 11, 44-45 — randombytes / randombytes_init                     */
/* ================================================================== */

#[test]
fn err_randombytes_zero_len() {
    // xlen == 0 skips the output loop but STILL re-derives Key/V and bumps
    // reseed_counter; a translation that early-returns would diverge.
    let (l, _p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE011);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let cr: FnRandombytes = *l.c("randombytes");
        let rr: FnRandombytes = *l.r("randombytes");
        let cd = l.c_data("DRBG_ctx");
        let rd = l.r_data("DRBG_ctx");

        let mut e = rng.bytes(48);
        let mut e2 = e.clone();
        ci(e.as_mut_ptr(), std::ptr::null_mut());
        ri(e2.as_mut_ptr(), std::ptr::null_mut());
        let seeded = std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES).to_vec();

        for step in 0..5 {
            let mut co = vec![0xA5u8; 32];
            let mut ro = co.clone();
            let c = cr(co.as_mut_ptr(), 0);
            let r = rr(ro.as_mut_ptr(), 0);
            same_i("randombytes(xlen=0) ret", c, r);
            assert_eq!(c, RNG_SUCCESS);
            same("randombytes(xlen=0) out untouched", &co, &ro);
            assert!(co.iter().all(|&b| b == 0xA5), "wrote output for xlen=0");
            let cstate = std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES);
            let rstate = std::slice::from_raw_parts(rd, DRBG_STRUCT_BYTES);
            same(&format!("DRBG_ctx after randombytes(0) #{step}"), cstate, rstate);
            assert_ne!(
                cstate,
                &seeded[..],
                "randombytes(0) must still advance the DRBG"
            );
        }
    }
}

#[test]
fn err_randombytes_init_null_pers() {
    // row 44: the `if (personalization_string)` branch.
    let (l, _p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE044);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let cd = l.c_data("DRBG_ctx");
        let rd = l.r_data("DRBG_ctx");
        for _ in 0..16 {
            let mut e = rng.bytes(48);
            let mut e2 = e.clone();
            ci(e.as_mut_ptr(), std::ptr::null_mut());
            let cstate = std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES).to_vec();
            ri(e2.as_mut_ptr(), std::ptr::null_mut());
            let rstate = std::slice::from_raw_parts(rd, DRBG_STRUCT_BYTES).to_vec();
            same("randombytes_init(pers=NULL)", &cstate, &rstate);

            // an all-zero personalization string must give the same state as
            // NULL (XOR with 0 is the identity) -- a cheap cross-check that the
            // NULL branch is really "skip the XOR"
            let mut zeros = vec![0u8; 48];
            let mut e3 = e.clone();
            ci(e3.as_mut_ptr(), zeros.as_mut_ptr());
            let czero = std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES).to_vec();
            same("pers=NULL == pers=zeros (C)", &cstate, &czero);
        }
    }
}

#[test]
fn ok_randombytes_init_with_pers() {
    // row 45: all 48 bytes XORed.
    let (l, _p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE045);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let cd = l.c_data("DRBG_ctx");
        let rd = l.r_data("DRBG_ctx");
        for i in 0..16 {
            let mut e = rng.bytes(48);
            let mut ps = if i == 0 {
                vec![0xFFu8; 48]
            } else {
                rng.bytes(48)
            };
            let mut e2 = e.clone();
            let mut ps2 = ps.clone();
            ci(e.as_mut_ptr(), ps.as_mut_ptr());
            let cstate = std::slice::from_raw_parts(cd, DRBG_STRUCT_BYTES).to_vec();
            ri(e2.as_mut_ptr(), ps2.as_mut_ptr());
            let rstate = std::slice::from_raw_parts(rd, DRBG_STRUCT_BYTES).to_vec();
            same("randombytes_init(pers != NULL)", &cstate, &rstate);
            same("entropy_input untouched", &e, &e2);
            same("personalization_string untouched", &ps, &ps2);
        }
    }
}

/* ================================================================== */
/* rows 19-26 — out-of-range values on the address setters             */
/* ================================================================== */

fn addr_bytes(a: &[u32; 8]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a.as_ptr() as *const u8, 32) }
}

#[test]
fn err_set_type_out_of_range_enum() {
    // address.h documents SPX_ADDR_TYPE_WOTS..FORSPRF = 0..6, but the parameter
    // is a bare uint32_t that address.c narrows with (unsigned char).  A C enum
    // accepts any int, so values with no valid variant are real inputs.
    let (l, p) = env();
    let off = p.n("SPX_OFFSET_TYPE");
    let mut rng = Rng::new(0xE019);
    unsafe {
        let cf: FnAddrU32 = *l.c("SPX_set_type");
        let rf: FnAddrU32 = *l.r("SPX_set_type");
        let mut vals: Vec<u32> = vec![
            0, 1, 2, 3, 4, 5, 6, // documented variants
            7, 8, 9, 100, 254, 255, // one step past, and the byte maximum
            256, 257, 259, 262, 511, 512, // wrap around the (unsigned char) cast
            0xFFFF, 0xFFFF_FF00, 0xFFFF_FF06, u32::MAX,
        ];
        for _ in 0..512 {
            vals.push(rng.next_u32());
        }
        for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
            for v in &vals {
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), *v);
                rf(ra.as_mut_ptr(), *v);
                same(
                    &format!("SPX_set_type({v:#x}) full address"),
                    addr_bytes(&ca),
                    addr_bytes(&ra),
                );
                // and the C really does truncate rather than reject
                assert_eq!(
                    addr_bytes(&ca)[off],
                    (*v & 0xFF) as u8,
                    "set_type({v:#x}) did not truncate"
                );
                // every other byte untouched
                let bb = addr_bytes(&base);
                let cb = addr_bytes(&ca);
                for i in 0..32 {
                    if i != off {
                        assert_eq!(cb[i], bb[i], "set_type clobbered byte {i}");
                    }
                }
            }
        }
    }
}

#[test]
fn err_addr_byte_field_truncation() {
    let (l, p) = env();
    let mut rng = Rng::new(0xE020);
    // (symbol, offset, width in bytes)
    let byte_fields: [(&str, usize); 4] = [
        ("SPX_set_layer_addr", p.n("SPX_OFFSET_LAYER")),
        ("SPX_set_chain_addr", p.n("SPX_OFFSET_CHAIN_ADDR")),
        ("SPX_set_hash_addr", p.n("SPX_OFFSET_HASH_ADDR")),
        ("SPX_set_tree_height", p.n("SPX_OFFSET_TREE_HGT")),
    ];
    let word_fields: [(&str, usize); 2] = [
        ("SPX_set_keypair_addr", p.n("SPX_OFFSET_KP_ADDR")),
        ("SPX_set_tree_index", p.n("SPX_OFFSET_TREE_INDEX")),
    ];
    let mut vals: Vec<u32> = vec![0, 1, 6, 7, 255, 256, 257, 0xFFFF, 0xFFFF_FF00, u32::MAX];
    for _ in 0..256 {
        vals.push(rng.next_u32());
    }
    unsafe {
        for (name, off) in byte_fields {
            let cf: FnAddrU32 = *l.c(name);
            let rf: FnAddrU32 = *l.r(name);
            for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
                for v in &vals {
                    let mut ca = base;
                    let mut ra = base;
                    cf(ca.as_mut_ptr(), *v);
                    rf(ra.as_mut_ptr(), *v);
                    same(&format!("{name}({v:#x})"), addr_bytes(&ca), addr_bytes(&ra));
                    assert_eq!(addr_bytes(&ca)[off], (*v & 0xFF) as u8, "{name} truncation");
                }
            }
        }
        for (name, off) in word_fields {
            let cf: FnAddrU32 = *l.c(name);
            let rf: FnAddrU32 = *l.r(name);
            for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
                for v in &vals {
                    let mut ca = base;
                    let mut ra = base;
                    cf(ca.as_mut_ptr(), *v);
                    rf(ra.as_mut_ptr(), *v);
                    same(&format!("{name}({v:#x})"), addr_bytes(&ca), addr_bytes(&ra));
                    assert_eq!(
                        &addr_bytes(&ca)[off..off + 4],
                        &v.to_be_bytes()[..],
                        "{name} must be big-endian"
                    );
                }
            }
        }
        // SPX_set_tree_addr: 8-byte big-endian field, any u64 accepted
        let off = p.n("SPX_OFFSET_TREE");
        let cf: FnAddrU64 = *l.c("SPX_set_tree_addr");
        let rf: FnAddrU64 = *l.r("SPX_set_tree_addr");
        let mut v64: Vec<u64> = vec![0, 1, u32::MAX as u64, 1 << 32, u64::MAX];
        for _ in 0..256 {
            v64.push(rng.next_u64());
        }
        for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
            for v in &v64 {
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), *v);
                rf(ra.as_mut_ptr(), *v);
                same(
                    &format!("SPX_set_tree_addr({v:#x})"),
                    addr_bytes(&ca),
                    addr_bytes(&ra),
                );
                assert_eq!(&addr_bytes(&ca)[off..off + 8], &v.to_be_bytes()[..]);
            }
        }
    }
}

/* ================================================================== */
/* rows 27-31 — conversion-function length edges                       */
/* ================================================================== */

#[test]
fn err_ull_to_bytes_outlen_edges() {
    let (l, _p) = env();
    let mut rng = Rng::new(0xE027);
    unsafe {
        let cf: FnUllToBytes = *l.c("SPX_ull_to_bytes");
        let rf: FnUllToBytes = *l.r("SPX_ull_to_bytes");
        for outlen in [0usize, 1, 7, 8, 9, 12, 16, 32, 64] {
            let mut vals = vec![0u64, 1, u64::MAX, 0x1_0000_0000];
            for _ in 0..32 {
                vals.push(rng.next_u64());
            }
            for v in vals {
                let mut cb = vec![0xA5u8; outlen + 32];
                let mut rb = cb.clone();
                cf(cb.as_mut_ptr(), outlen as c_uint, v);
                rf(rb.as_mut_ptr(), outlen as c_uint, v);
                same(
                    &format!("SPX_ull_to_bytes(outlen={outlen}, {v:#x})"),
                    &cb,
                    &rb,
                );
                if outlen == 0 {
                    assert!(cb.iter().all(|&b| b == 0xA5), "outlen=0 must write nothing");
                }
                if outlen > 8 {
                    // the high bytes are zero once `in` has been shifted out
                    assert!(
                        cb[..outlen - 8].iter().all(|&b| b == 0),
                        "outlen>8 high bytes must be zero"
                    );
                }
                // never past the end
                assert!(cb[outlen..].iter().all(|&b| b == 0xA5), "overrun");
            }
        }
    }
}

#[test]
fn err_bytes_to_ull_inlen_edges() {
    let (l, _p) = env();
    let mut rng = Rng::new(0xE029);
    unsafe {
        let cf: FnBytesToUll = *l.c("SPX_bytes_to_ull");
        let rf: FnBytesToUll = *l.r("SPX_bytes_to_ull");
        // rows 29-30: the well-defined range 0..=8 must agree exactly
        for inlen in 0usize..=8 {
            for _ in 0..64 {
                let b = rng.bytes(32);
                same_u(
                    &format!("SPX_bytes_to_ull(inlen={inlen})"),
                    cf(b.as_ptr(), inlen as c_uint),
                    rf(b.as_ptr(), inlen as c_uint),
                );
            }
        }
        let zero = vec![0u8; 32];
        assert_eq!(cf(zero.as_ptr(), 0), 0, "inlen=0 must decode to 0");
        assert_eq!(rf(zero.as_ptr(), 0), 0, "inlen=0 must decode to 0");
        let ones = vec![0xFFu8; 32];
        assert_eq!(cf(ones.as_ptr(), 8), u64::MAX);
        assert_eq!(rf(ones.as_ptr(), 8), u64::MAX);

        // row 31: inlen > 8 shifts a u64 by >= 64, which is undefined behaviour
        // in C.  There is no C semantics to match, so only assert that neither
        // library aborts or reads out of bounds (the buffer is 32 bytes).
        for inlen in 9usize..=16 {
            let b = rng.bytes(32);
            let _ = cf(b.as_ptr(), inlen as c_uint);
            let _ = rf(b.as_ptr(), inlen as c_uint);
        }
    }
}

/* ================================================================== */
/* row 32 — thash with inblocks == 0                                   */
/* ================================================================== */

#[test]
fn err_thash_inblocks_edges() {
    let (l, p) = env();
    let n = p.n_();
    let mut rng = Rng::new(0xE032);
    unsafe {
        let init: FnInitHash = *l.c("SPX_initialize_hash_function");
        let rinit: FnInitHash = *l.r("SPX_initialize_hash_function");
        let cf: FnThash = *l.c("SPX_thash");
        let rf: FnThash = *l.r("SPX_thash");
        for nb in [0u32, 1, 2] {
            for _ in 0..24 {
                let mut cctx = vec![0u8; p.ctx_size()];
                rng.fill(&mut cctx[..2 * n]);
                let mut rctx = cctx.clone();
                init(cctx.as_mut_ptr());
                rinit(rctx.as_mut_ptr());
                same("ctx", &cctx, &rctx);
                let input = rng.bytes((nb as usize) * n);
                let addr = rng.addr();
                let mut ca = addr;
                let mut ra = addr;
                let mut co = vec![0xA5u8; n + 32];
                let mut ro = co.clone();
                cf(
                    co.as_mut_ptr(),
                    input.as_ptr(),
                    nb as c_uint,
                    cctx.as_ptr(),
                    ca.as_mut_ptr(),
                );
                rf(
                    ro.as_mut_ptr(),
                    input.as_ptr(),
                    nb as c_uint,
                    rctx.as_ptr(),
                    ra.as_mut_ptr(),
                );
                same(&format!("SPX_thash(inblocks={nb})"), &co, &ro);
                same(
                    &format!("SPX_thash(inblocks={nb}) addr"),
                    addr_bytes(&ca),
                    addr_bytes(&ra),
                );
            }
        }
    }
}

/* ================================================================== */
/* rows 18, 38-39 — BLAKE edges                                        */
/* ================================================================== */

#[test]
fn ok_blake_return_zero() {
    if !backend_is("blake") {
        return;
    }
    let (l, _p) = env();
    unsafe {
        for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
            let cf: FnBlakeOneShot = *l.c(name);
            let rf: FnBlakeOneShot = *l.r(name);
            for inlen in [0usize, 1, 64, 128, 1000] {
                let input = vec![0x42u8; inlen];
                let mut co = vec![0u8; outlen];
                let mut ro = vec![0u8; outlen];
                let c = cf(co.as_mut_ptr(), input.as_ptr(), inlen as c_ulonglong);
                let r = rf(ro.as_mut_ptr(), input.as_ptr(), inlen as c_ulonglong);
                same_i(&format!("{name} return value"), c, r);
                assert_eq!(c, 0, "{name} returns 0 unconditionally in C");
                same(&format!("{name}(inlen={inlen})"), &co, &ro);
            }
        }
    }
}

#[test]
fn err_blake_update_zero() {
    if !backend_is("blake") {
        return;
    }
    let (l, p) = env();
    unsafe {
        for (bits, state_size) in [
            (256usize, p.n("sizeof_blakestate256")),
            (512, p.n("sizeof_blakestate512")),
        ] {
            let ci: FnBlakeInit = *l.c(&format!("blake{bits}_init"));
            let ri: FnBlakeInit = *l.r(&format!("blake{bits}_init"));
            let cu: FnBlakeUpdate = *l.c(&format!("blake{bits}_update"));
            let ru: FnBlakeUpdate = *l.r(&format!("blake{bits}_update"));
            let mut cs = vec![0xA5u8; state_size];
            let mut rs = cs.clone();
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
            let data = vec![0u8; 8];
            // datalen is in BITS; 0 must leave buflen at 0 and compress nothing
            for _ in 0..4 {
                cu(cs.as_mut_ptr(), data.as_ptr(), 0);
                ru(rs.as_mut_ptr(), data.as_ptr(), 0);
                same(&format!("blake{bits}_update(0 bits) state"), &cs, &rs);
            }
        }
    }
}

#[test]
fn err_blake_final_padding_branches() {
    if !backend_is("blake") {
        return;
    }
    let (l, p) = env();
    let mut rng = Rng::new(0xE039);
    unsafe {
        for (bits, state_size, outlen, block) in [
            (256usize, p.n("sizeof_blakestate256"), 32usize, 64usize),
            (512, p.n("sizeof_blakestate512"), 64, 128),
        ] {
            let ci: FnBlakeInit = *l.c(&format!("blake{bits}_init"));
            let ri: FnBlakeInit = *l.r(&format!("blake{bits}_init"));
            let cu: FnBlakeUpdate = *l.c(&format!("blake{bits}_update"));
            let ru: FnBlakeUpdate = *l.r(&format!("blake{bits}_update"));
            let cfin: FnBlakeFinal = *l.c(&format!("blake{bits}_final"));
            let rfin: FnBlakeFinal = *l.r(&format!("blake{bits}_final"));

            // `final` branches on buflen vs (block*8 - 72) bits, i.e. on
            // 55 bytes for BLAKE-256 and 111 bytes for BLAKE-512:
            //   buflen == boundary  -> single padding byte 0x81
            //   buflen <  boundary  -> one compression (nullt when buflen == 0)
            //   buflen >  boundary  -> two compressions
            let boundary = block - 9;
            let mut prefixes: Vec<usize> = vec![0, 1, boundary - 1, boundary, boundary + 1];
            prefixes.push(block - 1);
            prefixes.push(block);
            prefixes.push(block + boundary);
            prefixes.push(block + boundary - 1);
            prefixes.push(block + boundary + 1);
            for _ in 0..16 {
                prefixes.push(rng.below(3 * block));
            }
            for prefix in prefixes {
                let mut cs = vec![0xA5u8; state_size];
                let mut rs = cs.clone();
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                let data = rng.bytes(prefix);
                cu(cs.as_mut_ptr(), data.as_ptr(), (prefix * 8) as c_ulonglong);
                ru(rs.as_mut_ptr(), data.as_ptr(), (prefix * 8) as c_ulonglong);
                same(&format!("blake{bits} state before final ({prefix} B)"), &cs, &rs);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cfin(cs.as_mut_ptr(), co.as_mut_ptr());
                rfin(rs.as_mut_ptr(), ro.as_mut_ptr());
                same(&format!("blake{bits}_final({prefix} B) digest"), &co, &ro);
                same(&format!("blake{bits}_final({prefix} B) state"), &cs, &rs);
            }
        }
    }
}

/* ================================================================== */
/* row 40 — MGF1 length edges (all backends that expose one)           */
/* ================================================================== */

#[test]
fn err_mgf1_outlen_edges() {
    let (l, p) = env();
    let names: &[(&str, usize)] = match p.backend() {
        "blake" => &[("SPX_blake256_mgf1", 32), ("SPX_blake512_mgf1", 64)],
        "sha2" => &[("SPX_mgf1_256", 32), ("SPX_mgf1_512", 64)],
        _ => &[], // shake / haraka expose no MGF1
    };
    if names.is_empty() {
        return;
    }
    let mut rng = Rng::new(0xE040);
    unsafe {
        for &(name, out_bytes) in names {
            let cf: FnMgf1 = *l.c(name);
            let rf: FnMgf1 = *l.r(name);
            // 0 (nothing written), an exact multiple (tail branch skipped) and
            // one past it (tail branch taken)
            for outlen in [
                0usize,
                1,
                out_bytes - 1,
                out_bytes,
                out_bytes + 1,
                2 * out_bytes,
                2 * out_bytes + 1,
            ] {
                for inlen in [0usize, 1, 32, 64] {
                    let input = rng.bytes(inlen);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cf(co.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), inlen as c_ulong);
                    rf(ro.as_mut_ptr(), outlen as c_ulong, input.as_ptr(), inlen as c_ulong);
                    same(&format!("{name}(outlen={outlen}, inlen={inlen})"), &co, &ro);
                    if outlen == 0 {
                        assert!(co.iter().all(|&b| b == 0xA5), "outlen=0 must write nothing");
                    }
                    assert!(co[outlen..].iter().all(|&b| b == 0xA5), "{name} overrun");
                }
            }
        }
    }
}

/* ================================================================== */
/* row 41 — zero-length squeeze / absorb                               */
/* ================================================================== */

#[test]
fn err_squeeze_zero_len() {
    let (l, p) = env();
    let mut rng = Rng::new(0xE041);
    unsafe {
        match p.backend() {
            "shake" => {
                let cf: FnShake = *l.c("shake256");
                let rf: FnShake = *l.r("shake256");
                for (outlen, inlen) in [(0usize, 0usize), (0, 1), (0, 200), (1, 0), (200, 0)] {
                    let input = rng.bytes(inlen);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cf(co.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    rf(ro.as_mut_ptr(), outlen, input.as_ptr(), inlen);
                    same(&format!("shake256(out={outlen}, in={inlen})"), &co, &ro);
                    assert!(co[outlen..].iter().all(|&b| b == 0xA5));
                }
            }
            "haraka" => {
                let cf: FnHarakaS = *l.c("SPX_haraka_S");
                let rf: FnHarakaS = *l.r("SPX_haraka_S");
                let ct: FnTweakConstants = *l.c("SPX_tweak_constants");
                let rt: FnTweakConstants = *l.r("SPX_tweak_constants");
                let mut ctx = vec![0u8; p.ctx_size()];
                rng.fill(&mut ctx[..2 * p.n_()]);
                let mut rctx = ctx.clone();
                ct(ctx.as_mut_ptr());
                rt(rctx.as_mut_ptr());
                same("tweak_constants", &ctx, &rctx);
                for (outlen, inlen) in [(0usize, 0usize), (0, 1), (0, 200), (1, 0), (200, 0)] {
                    let input = rng.bytes(inlen);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    cf(
                        co.as_mut_ptr(),
                        outlen as c_ulonglong,
                        input.as_ptr(),
                        inlen as c_ulonglong,
                        ctx.as_ptr(),
                    );
                    rf(
                        ro.as_mut_ptr(),
                        outlen as c_ulonglong,
                        input.as_ptr(),
                        inlen as c_ulonglong,
                        rctx.as_ptr(),
                    );
                    same(&format!("SPX_haraka_S(out={outlen}, in={inlen})"), &co, &ro);
                    assert!(co[outlen..].iter().all(|&b| b == 0xA5));
                }
            }
            "blake" => {
                for (name, outlen) in [("blake256", 32usize), ("blake512", 64)] {
                    let cf: FnBlakeOneShot = *l.c(name);
                    let rf: FnBlakeOneShot = *l.r(name);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    let empty: [u8; 1] = [0];
                    cf(co.as_mut_ptr(), empty.as_ptr(), 0);
                    rf(ro.as_mut_ptr(), empty.as_ptr(), 0);
                    same(&format!("{name}(inlen=0)"), &co, &ro);
                }
            }
            "sha2" => {
                for (name, outlen) in [("sha256", 32usize), ("sha512", 64)] {
                    let cf: FnShaOneShot = *l.c(name);
                    let rf: FnShaOneShot = *l.r(name);
                    let mut co = vec![0xA5u8; outlen + 32];
                    let mut ro = co.clone();
                    let empty: [u8; 1] = [0];
                    cf(co.as_mut_ptr(), empty.as_ptr(), 0);
                    rf(ro.as_mut_ptr(), empty.as_ptr(), 0);
                    same(&format!("{name}(inlen=0)"), &co, &ro);
                }
            }
            b => panic!("unknown backend {b}"),
        }
    }
}

/* ================================================================== */
/* rows 42-43 — SHA-2 incremental edges                                */
/* ================================================================== */

#[test]
fn err_sha_inc_zero_blocks() {
    if !backend_is("sha2") {
        return;
    }
    let (l, _p) = env();
    unsafe {
        for (bits, state_size) in [(256usize, 40usize), (512, 72)] {
            let ci: FnShaIncInit = *l.c(&format!("sha{bits}_inc_init"));
            let ri: FnShaIncInit = *l.r(&format!("sha{bits}_inc_init"));
            let cb: FnShaIncBlocks = *l.c(&format!("sha{bits}_inc_blocks"));
            let rb: FnShaIncBlocks = *l.r(&format!("sha{bits}_inc_blocks"));
            let mut cs = vec![0xA5u8; state_size];
            let mut rs = cs.clone();
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
            let after_init = cs.clone();
            let data = vec![0u8; 256];
            for _ in 0..4 {
                cb(cs.as_mut_ptr(), data.as_ptr(), 0);
                rb(rs.as_mut_ptr(), data.as_ptr(), 0);
                same(&format!("sha{bits}_inc_blocks(0)"), &cs, &rs);
            }
            same(
                &format!("sha{bits}_inc_blocks(0) leaves the state alone"),
                &after_init,
                &cs,
            );
        }
    }
}

#[test]
fn err_sha_inc_finalize_padding() {
    if !backend_is("sha2") {
        return;
    }
    let (l, _p) = env();
    let mut rng = Rng::new(0xE043);
    unsafe {
        for (bits, state_size, outlen, block) in
            [(256usize, 40usize, 32usize, 64usize), (512, 72, 64, 128)]
        {
            let ci: FnShaIncInit = *l.c(&format!("sha{bits}_inc_init"));
            let ri: FnShaIncInit = *l.r(&format!("sha{bits}_inc_init"));
            let cfin: FnShaIncFinalize = *l.c(&format!("sha{bits}_inc_finalize"));
            let rfin: FnShaIncFinalize = *l.r(&format!("sha{bits}_inc_finalize"));
            // the padding needs 1 + 8 (or 1 + 16 for SHA-512) trailing bytes,
            // so the "one more block" boundary sits just below the block size
            let mut lens: Vec<usize> = vec![0, 1];
            for d in 0..20usize {
                if block > d {
                    lens.push(block - d);
                }
                lens.push(block + d);
                lens.push(2 * block + d);
            }
            lens.sort_unstable();
            lens.dedup();
            for inlen in lens {
                let mut cs = vec![0xA5u8; state_size];
                let mut rs = cs.clone();
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
                let data = rng.bytes(inlen);
                let mut co = vec![0xA5u8; outlen + 32];
                let mut ro = co.clone();
                cfin(co.as_mut_ptr(), cs.as_mut_ptr(), data.as_ptr(), inlen);
                rfin(ro.as_mut_ptr(), rs.as_mut_ptr(), data.as_ptr(), inlen);
                same(&format!("sha{bits}_inc_finalize(inlen={inlen})"), &co, &ro);
                assert!(co[outlen..].iter().all(|&b| b == 0xA5), "overrun");
            }
        }
    }
}

/* ================================================================== */
/* rows 12-17 — success sentinels of the public API                    */
/* ================================================================== */

#[test]
fn ok_sign_verify_open_sentinels() {
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE014);
    unsafe {
        let ckp: FnSeedKeypair = *l.c("crypto_sign_seed_keypair");
        let rkp: FnSeedKeypair = *l.r("crypto_sign_seed_keypair");
        let csigf: FnSignature = *l.c("crypto_sign_signature");
        let rsigf: FnSignature = *l.r("crypto_sign_signature");
        let cverf: FnVerify = *l.c("crypto_sign_verify");
        let rverf: FnVerify = *l.r("crypto_sign_verify");
        let csign: FnSign = *l.c("crypto_sign");
        let rsign: FnSign = *l.r("crypto_sign");
        let copen: FnOpen = *l.c("crypto_sign_open");
        let ropen: FnOpen = *l.r("crypto_sign_open");
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");

        let seed = rng.bytes(p.seed_bytes());
        let mut cpk = vec![0u8; p.pk_bytes()];
        let mut csk = vec![0u8; p.sk_bytes()];
        let mut rpk = vec![0u8; p.pk_bytes()];
        let mut rsk = vec![0u8; p.sk_bytes()];
        // row 12
        let c = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        let r = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
        same_i("crypto_sign_seed_keypair", c, r);
        assert_eq!(c, 0);
        same("seed_keypair pk", &cpk, &rpk);
        same("seed_keypair sk", &csk, &rsk);

        for mlen in [0usize, 1, 64] {
            let m = rng.bytes(mlen);
            let mut e = rng.bytes(48);
            let mut e2 = e.clone();

            // rows 14, 16
            ci(e.as_mut_ptr(), std::ptr::null_mut());
            let mut cs = vec![0u8; p.spx_bytes()];
            let mut cl = usize::MAX;
            let cr = csigf(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, csk.as_ptr());
            ri(e2.as_mut_ptr(), std::ptr::null_mut());
            let mut rs = vec![0u8; p.spx_bytes()];
            let mut rl = usize::MAX;
            let rr = rsigf(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, rsk.as_ptr());
            same_i("crypto_sign_signature", cr, rr);
            assert_eq!(cr, 0);
            assert_eq!((cl, rl), (p.spx_bytes(), p.spx_bytes()));
            same("signature", &cs, &rs);
            let cv = cverf(cs.as_ptr(), cl, m.as_ptr(), mlen, cpk.as_ptr());
            let rv = rverf(rs.as_ptr(), rl, m.as_ptr(), mlen, rpk.as_ptr());
            same_i("crypto_sign_verify(valid)", cv, rv);
            assert_eq!(cv, 0);

            // rows 15, 17
            let mut e3 = e.clone();
            let mut e4 = e.clone();
            ci(e3.as_mut_ptr(), std::ptr::null_mut());
            let mut csm = vec![0u8; p.spx_bytes() + mlen];
            let mut csl: c_ulonglong = 0;
            let cr = csign(
                csm.as_mut_ptr(),
                &mut csl,
                m.as_ptr(),
                mlen as c_ulonglong,
                csk.as_ptr(),
            );
            ri(e4.as_mut_ptr(), std::ptr::null_mut());
            let mut rsm = vec![0u8; p.spx_bytes() + mlen];
            let mut rsl: c_ulonglong = 0;
            let rr = rsign(
                rsm.as_mut_ptr(),
                &mut rsl,
                m.as_ptr(),
                mlen as c_ulonglong,
                rsk.as_ptr(),
            );
            same_i("crypto_sign", cr, rr);
            assert_eq!(cr, 0);
            same_u("crypto_sign smlen", csl, rsl);
            assert_eq!(csl as usize, p.spx_bytes() + mlen);
            same("crypto_sign sm", &csm, &rsm);

            let mut cm = vec![0u8; csl as usize];
            let mut rm = vec![0u8; csl as usize];
            let mut cml: c_ulonglong = u64::MAX;
            let mut rml: c_ulonglong = u64::MAX;
            let cv = copen(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), csl, cpk.as_ptr());
            let rv = ropen(rm.as_mut_ptr(), &mut rml, rsm.as_ptr(), rsl, rpk.as_ptr());
            same_i("crypto_sign_open(valid)", cv, rv);
            assert_eq!(cv, 0);
            same_u("crypto_sign_open mlen", cml, rml);
            assert_eq!(cml as usize, mlen);
            same("crypto_sign_open m", &cm[..mlen], &rm[..mlen]);
            assert_eq!(&cm[..mlen], &m[..]);
        }
    }
}

#[test]
fn ok_keypair_from_drbg() {
    // row 13
    let (l, p) = env();
    let _g = drbg_lock();
    let mut rng = Rng::new(0xE013);
    unsafe {
        let ci: FnRandombytesInit = *l.c("randombytes_init");
        let ri: FnRandombytesInit = *l.r("randombytes_init");
        let ckp: FnKeypair = *l.c("crypto_sign_keypair");
        let rkp: FnKeypair = *l.r("crypto_sign_keypair");
        for _ in 0..3 {
            let mut e = rng.bytes(48);
            let mut e2 = e.clone();
            ci(e.as_mut_ptr(), std::ptr::null_mut());
            let mut cpk = vec![0u8; p.pk_bytes()];
            let mut csk = vec![0u8; p.sk_bytes()];
            let c = ckp(cpk.as_mut_ptr(), csk.as_mut_ptr());
            ri(e2.as_mut_ptr(), std::ptr::null_mut());
            let mut rpk = vec![0u8; p.pk_bytes()];
            let mut rsk = vec![0u8; p.sk_bytes()];
            let r = rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            same_i("crypto_sign_keypair", c, r);
            assert_eq!(c, 0);
            same("keypair pk", &cpk, &rpk);
            same("keypair sk", &csk, &rsk);
        }
    }
}
