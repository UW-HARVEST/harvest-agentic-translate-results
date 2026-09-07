//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input/condition the C rejects and
//! asserts BOTH libraries return the SAME sentinel (`-1` / `-2` / `-3` / `0`),
//! not merely "both failed".
//!
//! NOTE on the BLAKE backend: `hash_blake.c`'s `hash_message` /
//! `gen_message_random` call `blakeX_update(&S, m, mlen)` -- but
//! `blake256_update` / `blake512_update` take their length in **BITS**
//! (`blake256.c`: `blake256_update(&S, in, inlen*8)`). So for the BLAKE backend
//! the C only absorbs `mlen / 8` bytes of the message (and `SPX_N / 8` bytes of
//! `R`, `SPX_PK_BYTES / 8` bytes of `pk`). Consequently a corrupted short
//! message can still VERIFY under the BLAKE backend. That is the C's behaviour
//! and the Rust reproduces it, so the corrupt-input rows below assert the two
//! libraries return the SAME sentinel and separately assert that the `-1`
//! rejection path is genuinely reachable, rather than demanding `-1` for every
//! individual corruption.
mod common;
use common::*;

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------
type FnSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> core::ffi::c_int;
type FnKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> core::ffi::c_int;
type FnSignature =
    unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> core::ffi::c_int;
type FnVerify =
    unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> core::ffi::c_int;
type FnSign = unsafe extern "C" fn(
    *mut u8,
    *mut core::ffi::c_ulonglong,
    *const u8,
    core::ffi::c_ulonglong,
    *const u8,
) -> core::ffi::c_int;
type FnOpen = unsafe extern "C" fn(
    *mut u8,
    *mut core::ffi::c_ulonglong,
    *const u8,
    core::ffi::c_ulonglong,
    *const u8,
) -> core::ffi::c_int;
type FnRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FnRb = unsafe extern "C" fn(*mut u8, core::ffi::c_ulonglong) -> core::ffi::c_int;
type FnSeInit = unsafe extern "C" fn(
    *mut AesXofFfi,
    *mut u8,
    *mut u8,
    core::ffi::c_ulong,
) -> core::ffi::c_int;
type FnSe =
    unsafe extern "C" fn(*mut AesXofFfi, *mut u8, core::ffi::c_ulong) -> core::ffi::c_int;
type FnUllToBytes = unsafe extern "C" fn(*mut u8, core::ffi::c_uint, core::ffi::c_ulonglong);
type FnBytesToUll =
    unsafe extern "C" fn(*const u8, core::ffi::c_uint) -> core::ffi::c_ulonglong;
type FnAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FnAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FnThash =
    unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_uint, *const SpxCtxFfi, *mut u32);
type FnInitHash = unsafe extern "C" fn(*mut SpxCtxFfi);
type FnComputeRoot = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    u32,
    u32,
    *const u8,
    u32,
    *const SpxCtxFfi,
    *mut u32,
);
type GenLeafFn = unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, u32, *const u32);
type FnTreehash = unsafe extern "C" fn(
    *mut u8,
    *mut u8,
    *const SpxCtxFfi,
    u32,
    u32,
    u32,
    Option<GenLeafFn>,
    *mut u32,
);
type FnChainLengths = unsafe extern "C" fn(*mut core::ffi::c_uint, *const u8);
type FnWotsGenLeafX1 = unsafe extern "C" fn(*mut u8, *const SpxCtxFfi, u32, *mut LeafInfoX1Ffi);
type FnGenMsgRandom = unsafe extern "C" fn(
    *mut u8,
    *const u8,
    *const u8,
    *const u8,
    core::ffi::c_ulonglong,
    *const SpxCtxFfi,
);
type FnHashMessage = unsafe extern "C" fn(
    *mut u8,
    *mut u64,
    *mut u32,
    *const u8,
    *const u8,
    *const u8,
    core::ffi::c_ulonglong,
    *const SpxCtxFfi,
);
type FnMgf1 = unsafe extern "C" fn(*mut u8, core::ffi::c_ulong, *const u8, core::ffi::c_ulong);
type FnDrbgPtr = *mut DrbgCtxFfi;

const RNG_SUCCESS: core::ffi::c_int = 0;
const RNG_BAD_MAXLEN: core::ffi::c_int = -1;
const RNG_BAD_OUTBUF: core::ffi::c_int = -2;
const RNG_BAD_REQ_LEN: core::ffi::c_int = -3;

fn make_ctx(rng: &mut Rng) -> (SpxCtxFfi, SpxCtxFfi) {
    let l = libs();
    let (c, r) = l.pair::<FnInitHash>("SPX_initialize_hash_function");
    let mut cc = SpxCtxFfi::zeroed();
    rng.fill(&mut cc.pub_seed);
    rng.fill(&mut cc.sk_seed);
    let mut rc = cc.clone();
    unsafe {
        c(&mut cc);
        r(&mut rc);
    }
    assert_bytes_eq("spx_ctx after init", cc.as_bytes(), rc.as_bytes());
    (cc, rc)
}

/// A genuine (pk, sk, m, sm) quadruple produced by the C library.
struct Kat {
    pk: Vec<u8>,
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
    sm: Vec<u8>,
    smlen: u64,
}

fn make_kat(rng: &mut Rng, mlen: usize) -> Kat {
    let l = libs();
    let (kg, _) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let (rb_init, rb_init2) = l.pair::<FnRbInit>("randombytes_init");
    let (sg, _) = l.pair::<FnSignature>("crypto_sign_signature");
    let (sn, _) = l.pair::<FnSign>("crypto_sign");

    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
    let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
    unsafe {
        kg(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    }
    let m = rng.bytes(mlen.max(1));
    let mut ent = [0u8; 48];
    rng.fill(&mut ent);
    let mut e2 = ent;
    unsafe {
        rb_init(ent.as_mut_ptr(), core::ptr::null_mut());
        rb_init2(e2.as_mut_ptr(), core::ptr::null_mut());
    }
    let mut sig = vec![0u8; SPX_BYTES];
    let mut siglen = 0usize;
    unsafe {
        sg(sig.as_mut_ptr(), &mut siglen, m.as_ptr(), mlen, sk.as_ptr());
    }
    let mut ent2 = ent;
    let mut ent3 = ent;
    unsafe {
        rb_init(ent2.as_mut_ptr(), core::ptr::null_mut());
        rb_init2(ent3.as_mut_ptr(), core::ptr::null_mut());
    }
    let mut sm = vec![0u8; SPX_BYTES + mlen];
    let mut smlen: core::ffi::c_ulonglong = 0;
    unsafe {
        sn(
            sm.as_mut_ptr(),
            &mut smlen,
            m.as_ptr(),
            mlen as core::ffi::c_ulonglong,
            sk.as_ptr(),
        );
    }
    Kat { pk, sk, m, sig, sm, smlen }
}

// ===========================================================================
// Row 1 -- crypto_sign_verify: siglen != SPX_BYTES
// ===========================================================================
#[test]
fn err_01_verify_siglen_not_exact() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnVerify>("crypto_sign_verify");
    let mut rng = Rng::new(0x0001);
    let k = make_kat(&mut rng, 32);

    for &sl in &[
        0usize,
        1,
        SPX_N,
        SPX_BYTES - 1,
        SPX_BYTES + 1,
        SPX_BYTES * 2,
        usize::MAX,
        usize::MAX - 1,
    ] {
        let a = unsafe { c(k.sig.as_ptr(), sl, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        let b = unsafe { r(k.sig.as_ptr(), sl, k.m.as_ptr(), k.m.len(), k.pk.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_verify(siglen={sl}) sentinel"), a, b);
        assert_eq_dbg(&format!("crypto_sign_verify(siglen={sl}) == -1"), a, -1);
    }

    // The C returns BEFORE dereferencing anything, so a wrong siglen with NULL
    // pointers must also just return -1 on both sides.
    for &sl in &[0usize, SPX_BYTES - 1, SPX_BYTES + 1] {
        let a = unsafe {
            c(
                core::ptr::null(),
                sl,
                core::ptr::null(),
                0,
                core::ptr::null(),
            )
        };
        let b = unsafe {
            r(
                core::ptr::null(),
                sl,
                core::ptr::null(),
                0,
                core::ptr::null(),
            )
        };
        assert_eq_dbg(&format!("crypto_sign_verify(NULL, siglen={sl})"), a, b);
        assert_eq_dbg(&format!("crypto_sign_verify(NULL, siglen={sl}) == -1"), a, -1);
    }
}

// ===========================================================================
// Row 2 -- crypto_sign_verify: root mismatch (corrupt sig / m / pk)
// ===========================================================================
#[test]
fn err_02_verify_corrupt_inputs() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnVerify>("crypto_sign_verify");
    let mut rng = Rng::new(0x0002);
    let mut rejections = 0usize;

    for &mlen in &[0usize, 1, 32, 200] {
        let k = make_kat(&mut rng, mlen);
        // (a) flip one bit of the signature, at many different positions.
        for &pos in &[
            0usize,
            1,
            SPX_N - 1,
            SPX_N,
            SPX_N + SPX_FORS_BYTES / 2,
            SPX_N + SPX_FORS_BYTES,
            SPX_BYTES / 2,
            SPX_BYTES - 1,
        ] {
            let mut bad = k.sig.clone();
            bad[pos] ^= 1 << (pos % 8);
            let a = unsafe {
                c(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr())
            };
            let b = unsafe {
                r(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr())
            };
            assert_eq_dbg(
                &format!("crypto_sign_verify(mlen={mlen}) corrupt sig[{pos}]"),
                a,
                b,
            );
            assert_eq_dbg(
                &format!("crypto_sign_verify(mlen={mlen}) corrupt sig[{pos}] == -1"),
                a,
                -1,
            );
            rejections += 1;
        }
        // (b) flip one bit of the message.
        if mlen > 0 {
            let mut bm = k.m.clone();
            bm[mlen / 2] ^= 0x80;
            let a = unsafe { c(k.sig.as_ptr(), SPX_BYTES, bm.as_ptr(), mlen, k.pk.as_ptr()) };
            let b = unsafe { r(k.sig.as_ptr(), SPX_BYTES, bm.as_ptr(), mlen, k.pk.as_ptr()) };
            // Differential requirement only -- see the BLAKE note above.
            assert_eq_dbg(&format!("verify(mlen={mlen}) corrupt m"), a, b);
            if a == -1 {
                rejections += 1;
            }
        }
        // (c) wrong message LENGTH (same buffer).
        if mlen > 1 {
            let a = unsafe {
                c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen - 1, k.pk.as_ptr())
            };
            let b = unsafe {
                r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen - 1, k.pk.as_ptr())
            };
            assert_eq_dbg(&format!("verify(mlen={mlen}) truncated m"), a, b);
            if a == -1 {
                rejections += 1;
            }
        }
        // (d) flip one bit of the public key -- both halves (pub_seed and root).
        for pos in [0usize, SPX_N - 1, SPX_N, SPX_PK_BYTES - 1] {
            let mut bp = k.pk.clone();
            bp[pos] ^= 0x01;
            let a = unsafe {
                c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, bp.as_ptr())
            };
            let b = unsafe {
                r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, bp.as_ptr())
            };
            assert_eq_dbg(&format!("verify(mlen={mlen}) corrupt pk[{pos}]"), a, b);
            assert_eq_dbg(&format!("verify(mlen={mlen}) corrupt pk[{pos}] == -1"), a, -1);
            rejections += 1;
        }
        // (e) all-zero and all-ones signatures.
        for (nm, bad) in [("zeros", vec![0u8; SPX_BYTES]), ("ones", vec![0xFFu8; SPX_BYTES])] {
            let a = unsafe { c(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
            let b = unsafe { r(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
            assert_eq_dbg(&format!("verify(mlen={mlen}) all-{nm} sig"), a, b);
            assert_eq_dbg(&format!("verify(mlen={mlen}) all-{nm} sig == -1"), a, -1);
            rejections += 1;
        }
        // (f) fully random signatures.
        for _ in 0..8 {
            let bad = rng.bytes(SPX_BYTES);
            let a = unsafe { c(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
            let b = unsafe { r(bad.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
            assert_eq_dbg(&format!("verify(mlen={mlen}) random sig"), a, b);
            assert_eq_dbg(&format!("verify(mlen={mlen}) random sig == -1"), a, -1);
            rejections += 1;
        }
    }
    // ERRORS.md row 2 must be genuinely reachable.
    assert!(
        rejections > 0,
        "{} the sign.c:235 root-mismatch `return -1` was never reached",
        tag()
    );
}

// ===========================================================================
// Row 3 -- crypto_sign_verify: valid input returns 0
// ===========================================================================
#[test]
fn err_03_verify_valid() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnVerify>("crypto_sign_verify");
    let mut rng = Rng::new(0x0003);
    for &mlen in &[0usize, 1, 32, 231] {
        let k = make_kat(&mut rng, mlen);
        let a = unsafe { c(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
        let b = unsafe { r(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), mlen, k.pk.as_ptr()) };
        assert_eq_dbg(&format!("verify valid (mlen={mlen})"), a, b);
        assert_eq_dbg(&format!("verify valid (mlen={mlen}) == 0"), a, 0);
    }
}

// ===========================================================================
// Row 4 -- crypto_sign_open: smlen < SPX_BYTES
// ===========================================================================
#[test]
fn err_04_open_smlen_too_small() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnOpen>("crypto_sign_open");
    let mut rng = Rng::new(0x0004);
    let k = make_kat(&mut rng, 32);

    for &sl in &[0u64, 1, 16, (SPX_BYTES - 1) as u64] {
        // The C does `memset(m, 0, smlen)` -- exactly `smlen` bytes, no more.
        let mut cm = vec![0xA5u8; SPX_BYTES + 64];
        let mut rm = vec![0xA5u8; SPX_BYTES + 64];
        let mut cml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
        let mut rml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
        let a = unsafe { c(cm.as_mut_ptr(), &mut cml, k.sm.as_ptr(), sl, k.pk.as_ptr()) };
        let b = unsafe { r(rm.as_mut_ptr(), &mut rml, k.sm.as_ptr(), sl, k.pk.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_open(smlen={sl}) sentinel"), a, b);
        assert_eq_dbg(&format!("crypto_sign_open(smlen={sl}) == -1"), a, -1);
        assert_eq_dbg(&format!("crypto_sign_open(smlen={sl}) *mlen"), cml, rml);
        assert_eq_dbg(&format!("crypto_sign_open(smlen={sl}) *mlen == 0"), cml, 0);
        assert_bytes_eq(
            &format!("crypto_sign_open(smlen={sl}) m buffer (memset of exactly smlen)"),
            &cm,
            &rm,
        );
        // Verify the C really zeroed exactly `smlen` bytes.
        assert!(
            cm[..sl as usize].iter().all(|&x| x == 0),
            "{} C did not zero m[..{sl}]",
            tag()
        );
        assert!(
            cm[sl as usize..].iter().all(|&x| x == 0xA5),
            "{} C zeroed past m[{sl}]",
            tag()
        );
    }
}

// ===========================================================================
// Row 5 -- crypto_sign_open: inner verify fails
// ===========================================================================
#[test]
fn err_05_open_verify_fails() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnOpen>("crypto_sign_open");
    let mut rng = Rng::new(0x0005);
    let mut rejections = 0usize;

    for &mlen in &[0usize, 1, 32, 128] {
        let k = make_kat(&mut rng, mlen);
        let cases: Vec<(String, Vec<u8>, Vec<u8>)> = vec![
            {
                let mut bad = k.sm.clone();
                bad[0] ^= 1;
                ("corrupt sig byte 0".to_string(), bad, k.pk.clone())
            },
            {
                let mut bad = k.sm.clone();
                bad[SPX_BYTES - 1] ^= 1;
                ("corrupt last sig byte".to_string(), bad, k.pk.clone())
            },
            {
                let mut bad = k.sm.clone();
                if mlen > 0 {
                    bad[SPX_BYTES] ^= 1;
                }
                ("corrupt appended message".to_string(), bad, k.pk.clone())
            },
            {
                let mut bp = k.pk.clone();
                bp[SPX_PK_BYTES - 1] ^= 1;
                ("wrong pk".to_string(), k.sm.clone(), bp)
            },
            ("all-zero sm".to_string(), vec![0u8; SPX_BYTES + mlen], k.pk.clone()),
            ("random sm".to_string(), rng.bytes(SPX_BYTES + mlen), k.pk.clone()),
        ];
        for (name, sm, pk) in cases {
            if name == "corrupt appended message" && mlen == 0 {
                continue;
            }
            let smlen = (SPX_BYTES + mlen) as core::ffi::c_ulonglong;
            let mut cm = vec![0xA5u8; SPX_BYTES + mlen + 64];
            let mut rm = vec![0xA5u8; SPX_BYTES + mlen + 64];
            let mut cml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
            let mut rml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
            let a = unsafe { c(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk.as_ptr()) };
            let b = unsafe { r(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk.as_ptr()) };
            assert_eq_dbg(&format!("open [{name}] (mlen={mlen}) sentinel"), a, b);
            assert_eq_dbg(&format!("open [{name}] (mlen={mlen}) *mlen"), cml, rml);
            assert_bytes_eq(&format!("open [{name}] (mlen={mlen}) m buffer"), &cm, &rm);
            if a == -1 {
                // The C's error path must zero *mlen ...
                assert_eq_dbg(
                    &format!("open [{name}] (mlen={mlen}) *mlen reset to 0"),
                    cml,
                    0,
                );
                rejections += 1;
            } else {
                // ... and on the accept path *mlen == smlen - SPX_BYTES.
                assert_eq_dbg(
                    &format!("open [{name}] (mlen={mlen}) *mlen == smlen-SPX_BYTES"),
                    cml as usize,
                    mlen,
                );
            }
        }
    }
    assert!(
        rejections > 0,
        "{} the sign.c:277 inner-verify `return -1` was never reached",
        tag()
    );
}

// ===========================================================================
// Row 6 -- crypto_sign_open: smlen == SPX_BYTES exactly (empty message)
// ===========================================================================
#[test]
fn err_06_open_smlen_exact() {
    let _g = drbg_lock();
    let l = libs();
    let (c, r) = l.pair::<FnOpen>("crypto_sign_open");
    let mut rng = Rng::new(0x0006);
    let k = make_kat(&mut rng, 0);
    let mut cm = vec![0xA5u8; SPX_BYTES + 8];
    let mut rm = vec![0xA5u8; SPX_BYTES + 8];
    let mut cml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
    let mut rml: core::ffi::c_ulonglong = 0xDEAD_BEEF;
    let smlen = SPX_BYTES as core::ffi::c_ulonglong;
    let a = unsafe { c(cm.as_mut_ptr(), &mut cml, k.sm.as_ptr(), smlen, k.pk.as_ptr()) };
    let b = unsafe { r(rm.as_mut_ptr(), &mut rml, k.sm.as_ptr(), smlen, k.pk.as_ptr()) };
    assert_eq_dbg("crypto_sign_open(smlen==SPX_BYTES) sentinel", a, b);
    assert_eq_dbg("crypto_sign_open(smlen==SPX_BYTES) == 0", a, 0);
    assert_eq_dbg("crypto_sign_open(smlen==SPX_BYTES) *mlen", cml, rml);
    assert_eq_dbg("crypto_sign_open(smlen==SPX_BYTES) *mlen == 0", cml, 0);
    assert_bytes_eq("crypto_sign_open(smlen==SPX_BYTES) m buffer", &cm, &rm);
}

// ===========================================================================
// Rows 7 / 8 -- seedexpander_init maxlen boundary
// ===========================================================================
#[test]
fn err_07_seedexpander_init_maxlen() {
    let l = libs();
    let (c, r) = l.pair::<FnSeInit>("seedexpander_init");
    let mut rng = Rng::new(0x0007);
    // `unsigned long` is 64-bit here, so values >= 2^32 are representable.
    for &maxlen in &[
        0x1_0000_0000u64,
        0x1_0000_0001,
        0xFFFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
    ] {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut s2 = seed.clone();
        let mut d2 = div.clone();
        // 0xA5-filled so we can prove `ctx` is left untouched.
        let mut cx = AesXofFfi::zeroed();
        let mut rx = AesXofFfi::zeroed();
        unsafe {
            core::ptr::write_bytes(&mut cx as *mut _ as *mut u8, 0xA5, 80);
            core::ptr::write_bytes(&mut rx as *mut _ as *mut u8, 0xA5, 80);
        }
        let a = unsafe {
            c(&mut cx, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as core::ffi::c_ulong)
        };
        let b = unsafe {
            r(&mut rx, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen as core::ffi::c_ulong)
        };
        assert_eq_dbg(&format!("seedexpander_init(maxlen={maxlen:#x}) sentinel"), a, b);
        assert_eq_dbg(
            &format!("seedexpander_init(maxlen={maxlen:#x}) == RNG_BAD_MAXLEN"),
            a,
            RNG_BAD_MAXLEN,
        );
        assert_bytes_eq(
            &format!("seedexpander_init(maxlen={maxlen:#x}) ctx untouched"),
            cx.as_bytes(),
            rx.as_bytes(),
        );
        assert!(
            cx.as_bytes().iter().all(|&x| x == 0xA5),
            "{} C modified ctx on the maxlen error path",
            tag()
        );
    }
}

#[test]
fn err_08_seedexpander_init_ok() {
    let l = libs();
    let (c, r) = l.pair::<FnSeInit>("seedexpander_init");
    let mut rng = Rng::new(0x0008);
    // 0xFFFFFFFF is the largest ACCEPTED value (the check is `>= 0x100000000`).
    for &maxlen in &[0u64, 1, 255, 256, 0xFFFF, 0xFF_FFFF, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut s2 = seed.clone();
        let mut d2 = div.clone();
        let mut cx = AesXofFfi::zeroed();
        let mut rx = AesXofFfi::zeroed();
        unsafe {
            core::ptr::write_bytes(&mut cx as *mut _ as *mut u8, 0xA5, 80);
            core::ptr::write_bytes(&mut rx as *mut _ as *mut u8, 0xA5, 80);
        }
        let a = unsafe {
            c(&mut cx, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as core::ffi::c_ulong)
        };
        let b = unsafe {
            r(&mut rx, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen as core::ffi::c_ulong)
        };
        assert_eq_dbg(&format!("seedexpander_init(maxlen={maxlen:#x}) sentinel"), a, b);
        assert_eq_dbg(
            &format!("seedexpander_init(maxlen={maxlen:#x}) == RNG_SUCCESS"),
            a,
            RNG_SUCCESS,
        );
        // The whole 80-byte struct (incl. padding) must match bit-for-bit.
        assert_bytes_eq(
            &format!("seedexpander_init(maxlen={maxlen:#x}) AES_XOF_struct"),
            cx.as_bytes(),
            rx.as_bytes(),
        );
    }
}

// ===========================================================================
// Rows 9 / 10 / 11 -- seedexpander
// ===========================================================================
fn fresh_xof(rng: &mut Rng, maxlen: u64) -> (AesXofFfi, AesXofFfi) {
    let l = libs();
    let (c, r) = l.pair::<FnSeInit>("seedexpander_init");
    let mut seed = rng.bytes(32);
    let mut div = rng.bytes(8);
    let mut s2 = seed.clone();
    let mut d2 = div.clone();
    let mut cx = AesXofFfi::zeroed();
    let mut rx = AesXofFfi::zeroed();
    unsafe {
        c(&mut cx, seed.as_mut_ptr(), div.as_mut_ptr(), maxlen as core::ffi::c_ulong);
        r(&mut rx, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen as core::ffi::c_ulong);
    }
    assert_bytes_eq("fresh AES_XOF_struct", cx.as_bytes(), rx.as_bytes());
    (cx, rx)
}

#[test]
fn err_09_seedexpander_null_out() {
    let l = libs();
    let (c, r) = l.pair::<FnSe>("seedexpander");
    let mut rng = Rng::new(0x0009);
    // The NULL check comes FIRST, so even an over-long xlen must give -2, not -3.
    for &xlen in &[0u64, 1, 16, 1000, 0xFFFF_FFFF, u64::MAX] {
        let (mut cx, mut rx) = fresh_xof(&mut rng, 256);
        let before = cx.as_bytes().to_vec();
        let a = unsafe { c(&mut cx, core::ptr::null_mut(), xlen as core::ffi::c_ulong) };
        let b = unsafe { r(&mut rx, core::ptr::null_mut(), xlen as core::ffi::c_ulong) };
        assert_eq_dbg(&format!("seedexpander(x=NULL, xlen={xlen}) sentinel"), a, b);
        assert_eq_dbg(
            &format!("seedexpander(x=NULL, xlen={xlen}) == RNG_BAD_OUTBUF"),
            a,
            RNG_BAD_OUTBUF,
        );
        assert_bytes_eq(
            &format!("seedexpander(x=NULL, xlen={xlen}) ctx untouched"),
            cx.as_bytes(),
            rx.as_bytes(),
        );
        assert_bytes_eq(
            &format!("seedexpander(x=NULL, xlen={xlen}) ctx == before"),
            &before,
            cx.as_bytes(),
        );
    }
}

#[test]
fn err_10_seedexpander_reqlen() {
    let l = libs();
    let (c, r) = l.pair::<FnSe>("seedexpander");
    let mut rng = Rng::new(0x0010);
    // The check is `xlen >= ctx->length_remaining`, so requesting EXACTLY the
    // remaining length is an error.
    for &maxlen in &[1u64, 16, 17, 256, 1000] {
        for &xlen in &[maxlen, maxlen + 1, maxlen * 2, 0xFFFF_FFFF] {
            let (mut cx, mut rx) = fresh_xof(&mut rng, maxlen);
            let before = cx.as_bytes().to_vec();
            let mut co = vec![0xA5u8; (xlen.min(4096) as usize) + 8];
            let mut ro = vec![0xA5u8; (xlen.min(4096) as usize) + 8];
            let a = unsafe { c(&mut cx, co.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            let b = unsafe { r(&mut rx, ro.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            assert_eq_dbg(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) sentinel"),
                a,
                b,
            );
            assert_eq_dbg(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) == RNG_BAD_REQ_LEN"),
                a,
                RNG_BAD_REQ_LEN,
            );
            assert_bytes_eq(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) ctx untouched"),
                &before,
                cx.as_bytes(),
            );
            assert_bytes_eq(
                &format!("seedexpander(maxlen={maxlen}, xlen={xlen}) out untouched"),
                &co,
                &ro,
            );
        }
    }
}

#[test]
fn err_11_seedexpander_ok() {
    let l = libs();
    let (c, r) = l.pair::<FnSe>("seedexpander");
    let mut rng = Rng::new(0x0011);
    for &maxlen in &[1u64, 17, 256, 4096, 0xFFFF_FFFF] {
        let (mut cx, mut rx) = fresh_xof(&mut rng, maxlen);
        for &xlen in &[0u64, 1, 15, 16, 17, 100, 500] {
            if xlen >= cx.length_remaining {
                continue;
            }
            let mut co = vec![0xA5u8; xlen as usize + 8];
            let mut ro = vec![0xA5u8; xlen as usize + 8];
            let a = unsafe { c(&mut cx, co.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            let b = unsafe { r(&mut rx, ro.as_mut_ptr(), xlen as core::ffi::c_ulong) };
            assert_eq_dbg(&format!("seedexpander ok (xlen={xlen}) sentinel"), a, b);
            assert_eq_dbg(
                &format!("seedexpander ok (xlen={xlen}) == RNG_SUCCESS"),
                a,
                RNG_SUCCESS,
            );
            assert_bytes_eq(&format!("seedexpander ok (xlen={xlen}) out"), &co, &ro);
            assert_bytes_eq(
                &format!("seedexpander ok (xlen={xlen}) ctx"),
                cx.as_bytes(),
                rx.as_bytes(),
            );
        }
    }
}

// ===========================================================================
// Row 12 -- randombytes always succeeds, incl. xlen == 0
// ===========================================================================
#[test]
fn err_12_randombytes_zero_len() {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let (cb, rb) = l.pair::<FnRb>("randombytes");
    let cd = l.sym::<FnDrbgPtr>(Which::C, "DRBG_ctx");
    let rd = l.sym::<FnDrbgPtr>(Which::R, "DRBG_ctx");
    let mut ent = [0u8; 48];
    for (i, b) in ent.iter_mut().enumerate() {
        *b = i as u8;
    }
    let mut e1 = ent;
    let mut e2 = ent;
    unsafe {
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
    }
    let c0 = unsafe { (**cd).clone() };
    let r0 = unsafe { (**rd).clone() };
    assert_eq_dbg("DRBG after init", c0.clone(), r0);

    // xlen == 0 with a valid buffer, then with a NULL buffer.
    let mut buf = [0xA5u8; 8];
    for (nm, p) in [
        ("valid ptr", buf.as_mut_ptr()),
        ("NULL ptr", core::ptr::null_mut()),
    ] {
        let a = unsafe { cb(p, 0) };
        let b = unsafe { rb(p, 0) };
        assert_eq_dbg(&format!("randombytes({nm}, 0) sentinel"), a, b);
        assert_eq_dbg(&format!("randombytes({nm}, 0) == RNG_SUCCESS"), a, RNG_SUCCESS);
        // C still performs the DRBG update and increments reseed_counter.
        let cv = unsafe { (**cd).clone() };
        let rv = unsafe { (**rd).clone() };
        assert_eq_dbg(&format!("DRBG after randombytes({nm}, 0)"), cv.clone(), rv);
        assert!(
            cv.reseed_counter > c0.reseed_counter,
            "{} reseed_counter did not advance on a zero-length draw",
            tag()
        );
    }
    assert_bytes_eq(
        "randombytes(_, 0) wrote nothing",
        &buf,
        &[0xA5u8; 8],
    );
}

// ===========================================================================
// Row 13 -- randombytes_init with NULL personalization_string
// ===========================================================================
#[test]
fn err_13_randombytes_init_null_pers() {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let cd = l.sym::<FnDrbgPtr>(Which::C, "DRBG_ctx");
    let rd = l.sym::<FnDrbgPtr>(Which::R, "DRBG_ctx");
    let mut rng = Rng::new(0x0013);
    for it in 0..16 {
        let mut ent = [0u8; 48];
        if it > 0 {
            rng.fill(&mut ent);
        }
        let mut e1 = ent;
        let mut e2 = ent;
        unsafe {
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        assert_eq_dbg(
            &format!("randombytes_init(_, NULL) #{it} DRBG"),
            unsafe { (**cd).clone() },
            unsafe { (**rd).clone() },
        );
        // An all-zero personalization string XORs to a no-op, so it must give
        // the same state as NULL.
        let mut z1 = [0u8; 48];
        let mut z2 = [0u8; 48];
        let mut e3 = ent;
        let mut e4 = ent;
        unsafe {
            ci(e3.as_mut_ptr(), z1.as_mut_ptr());
            ri(e4.as_mut_ptr(), z2.as_mut_ptr());
        }
        assert_eq_dbg(
            &format!("randombytes_init(_, zeros) #{it} DRBG"),
            unsafe { (**cd).clone() },
            unsafe { (**rd).clone() },
        );
    }
}

// ===========================================================================
// Rows 14 / 15 / 16 / 17 -- functions that always return 0
// ===========================================================================
#[test]
fn err_14_keypair_always_zero() {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let (c, r) = l.pair::<FnKeypair>("crypto_sign_keypair");
    let mut ent = [0u8; 48];
    for (i, b) in ent.iter_mut().enumerate() {
        *b = i as u8;
    }
    for _ in 0..4 {
        let mut e1 = ent;
        let mut e2 = ent;
        unsafe {
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        let a = unsafe { c(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        let b = unsafe { r(pk.as_mut_ptr(), sk.as_mut_ptr()) };
        assert_eq_dbg("crypto_sign_keypair sentinel", a, b);
        assert_eq_dbg("crypto_sign_keypair == 0", a, 0);
    }
}

#[test]
fn err_15_seed_keypair_always_zero() {
    let l = libs();
    let (c, r) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x0015);
    for it in 0..32 {
        let seed = match it {
            0 => vec![0u8; CRYPTO_SEEDBYTES],
            1 => vec![0xFFu8; CRYPTO_SEEDBYTES],
            _ => rng.bytes(CRYPTO_SEEDBYTES),
        };
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        let a = unsafe { c(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        let b = unsafe { r(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };
        assert_eq_dbg("crypto_sign_seed_keypair sentinel", a, b);
        assert_eq_dbg("crypto_sign_seed_keypair == 0", a, 0);
    }
}

#[test]
fn err_16_signature_always_zero() {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let (kg, _) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let (c, r) = l.pair::<FnSignature>("crypto_sign_signature");
    let mut rng = Rng::new(0x0016);
    for &mlen in &[0usize, 1, 231] {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        unsafe {
            kg(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut ent = [0u8; 48];
        rng.fill(&mut ent);
        let mut e1 = ent;
        let mut e2 = ent;
        unsafe {
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        let mut cs = vec![0u8; SPX_BYTES];
        let mut cl = usize::MAX;
        let a = unsafe { c(cs.as_mut_ptr(), &mut cl, m.as_ptr(), mlen, sk.as_ptr()) };
        let mut e3 = ent;
        let mut e4 = ent;
        unsafe {
            ci(e3.as_mut_ptr(), core::ptr::null_mut());
            ri(e4.as_mut_ptr(), core::ptr::null_mut());
        }
        let mut rs = vec![0u8; SPX_BYTES];
        let mut rl = usize::MAX;
        let b = unsafe { r(rs.as_mut_ptr(), &mut rl, m.as_ptr(), mlen, sk.as_ptr()) };
        assert_eq_dbg(&format!("crypto_sign_signature(mlen={mlen}) sentinel"), a, b);
        assert_eq_dbg(&format!("crypto_sign_signature(mlen={mlen}) == 0"), a, 0);
        assert_eq_dbg(&format!("crypto_sign_signature(mlen={mlen}) *siglen"), cl, rl);
        assert_eq_dbg(
            &format!("crypto_sign_signature(mlen={mlen}) *siglen == SPX_BYTES"),
            cl,
            SPX_BYTES,
        );
    }
}

#[test]
fn err_17_sign_always_zero() {
    let _g = drbg_lock();
    let l = libs();
    let (ci, ri) = l.pair::<FnRbInit>("randombytes_init");
    let (kg, _) = l.pair::<FnSeedKeypair>("crypto_sign_seed_keypair");
    let (c, r) = l.pair::<FnSign>("crypto_sign");
    let mut rng = Rng::new(0x0017);
    for &mlen in &[0usize, 1, 231] {
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pk = vec![0u8; CRYPTO_PUBLICKEYBYTES];
        let mut sk = vec![0u8; CRYPTO_SECRETKEYBYTES];
        unsafe {
            kg(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        }
        let m = rng.bytes(mlen.max(1));
        let mut ent = [0u8; 48];
        rng.fill(&mut ent);
        let mut e1 = ent;
        let mut e2 = ent;
        unsafe {
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
        }
        let mut cs = vec![0u8; SPX_BYTES + mlen];
        let mut cl: core::ffi::c_ulonglong = u64::MAX;
        let a = unsafe {
            c(
                cs.as_mut_ptr(),
                &mut cl,
                m.as_ptr(),
                mlen as core::ffi::c_ulonglong,
                sk.as_ptr(),
            )
        };
        let mut e3 = ent;
        let mut e4 = ent;
        unsafe {
            ci(e3.as_mut_ptr(), core::ptr::null_mut());
            ri(e4.as_mut_ptr(), core::ptr::null_mut());
        }
        let mut rs = vec![0u8; SPX_BYTES + mlen];
        let mut rl: core::ffi::c_ulonglong = u64::MAX;
        let b = unsafe {
            r(
                rs.as_mut_ptr(),
                &mut rl,
                m.as_ptr(),
                mlen as core::ffi::c_ulonglong,
                sk.as_ptr(),
            )
        };
        assert_eq_dbg(&format!("crypto_sign(mlen={mlen}) sentinel"), a, b);
        assert_eq_dbg(&format!("crypto_sign(mlen={mlen}) == 0"), a, 0);
        assert_eq_dbg(&format!("crypto_sign(mlen={mlen}) *smlen"), cl, rl);
        assert_eq_dbg(
            &format!("crypto_sign(mlen={mlen}) *smlen == SPX_BYTES+mlen"),
            cl as usize,
            SPX_BYTES + mlen,
        );
        assert_bytes_eq(&format!("crypto_sign(mlen={mlen}) sm"), &cs, &rs);
    }
}

// ===========================================================================
// Rows 18 / 19 / 20 -- utils.c degenerate lengths
// ===========================================================================
#[test]
fn err_18_ull_to_bytes_zero_len() {
    let l = libs();
    let (c, r) = l.pair::<FnUllToBytes>("SPX_ull_to_bytes");
    let mut rng = Rng::new(0x0018);
    for _ in 0..64 {
        let v = rng.next_u64();
        let mut cb = [0xA5u8; 16];
        let mut rb = [0xA5u8; 16];
        unsafe {
            c(cb.as_mut_ptr(), 0, v);
            r(rb.as_mut_ptr(), 0, v);
        }
        assert_bytes_eq("SPX_ull_to_bytes(outlen=0)", &cb, &rb);
        assert!(
            cb.iter().all(|&x| x == 0xA5),
            "{} SPX_ull_to_bytes(outlen=0) wrote something",
            tag()
        );
    }
}

#[test]
fn err_19_bytes_to_ull_zero_len() {
    let l = libs();
    let (c, r) = l.pair::<FnBytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(0x0019);
    for _ in 0..64 {
        let inp = rng.bytes(16);
        let a = unsafe { c(inp.as_ptr(), 0) };
        let b = unsafe { r(inp.as_ptr(), 0) };
        assert_eq_dbg("SPX_bytes_to_ull(inlen=0) value", a, b);
        assert_eq_dbg("SPX_bytes_to_ull(inlen=0) == 0", a, 0);
    }
}

#[test]
fn err_20_bytes_to_ull_oversized() {
    // inlen > 8 makes the C shift by >= 64 (UB), but it is a real input the C
    // accepts; the Rust must reproduce the value clang -O3 actually computes.
    let l = libs();
    let (c, r) = l.pair::<FnBytesToUll>("SPX_bytes_to_ull");
    let mut rng = Rng::new(0x0020);
    for &inlen in &[9u32, 10, 11, 12, 16, 20, 32] {
        for it in 0..32 {
            let inp = match it {
                0 => vec![0u8; 64],
                1 => vec![0xFFu8; 64],
                2 => vec![0x01u8; 64],
                _ => rng.bytes(64),
            };
            let a = unsafe { c(inp.as_ptr(), inlen) };
            let b = unsafe { r(inp.as_ptr(), inlen) };
            assert_eq_dbg(&format!("SPX_bytes_to_ull(inlen={inlen}) #{it}"), a, b);
        }
    }
}

// ===========================================================================
// Rows 21 / 22 / 23 / 24 -- address setters accept ANY integer (incl.
// out-of-range "enum" values coming across the FFI boundary)
// ===========================================================================
#[test]
fn err_21_set_type_out_of_range() {
    let l = libs();
    let (c, r) = l.pair::<FnAddrU32>("SPX_set_type");
    let mut rng = Rng::new(0x0021);
    // 0..=6 are the documented SPX_ADDR_TYPE_* values. A C enum accepts any
    // int, so 7, 8, 255, 256, 0xFFFFFFFF are all real inputs.
    let mut vals: Vec<u32> = (0..=8u32).collect();
    vals.extend_from_slice(&[
        127, 128, 254, 255, 256, 257, 511, 512, 0xFFFF, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000,
        0xFFFF_FFFE, 0xFFFF_FFFF,
    ]);
    for _ in 0..64 {
        vals.push(rng.next_u32());
    }
    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_bytes_eq(
            &format!("SPX_set_type({v:#x}) -- out-of-range enum value"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
        // Sanity: the C truncates to (unsigned char) at SPX_OFFSET_TYPE.
        assert_eq_dbg(
            &format!("SPX_set_type({v:#x}) truncates to a byte"),
            addr_bytes(&ca)[SPX_OFFSET_TYPE],
            (v & 0xFF) as u8,
        );
    }
}

#[test]
fn err_22_set_layer_addr_truncation() {
    let l = libs();
    let (c, r) = l.pair::<FnAddrU32>("SPX_set_layer_addr");
    let mut rng = Rng::new(0x0022);
    let mut vals: Vec<u32> = vec![
        0,
        1,
        (SPX_D - 1) as u32,
        SPX_D as u32,
        254,
        255,
        256,
        257,
        0xFFFF,
        0xFFFF_FFFF,
    ];
    for _ in 0..64 {
        vals.push(rng.next_u32());
    }
    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_bytes_eq(
            &format!("SPX_set_layer_addr({v:#x})"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
        assert_eq_dbg(
            &format!("SPX_set_layer_addr({v:#x}) truncates"),
            addr_bytes(&ca)[SPX_OFFSET_LAYER],
            (v & 0xFF) as u8,
        );
    }
}

#[test]
fn err_23_addr_byte_truncation() {
    let l = libs();
    let mut rng = Rng::new(0x0023);
    for (name, off) in [
        ("SPX_set_chain_addr", SPX_OFFSET_CHAIN_ADDR),
        ("SPX_set_hash_addr", SPX_OFFSET_HASH_ADDR),
        ("SPX_set_tree_height", SPX_OFFSET_TREE_HGT),
    ] {
        let (c, r) = l.pair::<FnAddrU32>(name);
        let mut vals: Vec<u32> = vec![
            0,
            1,
            SPX_WOTS_W as u32 - 1,
            SPX_WOTS_W as u32,
            SPX_WOTS_LEN as u32,
            SPX_TREE_HEIGHT as u32,
            254,
            255,
            256,
            0xFFFF_FFFF,
        ];
        for _ in 0..64 {
            vals.push(rng.next_u32());
        }
        for v in vals {
            let base = rng.addr();
            let mut ca = base;
            let mut ra = base;
            unsafe {
                c(ca.as_mut_ptr(), v);
                r(ra.as_mut_ptr(), v);
            }
            assert_bytes_eq(&format!("{name}({v:#x})"), &addr_bytes(&ca), &addr_bytes(&ra));
            assert_eq_dbg(
                &format!("{name}({v:#x}) truncates at offset {off}"),
                addr_bytes(&ca)[off],
                (v & 0xFF) as u8,
            );
        }
    }
}

#[test]
fn err_24_set_tree_addr_full_u64() {
    let l = libs();
    let (c, r) = l.pair::<FnAddrU64>("SPX_set_tree_addr");
    let mut rng = Rng::new(0x0024);
    let mut vals: Vec<u64> = vec![
        0,
        1,
        u64::MAX,
        u64::MAX - 1,
        1u64 << 63,
        (1u64 << SPX_TREE_BITS.min(63)) - 1,
        1u64 << SPX_TREE_BITS.min(63),
    ];
    for _ in 0..64 {
        vals.push(rng.next_u64());
    }
    for v in vals {
        let base = rng.addr();
        let mut ca = base;
        let mut ra = base;
        unsafe {
            c(ca.as_mut_ptr(), v);
            r(ra.as_mut_ptr(), v);
        }
        assert_bytes_eq(
            &format!("SPX_set_tree_addr({v:#x})"),
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
        assert_bytes_eq(
            &format!("SPX_set_tree_addr({v:#x}) writes 8 big-endian bytes"),
            &v.to_be_bytes(),
            &addr_bytes(&ca)[SPX_OFFSET_TREE..SPX_OFFSET_TREE + 8],
        );
    }
}

// ===========================================================================
// Rows 25 / 26 -- thash degenerate and branch-selecting inblocks
// ===========================================================================
#[test]
fn err_25_thash_inblocks_zero() {
    let l = libs();
    let (c, r) = l.pair::<FnThash>("SPX_thash");
    let mut rng = Rng::new(0x0025);
    for _ in 0..64 {
        let (cc, rc) = make_ctx(&mut rng);
        let inp = rng.bytes(64);
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut co = vec![0xA5u8; SPX_N + 8];
        let mut ro = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(co.as_mut_ptr(), inp.as_ptr(), 0, &cc, ca.as_mut_ptr());
            r(ro.as_mut_ptr(), inp.as_ptr(), 0, &rc, ra.as_mut_ptr());
        }
        assert_bytes_eq("SPX_thash(inblocks=0) out", &co, &ro);
        assert_bytes_eq(
            "SPX_thash(inblocks=0) addr after",
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

#[test]
fn err_26_thash_inblocks_branch() {
    // For SPX_SHA512/SPX_BLAKE512 configs, inblocks == 1 vs > 1 select DIFFERENT
    // primitives. Confirm both sides agree on BOTH sides of that boundary, and
    // that they really do differ from each other when the branch exists.
    let l = libs();
    let (c, r) = l.pair::<FnThash>("SPX_thash");
    let mut rng = Rng::new(0x0026);
    for _ in 0..32 {
        let (cc, rc) = make_ctx(&mut rng);
        let addr = rng.addr();
        let inp = rng.bytes(4 * SPX_N);
        let mut outs: Vec<Vec<u8>> = Vec::new();
        for nb in [1u32, 2] {
            let mut ca = addr;
            let mut ra = addr;
            let mut co = vec![0xA5u8; SPX_N + 8];
            let mut ro = vec![0xA5u8; SPX_N + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), nb, &cc, ca.as_mut_ptr());
                r(ro.as_mut_ptr(), inp.as_ptr(), nb, &rc, ra.as_mut_ptr());
            }
            assert_bytes_eq(&format!("SPX_thash(inblocks={nb})"), &co, &ro);
            assert_bytes_eq(
                &format!("SPX_thash(inblocks={nb}) addr"),
                &addr_bytes(&ca),
                &addr_bytes(&ra),
            );
            outs.push(co[..SPX_N].to_vec());
        }
        assert_ne!(
            outs[0], outs[1],
            "{} thash(1) and thash(2) must differ",
            tag()
        );
    }
}

// ===========================================================================
// Rows 28 / 29 -- degenerate tree heights (row 27 and row 30 are C UB and are
// deliberately excluded; see ERRORS.md)
// ===========================================================================
#[test]
fn err_28_compute_root_height_one() {
    let l = libs();
    let (c, r) = l.pair::<FnComputeRoot>("SPX_compute_root");
    let mut rng = Rng::new(0x0028);
    for it in 0..64 {
        let (cc, rc) = make_ctx(&mut rng);
        let leaf = rng.bytes(SPX_N);
        let auth = rng.bytes(SPX_N);
        let leaf_idx = match it {
            0 => 0u32,
            1 => 1,
            2 => u32::MAX,
            _ => rng.next_u32(),
        };
        let idx_offset = if it < 3 { 0 } else { rng.next_u32() };
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut cr = vec![0xA5u8; SPX_N + 8];
        let mut rr = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(
                cr.as_mut_ptr(), leaf.as_ptr(), leaf_idx, idx_offset, auth.as_ptr(), 1,
                &cc, ca.as_mut_ptr(),
            );
            r(
                rr.as_mut_ptr(), leaf.as_ptr(), leaf_idx, idx_offset, auth.as_ptr(), 1,
                &rc, ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq(
            &format!("SPX_compute_root(tree_height=1, leaf_idx={leaf_idx})"),
            &cr,
            &rr,
        );
        assert_bytes_eq(
            "SPX_compute_root(tree_height=1) addr after",
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

#[test]
fn err_29_treehash_height_zero() {
    let l = libs();
    let (ct, rt) = l.pair::<FnTreehash>("SPX_treehash");
    let c_leaf = l.sym::<GenLeafFn>(Which::C, "SPX_fors_gen_leafx1");
    let r_leaf = l.sym::<GenLeafFn>(Which::R, "SPX_fors_gen_leafx1");
    let mut rng = Rng::new(0x0029);
    for it in 0..32 {
        let (cc, rc) = make_ctx(&mut rng);
        let leaf_idx = if it == 0 { 0 } else { rng.next_u32() };
        let idx_offset = if it < 2 { 0 } else { rng.next_u32() & 0xFFFF };
        let addr = rng.addr();
        let mut ca = addr;
        let mut ra = addr;
        let mut croot = vec![0xA5u8; SPX_N + 8];
        let mut rroot = vec![0xA5u8; SPX_N + 8];
        let mut cauth = vec![0xA5u8; SPX_N + 8];
        let mut rauth = vec![0xA5u8; SPX_N + 8];
        unsafe {
            ct(
                croot.as_mut_ptr(), cauth.as_mut_ptr(), &cc, leaf_idx, idx_offset, 0,
                Some(*c_leaf), ca.as_mut_ptr(),
            );
            rt(
                rroot.as_mut_ptr(), rauth.as_mut_ptr(), &rc, leaf_idx, idx_offset, 0,
                Some(*r_leaf), ra.as_mut_ptr(),
            );
        }
        assert_bytes_eq("SPX_treehash(tree_height=0) root", &croot, &rroot);
        assert_bytes_eq("SPX_treehash(tree_height=0) auth_path", &cauth, &rauth);
        assert_bytes_eq(
            "SPX_treehash(tree_height=0) addr after",
            &addr_bytes(&ca),
            &addr_bytes(&ra),
        );
    }
}

// ===========================================================================
// Row 31 -- chain_lengths checksum extremes
// ===========================================================================
#[test]
fn err_31_chain_lengths_extremes() {
    let l = libs();
    let (c, r) = l.pair::<FnChainLengths>("SPX_chain_lengths");
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("all 0x00 (max checksum)", vec![0x00u8; SPX_N]),
        ("all 0xFF (zero checksum)", vec![0xFFu8; SPX_N]),
        ("all 0x0F", vec![0x0Fu8; SPX_N]),
        ("all 0xF0", vec![0xF0u8; SPX_N]),
        ("0x01 repeated", vec![0x01u8; SPX_N]),
        ("0xFE repeated", vec![0xFEu8; SPX_N]),
    ];
    for (name, m) in cases {
        let mut cl = vec![0xDEADBEEFu32; SPX_WOTS_LEN + 4];
        let mut rl = vec![0xDEADBEEFu32; SPX_WOTS_LEN + 4];
        unsafe {
            c(cl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
            r(rl.as_mut_ptr() as *mut core::ffi::c_uint, m.as_ptr());
        }
        assert_eq_dbg(&format!("SPX_chain_lengths [{name}]"), &cl, &rl);
    }
}

// ===========================================================================
// Rows 32 / 33 -- wots_gen_leafx1's wots_sign_leaf sentinel
// ===========================================================================
#[test]
fn err_32_wots_gen_leafx1_no_sign() {
    // wots_sign_leaf == ~0u => wots_k_mask = ~0 => `wots_sig` NEVER written, so
    // a NULL `wots_sig` is safe (this is what merkle_gen_root relies on).
    let l = libs();
    let (c, r) = l.pair::<FnWotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(0x0032);
    for _ in 0..32 {
        let (cc, rc) = make_ctx(&mut rng);
        let steps: Vec<u32> = (0..SPX_WOTS_LEN)
            .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
            .collect();
        let base = rng.addr();
        let leaf_idx = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);

        let mut ci: LeafInfoX1Ffi = unsafe { core::mem::zeroed() };
        ci.wots_sig = core::ptr::null_mut(); // NULL is safe in this mode
        ci.wots_sign_leaf = u32::MAX;
        ci.wots_steps = steps.as_ptr();
        ci.leaf_addr = base;
        ci.pk_addr = base;
        let mut ri: LeafInfoX1Ffi = unsafe { core::mem::zeroed() };
        ri.wots_sig = core::ptr::null_mut();
        ri.wots_sign_leaf = u32::MAX;
        ri.wots_steps = steps.as_ptr();
        ri.leaf_addr = base;
        ri.pk_addr = base;

        let mut cd = vec![0xA5u8; SPX_N + 8];
        let mut rd = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(cd.as_mut_ptr(), &cc, leaf_idx, &mut ci);
            r(rd.as_mut_ptr(), &rc, leaf_idx, &mut ri);
        }
        assert_bytes_eq("SPX_wots_gen_leafx1(wots_sign_leaf=~0, wots_sig=NULL)", &cd, &rd);
        assert_bytes_eq(
            "SPX_wots_gen_leafx1(~0) leaf_addr after",
            &addr_bytes(&ci.leaf_addr),
            &addr_bytes(&ri.leaf_addr),
        );
    }
}

#[test]
fn err_33_wots_gen_leafx1_sign() {
    let l = libs();
    let (c, r) = l.pair::<FnWotsGenLeafX1>("SPX_wots_gen_leafx1");
    let mut rng = Rng::new(0x0033);
    for it in 0..32 {
        let (cc, rc) = make_ctx(&mut rng);
        // Include the extremes of wots_steps: all 0 and all w-1.
        let steps: Vec<u32> = match it {
            0 => vec![0u32; SPX_WOTS_LEN],
            1 => vec![SPX_WOTS_W as u32 - 1; SPX_WOTS_LEN],
            _ => (0..SPX_WOTS_LEN)
                .map(|_| rng.next_u32() % SPX_WOTS_W as u32)
                .collect(),
        };
        let base = rng.addr();
        let leaf_idx = rng.next_u32() & ((1u32 << SPX_TREE_HEIGHT) - 1);
        let mut c_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];
        let mut r_sig = vec![0xA5u8; SPX_WOTS_BYTES + 8];

        let mut ci: LeafInfoX1Ffi = unsafe { core::mem::zeroed() };
        ci.wots_sig = c_sig.as_mut_ptr();
        ci.wots_sign_leaf = leaf_idx;
        ci.wots_steps = steps.as_ptr();
        ci.leaf_addr = base;
        ci.pk_addr = base;
        let mut ri: LeafInfoX1Ffi = unsafe { core::mem::zeroed() };
        ri.wots_sig = r_sig.as_mut_ptr();
        ri.wots_sign_leaf = leaf_idx;
        ri.wots_steps = steps.as_ptr();
        ri.leaf_addr = base;
        ri.pk_addr = base;

        let mut cd = vec![0xA5u8; SPX_N + 8];
        let mut rd = vec![0xA5u8; SPX_N + 8];
        unsafe {
            c(cd.as_mut_ptr(), &cc, leaf_idx, &mut ci);
            r(rd.as_mut_ptr(), &rc, leaf_idx, &mut ri);
        }
        assert_bytes_eq("SPX_wots_gen_leafx1(sign) dest", &cd, &rd);
        assert_bytes_eq("SPX_wots_gen_leafx1(sign) wots_sig", &c_sig, &r_sig);
        assert!(
            c_sig[..SPX_WOTS_BYTES].iter().any(|&b| b != 0xA5),
            "{} sign mode did not write wots_sig",
            tag()
        );
    }
}

// ===========================================================================
// Row 36 -- gen_message_random / hash_message with mlen == 0
// ===========================================================================
#[test]
fn err_36_hash_message_mlen_zero() {
    let l = libs();
    let (cg, rg) = l.pair::<FnGenMsgRandom>("SPX_gen_message_random");
    let (ch, rh) = l.pair::<FnHashMessage>("SPX_hash_message");
    let mut rng = Rng::new(0x0036);
    for _ in 0..32 {
        let (cc, rc) = make_ctx(&mut rng);
        let sk_prf = rng.bytes(SPX_N);
        let optrand = rng.bytes(SPX_N);
        let rr = rng.bytes(SPX_N);
        let pk = rng.bytes(SPX_PK_BYTES);
        let empty: [u8; 1] = [0];

        // 128 bytes: for the BLAKE backend the C writes the FULL blakeX digest.
        let mut co = vec![0xA5u8; 128];
        let mut ro = vec![0xA5u8; 128];
        unsafe {
            cg(co.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), empty.as_ptr(), 0, &cc);
            rg(ro.as_mut_ptr(), sk_prf.as_ptr(), optrand.as_ptr(), empty.as_ptr(), 0, &rc);
        }
        assert_bytes_eq("SPX_gen_message_random(mlen=0)", &co, &ro);

        let mut cd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
        let mut rd = vec![0xA5u8; SPX_FORS_MSG_BYTES + 8];
        let mut ct: u64 = 0xDEAD;
        let mut rt: u64 = 0xDEAD;
        let mut cli: u32 = 0xDEAD;
        let mut rli: u32 = 0xDEAD;
        unsafe {
            ch(cd.as_mut_ptr(), &mut ct, &mut cli, rr.as_ptr(), pk.as_ptr(), empty.as_ptr(), 0, &cc);
            rh(rd.as_mut_ptr(), &mut rt, &mut rli, rr.as_ptr(), pk.as_ptr(), empty.as_ptr(), 0, &rc);
        }
        assert_bytes_eq("SPX_hash_message(mlen=0) digest", &cd, &rd);
        assert_eq_dbg("SPX_hash_message(mlen=0) *tree", ct, rt);
        assert_eq_dbg("SPX_hash_message(mlen=0) *leaf_idx", cli, rli);
    }
}

// ===========================================================================
// Row 37 -- MGF1 outlen edge cases (backend-specific symbol names)
// ===========================================================================
#[test]
fn err_37_mgf1_outlen_edge() {
    let l = libs();
    let names: &[&str] = if cfg!(spx_backend = "blake") {
        &["SPX_blake256_mgf1", "SPX_blake512_mgf1"]
    } else if cfg!(spx_backend = "sha2") {
        &["SPX_mgf1_256", "SPX_mgf1_512"]
    } else {
        &[] // shake and haraka expose no mgf1
    };
    if names.is_empty() {
        return;
    }
    let mut rng = Rng::new(0x0037);
    for name in names {
        let (c, r) = l.pair::<FnMgf1>(name);
        for &outlen in &[0usize, 1, 31, 32, 33, 63, 64, 65, 95, 96, 97, 128, 129] {
            for &inlen in &[0usize, 1, 32, 64] {
                let inp = rng.bytes(inlen.max(1));
                let mut co = vec![0xA5u8; outlen + 8];
                let mut ro = vec![0xA5u8; outlen + 8];
                unsafe {
                    c(
                        co.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        inp.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                    r(
                        ro.as_mut_ptr(),
                        outlen as core::ffi::c_ulong,
                        inp.as_ptr(),
                        inlen as core::ffi::c_ulong,
                    );
                }
                assert_bytes_eq(
                    &format!("{name}(outlen={outlen}, inlen={inlen})"),
                    &co,
                    &ro,
                );
                if outlen == 0 {
                    assert!(
                        co.iter().all(|&x| x == 0xA5),
                        "{} {name}(outlen=0) wrote something",
                        tag()
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Rows 38 / 39 / 40 -- backend hash length boundaries and zero-length
// squeeze / zero-block absorb
// ===========================================================================
#[cfg(spx_backend = "blake")]
#[test]
fn err_38_hash_length_boundaries() {
    type FnHash =
        unsafe extern "C" fn(*mut u8, *const u8, core::ffi::c_ulonglong) -> core::ffi::c_int;
    let l = libs();
    let mut rng = Rng::new(0x0038);
    for (name, outb) in [("blake256", 32usize), ("blake512", 64usize)] {
        let (c, r) = l.pair::<FnHash>(name);
        for &n in &[
            0usize, 1, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113, 118, 119, 120, 127, 128, 129,
        ] {
            let inp = rng.bytes(n.max(1));
            let mut co = vec![0xA5u8; outb + 8];
            let mut ro = vec![0xA5u8; outb + 8];
            let a = unsafe { c(co.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong) };
            let b = unsafe { r(ro.as_mut_ptr(), inp.as_ptr(), n as core::ffi::c_ulonglong) };
            assert_eq_dbg(&format!("{name}(inlen={n}) retval"), a, b);
            assert_bytes_eq(&format!("{name}(inlen={n})"), &co, &ro);
        }
    }
}

#[cfg(spx_backend = "sha2")]
#[test]
fn err_38_hash_length_boundaries() {
    type FnHash = unsafe extern "C" fn(*mut u8, *const u8, usize);
    let l = libs();
    let mut rng = Rng::new(0x0038);
    for (name, outb) in [("sha256", 32usize), ("sha512", 64usize)] {
        let (c, r) = l.pair::<FnHash>(name);
        for &n in &[
            0usize, 1, 54, 55, 56, 57, 63, 64, 65, 110, 111, 112, 113, 119, 120, 127, 128, 129,
        ] {
            let inp = rng.bytes(n.max(1));
            let mut co = vec![0xA5u8; outb + 8];
            let mut ro = vec![0xA5u8; outb + 8];
            unsafe {
                c(co.as_mut_ptr(), inp.as_ptr(), n);
                r(ro.as_mut_ptr(), inp.as_ptr(), n);
            }
            assert_bytes_eq(&format!("{name}(inlen={n})"), &co, &ro);
        }
    }
}

#[cfg(spx_backend = "shake")]
#[test]
fn err_38_hash_length_boundaries() {
    type FnShake = unsafe extern "C" fn(*mut u8, usize, *const u8, usize);
    let l = libs();
    let (c, r) = l.pair::<FnShake>("shake256");
    let mut rng = Rng::new(0x0038);
    for &n in &[0usize, 1, 134, 135, 136, 137, 271, 272, 273] {
        for &outlen in &[0usize, 1, 135, 136, 137, 272] {
            let inp = rng.bytes(n.max(1));
            let mut co = vec![0xA5u8; outlen + 8];
            let mut ro = vec![0xA5u8; outlen + 8];
            unsafe {
                c(co.as_mut_ptr(), outlen, inp.as_ptr(), n);
                r(ro.as_mut_ptr(), outlen, inp.as_ptr(), n);
            }
            assert_bytes_eq(&format!("shake256(inlen={n}, outlen={outlen})"), &co, &ro);
        }
    }
}

#[cfg(spx_backend = "haraka")]
#[test]
fn err_38_hash_length_boundaries() {
    type FnTweak = unsafe extern "C" fn(*mut SpxCtxFfi);
    type FnS = unsafe extern "C" fn(
        *mut u8,
        core::ffi::c_ulonglong,
        *const u8,
        core::ffi::c_ulonglong,
        *const SpxCtxFfi,
    );
    let l = libs();
    let (ct, rt) = l.pair::<FnTweak>("SPX_tweak_constants");
    let (c, r) = l.pair::<FnS>("SPX_haraka_S");
    let mut rng = Rng::new(0x0038);
    for &n in &[0usize, 1, 31, 32, 33, 63, 64, 65] {
        for &outlen in &[0usize, 1, 31, 32, 33, 64] {
            let mut cc = SpxCtxFfi::zeroed();
            rng.fill(&mut cc.pub_seed);
            rng.fill(&mut cc.sk_seed);
            let mut rc = cc.clone();
            unsafe {
                ct(&mut cc);
                rt(&mut rc);
            }
            let inp = rng.bytes(n.max(1));
            let mut co = vec![0xA5u8; outlen + 8];
            let mut ro = vec![0xA5u8; outlen + 8];
            unsafe {
                c(
                    co.as_mut_ptr(),
                    outlen as core::ffi::c_ulonglong,
                    inp.as_ptr(),
                    n as core::ffi::c_ulonglong,
                    &cc,
                );
                r(
                    ro.as_mut_ptr(),
                    outlen as core::ffi::c_ulonglong,
                    inp.as_ptr(),
                    n as core::ffi::c_ulonglong,
                    &rc,
                );
            }
            assert_bytes_eq(&format!("SPX_haraka_S(inlen={n}, outlen={outlen})"), &co, &ro);
        }
    }
}

#[cfg(spx_backend = "shake")]
#[test]
fn err_39_squeeze_zero_len() {
    type FnIncInit = unsafe extern "C" fn(*mut u64);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u64, *const u8, usize);
    type FnIncFinal = unsafe extern "C" fn(*mut u64);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u64);
    let l = libs();
    let (ci, ri) = l.pair::<FnIncInit>("shake256_inc_init");
    let (ca, ra) = l.pair::<FnIncAbsorb>("shake256_inc_absorb");
    let (cf, rf) = l.pair::<FnIncFinal>("shake256_inc_finalize");
    let (cq, rq) = l.pair::<FnIncSqueeze>("shake256_inc_squeeze");
    let mut rng = Rng::new(0x0039);
    let raw = |s: &[u64; 26]| unsafe { core::slice::from_raw_parts(s.as_ptr() as *const u8, 208) };
    for _ in 0..16 {
        let inp = rng.bytes(64);
        let mut cs = [0u64; 26];
        let mut rs = [0u64; 26];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
            ca(cs.as_mut_ptr(), inp.as_ptr(), 64);
            ra(rs.as_mut_ptr(), inp.as_ptr(), 64);
            // Zero-length absorb must behave identically too.
            ca(cs.as_mut_ptr(), inp.as_ptr(), 0);
            ra(rs.as_mut_ptr(), inp.as_ptr(), 0);
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        assert_bytes_eq("shake256 state before zero squeeze", raw(&cs), raw(&rs));
        let mut co = [0xA5u8; 8];
        let mut ro = [0xA5u8; 8];
        unsafe {
            cq(co.as_mut_ptr(), 0, cs.as_mut_ptr());
            rq(ro.as_mut_ptr(), 0, rs.as_mut_ptr());
        }
        assert_bytes_eq("shake256_inc_squeeze(0) out", &co, &ro);
        assert_bytes_eq("shake256_inc_squeeze(0) state", raw(&cs), raw(&rs));
    }
}

#[cfg(spx_backend = "haraka")]
#[test]
fn err_39_squeeze_zero_len() {
    type FnTweak = unsafe extern "C" fn(*mut SpxCtxFfi);
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncAbsorb = unsafe extern "C" fn(*mut u8, *const u8, usize, *const SpxCtxFfi);
    type FnIncFinal = unsafe extern "C" fn(*mut u8);
    type FnIncSqueeze = unsafe extern "C" fn(*mut u8, usize, *mut u8, *const SpxCtxFfi);
    let l = libs();
    let (ctw, rtw) = l.pair::<FnTweak>("SPX_tweak_constants");
    let (ci, ri) = l.pair::<FnIncInit>("SPX_haraka_S_inc_init");
    let (ca, ra) = l.pair::<FnIncAbsorb>("SPX_haraka_S_inc_absorb");
    let (cf, rf) = l.pair::<FnIncFinal>("SPX_haraka_S_inc_finalize");
    let (cq, rq) = l.pair::<FnIncSqueeze>("SPX_haraka_S_inc_squeeze");
    let mut rng = Rng::new(0x0039);
    for _ in 0..16 {
        let mut cc = SpxCtxFfi::zeroed();
        rng.fill(&mut cc.pub_seed);
        rng.fill(&mut cc.sk_seed);
        let mut rc = cc.clone();
        unsafe {
            ctw(&mut cc);
            rtw(&mut rc);
        }
        let inp = rng.bytes(64);
        let mut cs = [0u8; 65];
        let mut rs = [0u8; 65];
        unsafe {
            ci(cs.as_mut_ptr());
            ri(rs.as_mut_ptr());
            ca(cs.as_mut_ptr(), inp.as_ptr(), 64, &cc);
            ra(rs.as_mut_ptr(), inp.as_ptr(), 64, &rc);
            ca(cs.as_mut_ptr(), inp.as_ptr(), 0, &cc);
            ra(rs.as_mut_ptr(), inp.as_ptr(), 0, &rc);
            cf(cs.as_mut_ptr());
            rf(rs.as_mut_ptr());
        }
        assert_bytes_eq("haraka_S state before zero squeeze", &cs, &rs);
        let mut co = [0xA5u8; 8];
        let mut ro = [0xA5u8; 8];
        unsafe {
            cq(co.as_mut_ptr(), 0, cs.as_mut_ptr(), &cc);
            rq(ro.as_mut_ptr(), 0, rs.as_mut_ptr(), &rc);
        }
        assert_bytes_eq("haraka_S_inc_squeeze(0) out", &co, &ro);
        assert_bytes_eq("haraka_S_inc_squeeze(0) state", &cs, &rs);
    }
}

#[cfg(any(spx_backend = "blake", spx_backend = "sha2"))]
#[test]
fn err_39_squeeze_zero_len() {
    // blake and sha2 expose no incremental *squeeze*; the equivalent
    // zero-length surface is `mgf1(outlen=0)`, covered by err_37, and
    // `blake*_update(len=0)` / `sha*_inc_blocks(0)`, covered by err_40.
}

#[cfg(spx_backend = "sha2")]
#[test]
fn err_40_inc_blocks_zero() {
    type FnIncInit = unsafe extern "C" fn(*mut u8);
    type FnIncBlocks = unsafe extern "C" fn(*mut u8, *const u8, usize);
    let l = libs();
    let mut rng = Rng::new(0x0040);
    for (init, blocks, slen) in [
        ("sha256_inc_init", "sha256_inc_blocks", 40usize),
        ("sha512_inc_init", "sha512_inc_blocks", 72),
    ] {
        let (ci, ri) = l.pair::<FnIncInit>(init);
        let (cb, rb) = l.pair::<FnIncBlocks>(blocks);
        for _ in 0..16 {
            let inp = rng.bytes(256);
            let mut cs = vec![0xA5u8; slen];
            let mut rs = vec![0xA5u8; slen];
            unsafe {
                ci(cs.as_mut_ptr());
                ri(rs.as_mut_ptr());
            }
            let before = cs.clone();
            unsafe {
                cb(cs.as_mut_ptr(), inp.as_ptr(), 0);
                rb(rs.as_mut_ptr(), inp.as_ptr(), 0);
            }
            assert_bytes_eq(&format!("{blocks}(0) state"), &cs, &rs);
            assert_bytes_eq(&format!("{blocks}(0) leaves state unchanged"), &before, &cs);
        }
    }
}

#[cfg(spx_backend = "blake")]
#[test]
fn err_40_inc_blocks_zero() {
    // BLAKE's equivalent of a zero-length incremental step is
    // `blake*_update(S, data, 0)`, which -- per blake256.c line 327 -- WIPES
    // `S->buflen`. That quirk must be reproduced exactly.
    type FnInit256 = unsafe extern "C" fn(*mut BlakeState256Ffi);
    type FnUpd256 = unsafe extern "C" fn(*mut BlakeState256Ffi, *const u8, core::ffi::c_ulonglong);
    type FnFin256 = unsafe extern "C" fn(*mut BlakeState256Ffi, *mut u8);
    let l = libs();
    let (ci, ri) = l.pair::<FnInit256>("blake256_init");
    let (cu, ru) = l.pair::<FnUpd256>("blake256_update");
    let (cf, rf) = l.pair::<FnFin256>("blake256_final");
    let mut rng = Rng::new(0x0040);
    let raw = |s: &BlakeState256Ffi| unsafe {
        core::slice::from_raw_parts(
            s as *const _ as *const u8,
            core::mem::size_of::<BlakeState256Ffi>(),
        )
    };
    for &partial in &[0usize, 1, 10, 55, 63, 64, 65] {
        let inp = rng.bytes(partial.max(1));
        let mut cs: BlakeState256Ffi = unsafe { core::mem::zeroed() };
        let mut rs: BlakeState256Ffi = unsafe { core::mem::zeroed() };
        unsafe {
            ci(&mut cs);
            ri(&mut rs);
            cu(&mut cs, inp.as_ptr(), (partial * 8) as core::ffi::c_ulonglong);
            ru(&mut rs, inp.as_ptr(), (partial * 8) as core::ffi::c_ulonglong);
        }
        assert_bytes_eq(&format!("blake256 state after {partial}B"), raw(&cs), raw(&rs));
        unsafe {
            cu(&mut cs, inp.as_ptr(), 0);
            ru(&mut rs, inp.as_ptr(), 0);
        }
        assert_bytes_eq(
            &format!("blake256 state after zero-length update (partial={partial})"),
            raw(&cs),
            raw(&rs),
        );
        assert_eq_dbg(
            &format!("blake256_update(len=0) wipes buflen (partial={partial})"),
            cs.buflen,
            0,
        );
        let mut cd = [0xA5u8; 40];
        let mut rd = [0xA5u8; 40];
        unsafe {
            cf(&mut cs, cd.as_mut_ptr());
            rf(&mut rs, rd.as_mut_ptr());
        }
        assert_bytes_eq(
            &format!("blake256 digest after zero-length update (partial={partial})"),
            &cd,
            &rd,
        );
    }
}

#[cfg(any(spx_backend = "shake", spx_backend = "haraka"))]
#[test]
fn err_40_inc_blocks_zero() {
    // Covered by err_39 (zero-length absorb on the incremental sponge).
}
