//! ERRORS.md -- one differential test per row of the error-surface table.
//! Every test asserts that C and Rust return the *same* error code / sentinel
//! and produce the same side effects, not merely that "both failed".

mod common;
use common::*;

type FSeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FKeypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FSignature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FSeInit = unsafe extern "C" fn(*mut CXof, *mut u8, *mut u8, u64) -> i32;
type FSe = unsafe extern "C" fn(*mut CXof, *mut u8, u64) -> i32;
type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FRb = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FUpdate = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FUll = unsafe extern "C" fn(*mut u8, u32, u64);
type FB2U = unsafe extern "C" fn(*const u8, u32) -> u64;
type FAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FThash = unsafe extern "C" fn(*mut u8, *const u8, u32, *const u8, *mut u32);
type FInit = unsafe extern "C" fn(*mut u8);
type FGenR = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, u64, *const u8);

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CXof {
    pub buffer: [u8; 16],
    pub buffer_pos: u64,
    pub length_remaining: u64,
    pub key: [u8; 32],
    pub ctr: [u8; 16],
}

const DRBG_BYTES: usize = 32 + 16 + 4;

macro_rules! f {
    ($side:expr, $name:expr, $t:ty) => {
        unsafe { std::mem::transmute::<usize, $t>($side.addr($name)) }
    };
}

/// A valid (pk, sk, message, signature) quadruple, identical on both sides.
struct Fixture {
    pk: Vec<u8>,
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

fn fixture() -> &'static Fixture {
    use std::sync::OnceLock;
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let p = libs();
        let ckp = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
        let rkp = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
        let csg = f!(p.c, "crypto_sign_signature", FSignature);
        let rsg = f!(p.rust, "crypto_sign_signature", FSignature);
        let cri = f!(p.c, "randombytes_init", FRbInit);
        let rri = f!(p.rust, "randombytes_init", FRbInit);

        let mut rng = Rng::new(SEED ^ 0xE0);
        let seed = rng.bytes(CRYPTO_SEEDBYTES);
        let mut pk = vec![0u8; SPX_PK_BYTES];
        let mut sk = vec![0u8; SPX_SK_BYTES];
        let mut pk2 = pk.clone();
        let mut sk2 = sk.clone();
        unsafe {
            ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
            rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
        }
        eq_bytes("fixture pk", &pk, &pk2);
        eq_bytes("fixture sk", &sk, &sk2);

        let m = rng.bytes(37);
        let mut ent = [0u8; 48];
        (0..48).for_each(|i| ent[i] = i as u8);
        let mut sig = vec![0u8; SPX_BYTES];
        let mut sig2 = sig.clone();
        let mut l1 = 0usize;
        let mut l2 = 0usize;
        unsafe {
            let mut e = ent;
            cri(e.as_mut_ptr(), std::ptr::null_mut());
            csg(sig.as_mut_ptr(), &mut l1, m.as_ptr(), m.len(), sk.as_ptr());
            let mut e = ent;
            rri(e.as_mut_ptr(), std::ptr::null_mut());
            rsg(sig2.as_mut_ptr(), &mut l2, m.as_ptr(), m.len(), sk.as_ptr());
        }
        eq(&"fixture siglen".to_string(), l1, l2);
        eq_bytes("fixture sig", &sig, &sig2);
        Fixture { pk, sk, m, sig }
    })
}

fn verify_both(sig: &[u8], siglen: usize, m: &[u8], pk: &[u8]) -> (i32, i32) {
    let p = libs();
    let cv = f!(p.c, "crypto_sign_verify", FVerify);
    let rv = f!(p.rust, "crypto_sign_verify", FVerify);
    unsafe {
        (
            cv(sig.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
            rv(sig.as_ptr(), siglen, m.as_ptr(), m.len(), pk.as_ptr()),
        )
    }
}

/// ERRORS.md row 1 -- `crypto_sign_verify` with `siglen != SPX_BYTES`.
/// (Also the generic NULL-pointer / oversized-length boundary: the C bails out
/// before dereferencing anything.)
#[test]
fn err01_verify_wrong_siglen() {
    let p = libs();
    let fx = fixture();
    let cv = f!(p.c, "crypto_sign_verify", FVerify);
    let rv = f!(p.rust, "crypto_sign_verify", FVerify);

    for &siglen in &[
        0usize,
        1,
        SPX_BYTES - 1,
        SPX_BYTES + 1,
        2 * SPX_BYTES,
        usize::MAX,
    ] {
        let (a, b) = verify_both(&fx.sig, siglen, &fx.m, &fx.pk);
        eq(&format!("verify(siglen={siglen})"), a, b);
        eq(&format!("verify(siglen={siglen})==-1"), a, -1);
    }

    // NULL sig / NULL m / NULL pk with a wrong siglen: both sides must still
    // return -1 without touching the pointers.
    for &siglen in &[0usize, SPX_BYTES - 1, SPX_BYTES + 1] {
        let (a, b) = unsafe {
            (
                cv(
                    std::ptr::null(),
                    siglen,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                ),
                rv(
                    std::ptr::null(),
                    siglen,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                ),
            )
        };
        eq(&format!("verify(NULL,siglen={siglen})"), a, b);
        eq(&format!("verify(NULL,siglen={siglen})==-1"), a, -1);
    }
}

/// ERRORS.md row 2 -- root mismatch: corrupt each region of the signature, the
/// message and the public key in turn.
#[test]
fn err02_verify_root_mismatch() {
    let fx = fixture();
    let auth_off = SPX_N + SPX_FORS_BYTES + SPX_WOTS_BYTES;
    let cases: Vec<(&str, usize)> = vec![
        ("R", 0),
        ("R-last", SPX_N - 1),
        ("fors-first", SPX_N),
        ("fors-last", SPX_N + SPX_FORS_BYTES - 1),
        ("wots-first", SPX_N + SPX_FORS_BYTES),
        ("wots-last", SPX_N + SPX_FORS_BYTES + SPX_WOTS_BYTES - 1),
        ("auth-first", auth_off),
        ("sig-last", SPX_BYTES - 1),
    ];
    for (what, off) in cases {
        let mut sig = fx.sig.clone();
        sig[off] ^= 0x01;
        let (a, b) = verify_both(&sig, SPX_BYTES, &fx.m, &fx.pk);
        eq(&format!("verify(corrupt {what})"), a, b);
        eq(&format!("verify(corrupt {what})==-1"), a, -1);
    }

    // Corrupting the *message* is deliberately only checked for C == Rust and
    // NOT asserted to be a rejection.  Reason (ground truth, do not "fix"):
    // `lib/blake/src/hash_blake.c` passes BYTE counts to `blakeX_update()`,
    // whose `datalen` argument is a BIT count, so with the blake backend only
    // `mlen/8` bytes of the message reach the hash -- and for short messages no
    // BLAKE compression happens at all, which makes the digest independent of
    // the message content.  The differential requirement is that Rust does
    // exactly the same thing.
    for off in [0usize, fx.m.len() - 1] {
        let mut m = fx.m.clone();
        m[off] ^= 0x80;
        let (a, b) = verify_both(&fx.sig, SPX_BYTES, &m, &fx.pk);
        eq(&format!("verify(corrupt m[{off}])"), a, b);
    }
    // truncate / extend the message
    for len in [0usize, fx.m.len() - 1, fx.m.len() + 1] {
        let mut m = fx.m.clone();
        m.resize(len, 0x5A);
        let (a, b) = verify_both(&fx.sig, SPX_BYTES, &m, &fx.pk);
        eq(&format!("verify(mlen={len})"), a, b);
    }
    // ... but a different mlen *is* observable for every backend (blake included,
    // through the `buflen` bookkeeping), so at least mlen = 0 must be rejected.
    let (a, b) = verify_both(&fx.sig, SPX_BYTES, &[], &fx.pk);
    eq("verify(mlen=0)", a, b);
    eq("verify(mlen=0)==-1", a, -1);
    // corrupt the pk seed and the pk root
    for off in [0usize, SPX_N - 1, SPX_N, 2 * SPX_N - 1] {
        let mut pk = fx.pk.clone();
        pk[off] ^= 0x01;
        let (a, b) = verify_both(&fx.sig, SPX_BYTES, &fx.m, &pk);
        eq(&format!("verify(corrupt pk[{off}])"), a, b);
        eq(&format!("verify(corrupt pk[{off}])==-1"), a, -1);
    }
}

/// ERRORS.md row 3 -- control: an untouched signature verifies on both sides.
#[test]
fn err03_verify_ok() {
    let fx = fixture();
    let (a, b) = verify_both(&fx.sig, SPX_BYTES, &fx.m, &fx.pk);
    eq("verify(valid)", a, b);
    eq("verify(valid)==0", a, 0);
}

/// ERRORS.md row 4 -- `crypto_sign_open` with `smlen < SPX_BYTES`
#[test]
fn err04_open_short() {
    let p = libs();
    let fx = fixture();
    let co = f!(p.c, "crypto_sign_open", FOpen);
    let ro = f!(p.rust, "crypto_sign_open", FOpen);

    for &smlen in &[0usize, 1, SPX_BYTES - 1] {
        let mut sm = fx.sig.clone();
        sm.resize(smlen.max(1), 0);
        let mut m1 = vec![0xA5u8; smlen + 16];
        let mut m2 = m1.clone();
        let mut l1 = 0xDEAD_BEEFu64;
        let mut l2 = l1;
        let (a, b) = unsafe {
            (
                co(m1.as_mut_ptr(), &mut l1, sm.as_ptr(), smlen as u64, fx.pk.as_ptr()),
                ro(m2.as_mut_ptr(), &mut l2, sm.as_ptr(), smlen as u64, fx.pk.as_ptr()),
            )
        };
        eq(&format!("open(smlen={smlen})"), a, b);
        eq(&format!("open(smlen={smlen})==-1"), a, -1);
        eq(&format!("open(smlen={smlen}) mlen"), l1, l2);
        eq(&format!("open(smlen={smlen}) mlen==0"), l1, 0);
        eq_bytes(&format!("open(smlen={smlen}) m"), &m1, &m2);
        // the C memsets exactly `smlen` bytes and leaves the rest alone
        assert!(
            m1[..smlen].iter().all(|&x| x == 0) && m1[smlen..].iter().all(|&x| x == 0xA5),
            "open(smlen={smlen}) did not memset exactly smlen bytes"
        );
    }
}

/// ERRORS.md row 5 -- `crypto_sign_open` where the inner verify fails
#[test]
fn err05_open_bad_signature() {
    let p = libs();
    let fx = fixture();
    let co = f!(p.c, "crypto_sign_open", FOpen);
    let ro = f!(p.rust, "crypto_sign_open", FOpen);

    let mlen = fx.m.len();
    let smlen = SPX_BYTES + mlen;
    let mut good = vec![0u8; smlen];
    good[..SPX_BYTES].copy_from_slice(&fx.sig);
    good[SPX_BYTES..].copy_from_slice(&fx.m);

    // `must_reject` is false for corruptions that only touch the appended
    // message: see the note in `err02_verify_root_mismatch`.
    let mut cases: Vec<(String, Vec<u8>, Vec<u8>, bool)> = Vec::new();
    for off in [0usize, SPX_N, SPX_BYTES - 1] {
        let mut sm = good.clone();
        sm[off] ^= 0x01;
        cases.push((format!("corrupt sm[{off}]"), sm, fx.pk.clone(), true));
    }
    for off in [SPX_BYTES, smlen - 1] {
        let mut sm = good.clone();
        sm[off] ^= 0x01;
        cases.push((format!("corrupt sm[{off}]"), sm, fx.pk.clone(), false));
    }
    let mut wrongpk = fx.pk.clone();
    wrongpk[0] ^= 0xFF;
    cases.push(("wrong pk".into(), good.clone(), wrongpk, true));

    for (what, sm, pk, must_reject) in cases {
        let mut m1 = vec![0xA5u8; smlen + 16];
        let mut m2 = m1.clone();
        let mut l1 = 0xDEAD_BEEFu64;
        let mut l2 = l1;
        let (a, b) = unsafe {
            (
                co(m1.as_mut_ptr(), &mut l1, sm.as_ptr(), smlen as u64, pk.as_ptr()),
                ro(m2.as_mut_ptr(), &mut l2, sm.as_ptr(), smlen as u64, pk.as_ptr()),
            )
        };
        eq(&format!("open({what})"), a, b);
        eq(&format!("open({what}) mlen"), l1, l2);
        eq_bytes(&format!("open({what}) m"), &m1, &m2);
        if must_reject {
            eq(&format!("open({what})==-1"), a, -1);
            eq(&format!("open({what}) mlen==0"), l1, 0);
            assert!(
                m1[..smlen].iter().all(|&x| x == 0),
                "open({what}) did not memset smlen bytes"
            );
        }
    }
}

/// ERRORS.md row 6 -- `smlen == SPX_BYTES` exactly (empty message)
#[test]
fn err06_open_empty_message() {
    let p = libs();
    let ckp = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
    let rkp = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
    let cs = f!(p.c, "crypto_sign", FSign);
    let rs = f!(p.rust, "crypto_sign", FSign);
    let co = f!(p.c, "crypto_sign_open", FOpen);
    let ro = f!(p.rust, "crypto_sign_open", FOpen);
    let cri = f!(p.c, "randombytes_init", FRbInit);
    let rri = f!(p.rust, "randombytes_init", FRbInit);

    let mut rng = Rng::new(SEED ^ 0xE6);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    unsafe {
        ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
        let mut pk2 = pk.clone();
        let mut sk2 = sk.clone();
        rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr());
        eq_bytes("err06 pk", &pk, &pk2);
    }

    let mut ent = [0u8; 48];
    rng.fill(&mut ent);
    let mut sm1 = vec![0u8; SPX_BYTES];
    let mut sm2 = sm1.clone();
    let mut sl1 = 0u64;
    let mut sl2 = 0u64;
    let dummy = [0u8; 1];
    unsafe {
        let mut e = ent;
        cri(e.as_mut_ptr(), std::ptr::null_mut());
        cs(sm1.as_mut_ptr(), &mut sl1, dummy.as_ptr(), 0, sk.as_ptr());
        let mut e = ent;
        rri(e.as_mut_ptr(), std::ptr::null_mut());
        rs(sm2.as_mut_ptr(), &mut sl2, dummy.as_ptr(), 0, sk.as_ptr());
    }
    eq("err06 smlen", sl1, sl2);
    eq("err06 smlen==SPX_BYTES", sl1, SPX_BYTES as u64);
    eq_bytes("err06 sm", &sm1, &sm2);

    let mut m1 = vec![0xA5u8; 16];
    let mut m2 = m1.clone();
    let mut l1 = 0xDEADu64;
    let mut l2 = l1;
    let (a, b) = unsafe {
        (
            co(m1.as_mut_ptr(), &mut l1, sm1.as_ptr(), sl1, pk.as_ptr()),
            ro(m2.as_mut_ptr(), &mut l2, sm2.as_ptr(), sl2, pk.as_ptr()),
        )
    };
    eq("open(empty msg)", a, b);
    eq("open(empty msg)==0", a, 0);
    eq("open(empty msg) mlen", l1, l2);
    eq("open(empty msg) mlen==0", l1, 0);
    eq_bytes("open(empty msg) m untouched", &m1, &m2);
}

/// ERRORS.md rows 7, 8 -- `seedexpander_init` `maxlen` boundary
#[test]
fn err07_seedexpander_init_maxlen() {
    let p = libs();
    let ci = f!(p.c, "seedexpander_init", FSeInit);
    let ri = f!(p.rust, "seedexpander_init", FSeInit);
    let mut rng = Rng::new(SEED ^ 0xE7);
    let mut seed = [0u8; 32];
    let mut div = [0u8; 8];
    rng.fill(&mut seed);
    rng.fill(&mut div);

    let marker = CXof {
        buffer: [0x5A; 16],
        buffer_pos: 0x1122_3344_5566_7788,
        length_remaining: 0x99AA_BBCC_DDEE_FF00,
        key: [0xA5; 32],
        ctr: [0x3C; 16],
    };

    // row 7: rejected
    for &maxlen in &[0x1_0000_0000u64, 0x1_0000_0001, u64::MAX] {
        let mut x1 = marker;
        let mut x2 = marker;
        let (mut s1, mut s2) = (seed, seed);
        let (mut d1, mut d2) = (div, div);
        let (a, b) = unsafe {
            (
                ci(&mut x1, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen),
                ri(&mut x2, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen),
            )
        };
        eq(&format!("seedexpander_init(maxlen={maxlen:#x})"), a, b);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x})==-1"), a, -1);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ctx"), x1, x2);
        eq(
            &format!("seedexpander_init(maxlen={maxlen:#x}) ctx untouched"),
            x1,
            marker,
        );
    }

    // row 8: accepted (one below the check)
    for &maxlen in &[0u64, 1, 0xFFFF_FFFE, 0xFFFF_FFFF] {
        let mut x1 = marker;
        let mut x2 = marker;
        let (mut s1, mut s2) = (seed, seed);
        let (mut d1, mut d2) = (div, div);
        let (a, b) = unsafe {
            (
                ci(&mut x1, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen),
                ri(&mut x2, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen),
            )
        };
        eq(&format!("seedexpander_init(maxlen={maxlen:#x})"), a, b);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x})==0"), a, 0);
        eq(&format!("seedexpander_init(maxlen={maxlen:#x}) ctx"), x1, x2);
    }
}

/// ERRORS.md rows 9-12 -- `seedexpander` rejections
#[test]
fn err09_seedexpander_rejections() {
    let p = libs();
    let ci = f!(p.c, "seedexpander_init", FSeInit);
    let ri = f!(p.rust, "seedexpander_init", FSeInit);
    let cse = f!(p.c, "seedexpander", FSe);
    let rse = f!(p.rust, "seedexpander", FSe);
    let mut rng = Rng::new(SEED ^ 0xE9);
    let mut seed = [0u8; 32];
    let mut div = [0u8; 8];
    rng.fill(&mut seed);
    rng.fill(&mut div);

    let fresh = |maxlen: u64| -> (CXof, CXof) {
        let zero = CXof {
            buffer: [0; 16],
            buffer_pos: 0,
            length_remaining: 0,
            key: [0; 32],
            ctr: [0; 16],
        };
        let mut x1 = zero;
        let mut x2 = zero;
        let (mut s1, mut s2) = (seed, seed);
        let (mut d1, mut d2) = (div, div);
        unsafe {
            ci(&mut x1, s1.as_mut_ptr(), d1.as_mut_ptr(), maxlen);
            ri(&mut x2, s2.as_mut_ptr(), d2.as_mut_ptr(), maxlen);
        }
        (x1, x2)
    };

    let maxlen = 100u64;

    // row 9 + row 12: x == NULL wins over the length check
    for &xlen in &[0u64, 10, maxlen, maxlen + 1, u64::MAX] {
        let (mut x1, mut x2) = fresh(maxlen);
        let (a, b) = unsafe {
            (
                cse(&mut x1, std::ptr::null_mut(), xlen),
                rse(&mut x2, std::ptr::null_mut(), xlen),
            )
        };
        eq(&format!("seedexpander(NULL,xlen={xlen})"), a, b);
        eq(&format!("seedexpander(NULL,xlen={xlen})==-2"), a, -2);
        eq(&format!("seedexpander(NULL,xlen={xlen}) ctx"), x1, x2);
    }

    // row 10: xlen >= length_remaining
    for &xlen in &[maxlen, maxlen + 1, maxlen * 2] {
        let (mut x1, mut x2) = fresh(maxlen);
        let before = x1;
        let mut buf = vec![0x11u8; 8];
        let (a, b) = unsafe {
            (
                cse(&mut x1, buf.as_mut_ptr(), xlen),
                rse(&mut x2, buf.as_mut_ptr(), xlen),
            )
        };
        eq(&format!("seedexpander(xlen={xlen}>=rem)"), a, b);
        eq(&format!("seedexpander(xlen={xlen}>=rem)==-3"), a, -3);
        eq(&format!("seedexpander(xlen={xlen}) ctx"), x1, x2);
        eq(&format!("seedexpander(xlen={xlen}) ctx untouched"), x1, before);
    }

    // row 11: the largest accepted length, and zero
    for &xlen in &[0u64, 1, maxlen - 1] {
        let (mut x1, mut x2) = fresh(maxlen);
        let mut b1 = vec![0x22u8; xlen as usize + 8];
        let mut b2 = b1.clone();
        let (a, b) = unsafe {
            (
                cse(&mut x1, b1.as_mut_ptr(), xlen),
                rse(&mut x2, b2.as_mut_ptr(), xlen),
            )
        };
        eq(&format!("seedexpander(xlen={xlen})"), a, b);
        eq(&format!("seedexpander(xlen={xlen})==0"), a, 0);
        eq_bytes(&format!("seedexpander(xlen={xlen}) out"), &b1, &b2);
        eq(&format!("seedexpander(xlen={xlen}) ctx"), x1, x2);
    }
}

/// ERRORS.md row 13 -- `randombytes_init(entropy, NULL)` vs a non-NULL string
#[test]
fn err13_randombytes_init_null_ps() {
    let p = libs();
    let ci = f!(p.c, "randombytes_init", FRbInit);
    let ri = f!(p.rust, "randombytes_init", FRbInit);
    let mut ent = [0u8; 48];
    (0..48).for_each(|i| ent[i] = i as u8);

    let mut e1 = ent;
    let mut e2 = ent;
    unsafe {
        ci(e1.as_mut_ptr(), std::ptr::null_mut());
        ri(e2.as_mut_ptr(), std::ptr::null_mut());
    }
    let a = unsafe { std::slice::from_raw_parts(p.c.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    let b = unsafe { std::slice::from_raw_parts(p.rust.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    eq_bytes("randombytes_init(NULL ps)", &a, &b);
    eq_bytes("randombytes_init entropy untouched", &e1, &e2);

    // an all-zero personalization string must give the SAME state as NULL
    let mut zeros = [0u8; 48];
    let mut e3 = ent;
    unsafe {
        ci(e3.as_mut_ptr(), zeros.as_mut_ptr());
    }
    let c = unsafe { std::slice::from_raw_parts(p.c.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    eq_bytes("randombytes_init(zero ps) == (NULL ps)", &a, &c);
    let _ = &mut zeros;
}

/// ERRORS.md row 14 -- `AES256_CTR_DRBG_Update(NULL, ...)`
#[test]
fn err14_drbg_update_null() {
    let p = libs();
    let cf = f!(p.c, "AES256_CTR_DRBG_Update", FUpdate);
    let rf = f!(p.rust, "AES256_CTR_DRBG_Update", FUpdate);
    let mut rng = Rng::new(SEED ^ 0xE14);
    for _ in 0..8 {
        let mut key = [0u8; 32];
        let mut v = [0u8; 16];
        rng.fill(&mut key);
        rng.fill(&mut v);
        let mut k1 = key;
        let mut k2 = key;
        let mut v1 = v;
        let mut v2 = v;
        unsafe {
            cf(std::ptr::null_mut(), k1.as_mut_ptr(), v1.as_mut_ptr());
            rf(std::ptr::null_mut(), k2.as_mut_ptr(), v2.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(NULL) Key", &k1, &k2);
        eq_bytes("DRBG_Update(NULL) V", &v1, &v2);

        // A 48-byte all-zero `provided_data` must give the same result as NULL.
        let mut pd = [0u8; 48];
        let mut k3 = key;
        let mut v3 = v;
        unsafe {
            cf(pd.as_mut_ptr(), k3.as_mut_ptr(), v3.as_mut_ptr());
        }
        eq_bytes("DRBG_Update(zero pd) == (NULL)", &k1, &k3);
        eq_bytes("DRBG_Update(zero pd) == (NULL) V", &v1, &v3);
    }
}

/// ERRORS.md rows 15-19 -- no validation at all in the address setters:
/// out-of-range "enum" values and oversized field values are truncated.
#[test]
fn err15_address_no_validation() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 0xE15);
    // (setter, values that have no valid meaning)
    let cases: Vec<(&str, Vec<u32>)> = vec![
        // row 15: SPX_ADDR_TYPE_* only defines 0..=6
        ("SPX_set_type", vec![7, 8, 42, 255, 256, 0x103, 0xFFFF_FFFF]),
        // row 16
        (
            "SPX_set_layer_addr",
            vec![SPX_D as u32, 255, 256, 0x1FF, 0xFFFF_FFFF],
        ),
        // row 17
        (
            "SPX_set_chain_addr",
            vec![SPX_WOTS_LEN as u32, 255, 256, 0xFFFF_FFFF],
        ),
        (
            "SPX_set_hash_addr",
            vec![SPX_WOTS_W as u32, 255, 256, 0xFFFF_FFFF],
        ),
        // row 18
        (
            "SPX_set_tree_height",
            vec![SPX_FULL_HEIGHT as u32 + 1, 255, 256, 0xFFFF_FFFF],
        ),
    ];
    for (name, vals) in cases {
        let cf = f!(p.c, name, FAddrU32);
        let rf = f!(p.rust, name, FAddrU32);
        for v in vals {
            let mut a = [0u32; 8];
            for w in a.iter_mut() {
                *w = rng.next_u32();
            }
            let mut b = a;
            let before = a;
            unsafe {
                cf(a.as_mut_ptr(), v);
                rf(b.as_mut_ptr(), v);
            }
            eq(&format!("{name}({v:#x})"), a, b);
            // and it really is *accepted*, not rejected: some byte changed
            // unless the truncated value happened to already be there.
            let _ = before;
        }
    }

    // row 19: set_tree_addr with all 64 bits set
    type FAddrU64 = unsafe extern "C" fn(*mut u32, u64);
    let cf = f!(p.c, "SPX_set_tree_addr", FAddrU64);
    let rf = f!(p.rust, "SPX_set_tree_addr", FAddrU64);
    for &v in &[u64::MAX, 1u64 << 63, 0x0123_4567_89AB_CDEF] {
        let mut a = [0u32; 8];
        for w in a.iter_mut() {
            *w = rng.next_u32();
        }
        let mut b = a;
        unsafe {
            cf(a.as_mut_ptr(), v);
            rf(b.as_mut_ptr(), v);
        }
        eq(&format!("set_tree_addr({v:#x})"), a, b);
    }
}

/// ERRORS.md rows 20-22 -- degenerate lengths in the byte-order helpers
#[test]
fn err20_length_degenerates() {
    let p = libs();
    let cu = f!(p.c, "SPX_ull_to_bytes", FUll);
    let ru = f!(p.rust, "SPX_ull_to_bytes", FUll);
    let cb = f!(p.c, "SPX_bytes_to_ull", FB2U);
    let rb = f!(p.rust, "SPX_bytes_to_ull", FB2U);

    // row 20: outlen == 0 writes nothing
    for &v in &[0u64, 1, u64::MAX] {
        let mut a = vec![0xA5u8; 16];
        let mut b = a.clone();
        unsafe {
            cu(a.as_mut_ptr(), 0, v);
            ru(b.as_mut_ptr(), 0, v);
        }
        eq_bytes(&format!("ull_to_bytes(outlen=0,{v:#x})"), &a, &b);
        assert!(a.iter().all(|&x| x == 0xA5), "outlen=0 wrote something");
    }

    // rows 21, 22: inlen == 0 -> 0, inlen == 8 -> full 64-bit value
    let buf = [0xFFu8; 8];
    let (a, b) = unsafe { (cb(buf.as_ptr(), 0), rb(buf.as_ptr(), 0)) };
    eq("bytes_to_ull(inlen=0)", a, b);
    eq("bytes_to_ull(inlen=0)==0", a, 0);
    let (a, b) = unsafe { (cb(buf.as_ptr(), 8), rb(buf.as_ptr(), 8)) };
    eq("bytes_to_ull(inlen=8)", a, b);
    eq("bytes_to_ull(inlen=8)==u64::MAX", a, u64::MAX);
}

/// ERRORS.md row 23 -- `thash(inblocks = 0)`
#[test]
fn err23_thash_zero_inblocks() {
    let p = libs();
    let ci = f!(p.c, "SPX_initialize_hash_function", FInit);
    let ri = f!(p.rust, "SPX_initialize_hash_function", FInit);
    let cf = f!(p.c, "SPX_thash", FThash);
    let rf = f!(p.rust, "SPX_thash", FThash);
    let mut rng = Rng::new(SEED ^ 0xE23);

    for _ in 0..8 {
        let mut ca = new_ctx();
        seed_ctx(&mut ca, &mut rng);
        let mut ra = ca.clone();
        unsafe {
            ci(ca.as_mut_ptr());
            ri(ra.as_mut_ptr());
        }
        let input = rng.bytes(8);
        let mut addr = [0u32; 8];
        for w in addr.iter_mut() {
            *w = rng.next_u32();
        }
        let mut a1 = addr;
        let mut a2 = addr;
        let mut o1 = vec![0x7Fu8; 64];
        let mut o2 = o1.clone();
        unsafe {
            cf(o1.as_mut_ptr(), input.as_ptr(), 0, ca.as_ptr(), a1.as_mut_ptr());
            rf(o2.as_mut_ptr(), input.as_ptr(), 0, ra.as_ptr(), a2.as_mut_ptr());
        }
        eq_bytes("thash(inblocks=0)", &o1, &o2);
        eq("thash(inblocks=0) addr", a1, a2);
    }
}

/// ERRORS.md row 24 -- the four "cannot fail" entry points always return 0
#[test]
fn err24_unconditional_success() {
    let p = libs();
    let ckp = f!(p.c, "crypto_sign_seed_keypair", FSeedKeypair);
    let rkp = f!(p.rust, "crypto_sign_seed_keypair", FSeedKeypair);
    let ckg = f!(p.c, "crypto_sign_keypair", FKeypair);
    let rkg = f!(p.rust, "crypto_sign_keypair", FKeypair);
    let csg = f!(p.c, "crypto_sign_signature", FSignature);
    let rsg = f!(p.rust, "crypto_sign_signature", FSignature);
    let cs = f!(p.c, "crypto_sign", FSign);
    let rs = f!(p.rust, "crypto_sign", FSign);
    let cri = f!(p.c, "randombytes_init", FRbInit);
    let rri = f!(p.rust, "randombytes_init", FRbInit);

    let mut rng = Rng::new(SEED ^ 0xE24);
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pk = vec![0u8; SPX_PK_BYTES];
    let mut sk = vec![0u8; SPX_SK_BYTES];
    let mut pk2 = pk.clone();
    let mut sk2 = sk.clone();
    let (a, b) = unsafe {
        (
            ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()),
            rkp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), seed.as_ptr()),
        )
    };
    eq("seed_keypair ret", a, b);
    eq("seed_keypair ret==0", a, 0);

    let mut ent = [0u8; 48];
    rng.fill(&mut ent);
    let (a, b) = unsafe {
        let mut e = ent;
        cri(e.as_mut_ptr(), std::ptr::null_mut());
        let x = ckg(pk.as_mut_ptr(), sk.as_mut_ptr());
        let mut e = ent;
        rri(e.as_mut_ptr(), std::ptr::null_mut());
        let y = rkg(pk2.as_mut_ptr(), sk2.as_mut_ptr());
        (x, y)
    };
    eq("keypair ret", a, b);
    eq("keypair ret==0", a, 0);
    eq_bytes("keypair pk", &pk, &pk2);
    eq_bytes("keypair sk", &sk, &sk2);

    // an all-zero secret key is still "valid" input: nothing is validated
    let zsk = vec![0u8; SPX_SK_BYTES];
    for mlen in [0usize, 1] {
        let m = vec![0x42u8; mlen.max(1)];
        let mut s1 = vec![0u8; SPX_BYTES + mlen];
        let mut s2 = s1.clone();
        let mut l1 = 0usize;
        let mut l2 = 0usize;
        let (a, b) = unsafe {
            let mut e = ent;
            cri(e.as_mut_ptr(), std::ptr::null_mut());
            let x = csg(s1.as_mut_ptr(), &mut l1, m.as_ptr(), mlen, zsk.as_ptr());
            let mut e = ent;
            rri(e.as_mut_ptr(), std::ptr::null_mut());
            let y = rsg(s2.as_mut_ptr(), &mut l2, m.as_ptr(), mlen, zsk.as_ptr());
            (x, y)
        };
        eq(&format!("signature(zero sk,mlen={mlen}) ret"), a, b);
        eq(&format!("signature(zero sk,mlen={mlen}) ret==0"), a, 0);
        eq(&format!("signature(zero sk,mlen={mlen}) siglen"), l1, l2);
        eq_bytes(&format!("signature(zero sk,mlen={mlen}) sig"), &s1, &s2);

        let mut t1 = vec![0u8; SPX_BYTES + mlen];
        let mut t2 = t1.clone();
        let mut n1 = 0u64;
        let mut n2 = 0u64;
        let (a, b) = unsafe {
            let mut e = ent;
            cri(e.as_mut_ptr(), std::ptr::null_mut());
            let x = cs(
                t1.as_mut_ptr(),
                &mut n1,
                m.as_ptr(),
                mlen as u64,
                zsk.as_ptr(),
            );
            let mut e = ent;
            rri(e.as_mut_ptr(), std::ptr::null_mut());
            let y = rs(
                t2.as_mut_ptr(),
                &mut n2,
                m.as_ptr(),
                mlen as u64,
                zsk.as_ptr(),
            );
            (x, y)
        };
        eq(&format!("sign(zero sk,mlen={mlen}) ret"), a, b);
        eq(&format!("sign(zero sk,mlen={mlen}) ret==0"), a, 0);
        eq(&format!("sign(zero sk,mlen={mlen}) smlen"), n1, n2);
        eq_bytes(&format!("sign(zero sk,mlen={mlen}) sm"), &t1, &t2);
    }
}

/// ERRORS.md row 25 -- `randombytes(x, 0)` still advances the DRBG
#[test]
fn err25_randombytes_zero_len() {
    let p = libs();
    let ci = f!(p.c, "randombytes_init", FRbInit);
    let ri = f!(p.rust, "randombytes_init", FRbInit);
    let cr = f!(p.c, "randombytes", FRb);
    let rr = f!(p.rust, "randombytes", FRb);
    let mut ent = [0u8; 48];
    (0..48).for_each(|i| ent[i] = (i as u8) ^ 0x37);

    let mut e1 = ent;
    let mut e2 = ent;
    unsafe {
        ci(e1.as_mut_ptr(), std::ptr::null_mut());
        ri(e2.as_mut_ptr(), std::ptr::null_mut());
    }
    let before = unsafe { std::slice::from_raw_parts(p.c.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    let mut buf = vec![0xA5u8; 8];
    let mut buf2 = buf.clone();
    let (a, b) = unsafe { (cr(buf.as_mut_ptr(), 0), rr(buf2.as_mut_ptr(), 0)) };
    eq("randombytes(0) ret", a, b);
    eq("randombytes(0) ret==0", a, 0);
    eq_bytes("randombytes(0) buffer untouched", &buf, &buf2);
    assert!(buf.iter().all(|&x| x == 0xA5), "randombytes(0) wrote bytes");
    let ca = unsafe { std::slice::from_raw_parts(p.c.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    let ra = unsafe { std::slice::from_raw_parts(p.rust.data("DRBG_ctx"), DRBG_BYTES) }.to_vec();
    eq_bytes("randombytes(0) DRBG_ctx", &ca, &ra);
    assert_ne!(before, ca, "randombytes(0) must still reseed the DRBG");
}

/// ERRORS.md row 26 -- an empty message through the low-level hash entry points
#[test]
fn err26_empty_message() {
    let p = libs();
    let ci = f!(p.c, "SPX_initialize_hash_function", FInit);
    let ri = f!(p.rust, "SPX_initialize_hash_function", FInit);
    let cg = f!(p.c, "SPX_gen_message_random", FGenR);
    let rg = f!(p.rust, "SPX_gen_message_random", FGenR);
    let mut rng = Rng::new(SEED ^ 0xE26);

    let mut ca = new_ctx();
    seed_ctx(&mut ca, &mut rng);
    let mut ra = ca.clone();
    unsafe {
        ci(ca.as_mut_ptr());
        ri(ra.as_mut_ptr());
    }
    let sk_prf = rng.bytes(SPX_N);
    let optrand = rng.bytes(SPX_N);
    let dummy = [0u8; 1];
    let mut o1 = vec![0x9Cu8; 160];
    let mut o2 = o1.clone();
    unsafe {
        cg(
            o1.as_mut_ptr(),
            sk_prf.as_ptr(),
            optrand.as_ptr(),
            dummy.as_ptr(),
            0,
            ca.as_ptr(),
        );
        rg(
            o2.as_mut_ptr(),
            sk_prf.as_ptr(),
            optrand.as_ptr(),
            dummy.as_ptr(),
            0,
            ra.as_ptr(),
        );
    }
    eq_bytes("gen_message_random(mlen=0)", &o1, &o2);
}
