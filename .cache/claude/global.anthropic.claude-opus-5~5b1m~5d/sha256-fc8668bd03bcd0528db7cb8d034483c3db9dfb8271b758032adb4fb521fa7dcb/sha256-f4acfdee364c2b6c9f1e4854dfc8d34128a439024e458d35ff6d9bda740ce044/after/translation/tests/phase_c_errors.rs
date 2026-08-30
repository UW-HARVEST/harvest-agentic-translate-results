//! Phase C: one differential test per row of ERRORS.md.
mod common;
use common::*;

type FSeedKp = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type FKp = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type FSignature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type FVerify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type FSign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type FRbInit = unsafe extern "C" fn(*mut u8, *mut u8);
type FRb = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FUpd = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8);
type FSeInit = unsafe extern "C" fn(*mut u8, *mut u8, *mut u8, u64) -> i32;
type FSe = unsafe extern "C" fn(*mut u8, *mut u8, u64) -> i32;
type FAddrU32 = unsafe extern "C" fn(*mut u32, u32);
type FAddrU64 = unsafe extern "C" fn(*mut u32, u64);
type FUll = unsafe extern "C" fn(*mut u8, u32, u64);
type FB2U = unsafe extern "C" fn(*const u8, u32) -> u64;
type FThash = unsafe extern "C" fn(*mut u8, *const u8, std::ffi::c_uint, *const u8, *mut u32);
type FSizes = unsafe extern "C" fn() -> u64;

const XOF_BYTES: usize = 80;

const RNG_SUCCESS: i32 = 0;
const RNG_BAD_MAXLEN: i32 = -1;
const RNG_BAD_OUTBUF: i32 = -2;
const RNG_BAD_REQ_LEN: i32 = -3;

/// A valid (pk, sk, m, sig) tuple produced by the C library.
struct Kit {
    pk: Vec<u8>,
    sk: Vec<u8>,
    m: Vec<u8>,
    sig: Vec<u8>,
}

/// The full keygen+sign is expensive for the `*s` parameter sets, so the kits
/// are built once per (mlen) and reused by every error-path test in this binary.
static KITS: std::sync::Mutex<Option<std::collections::HashMap<usize, std::sync::Arc<Kit>>>> =
    std::sync::Mutex::new(None);

unsafe fn kit(p: &Pair, rng: &mut Rng, mlen: usize) -> std::sync::Arc<Kit> {
    let mut g = KITS.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(std::collections::HashMap::new);
    if let Some(k) = map.get(&mlen) {
        return k.clone();
    }
    let k = std::sync::Arc::new(make_kit(p, rng, mlen));
    map.insert(mlen, k.clone());
    k
}

unsafe fn make_kit(p: &Pair, rng: &mut Rng, mlen: usize) -> Kit {
    let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
    let csg = sym!(p.c, b"crypto_sign_signature\0", FSignature);
    let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
    let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
    let mut e1 = rng.bytes(48);
    let mut e2 = e1.clone();
    ci(e1.as_mut_ptr(), core::ptr::null_mut());
    ri(e2.as_mut_ptr(), core::ptr::null_mut());

    let seed = rng.bytes(SEED_BYTES);
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    ckp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr());
    let m = rng.bytes(mlen.max(1));
    let mut sig = vec![0u8; SPX_BYTES];
    let mut sl = 0usize;
    csg(sig.as_mut_ptr(), &mut sl, m.as_ptr(), mlen, sk.as_ptr());
    assert_eq!(sl, SPX_BYTES);
    Kit { pk, sk, m: m[..mlen].to_vec(), sig }
}

// ============ rows 1-3: crypto_sign_verify ============
#[test]
fn e01_verify_siglen_mismatch() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 101);
    unsafe {
        let k = kit(p, &mut rng, 32);
        let cvf = sym!(p.c, b"crypto_sign_verify\0", FVerify);
        let rvf = sym!(p.r, b"crypto_sign_verify\0", FVerify);
        // a buffer large enough for every siglen we pass
        let mut big = vec![0u8; 2 * SPX_BYTES + 64];
        big[..SPX_BYTES].copy_from_slice(&k.sig);
        for siglen in [0usize, 1, SPX_BYTES - 1, SPX_BYTES + 1, 2 * SPX_BYTES] {
            let a = cvf(big.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            let b = rvf(big.as_ptr(), siglen, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            eqv(&format!("verify siglen={siglen}"), a, b);
            eqv(&format!("verify siglen={siglen} is -1"), a, -1);
        }
        // ... and with NULL message/pk, which the C never touches on this path
        let a = cvf(big.as_ptr(), 0, core::ptr::null(), 0, core::ptr::null());
        let b = rvf(big.as_ptr(), 0, core::ptr::null(), 0, core::ptr::null());
        eqv("verify siglen=0 with NULLs", a, b);
        eqv("verify siglen=0 with NULLs is -1", a, -1);
    }
}

#[test]
fn e02_verify_root_mismatch() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 102);
    unsafe {
        let k = kit(p, &mut rng, 32);
        let cvf = sym!(p.c, b"crypto_sign_verify\0", FVerify);
        let rvf = sym!(p.r, b"crypto_sign_verify\0", FVerify);

        // (a) flip a bit in each region of the signature
        for off in [0usize, N, N + 1, N + FORS_BYTES / 2, N + FORS_BYTES, SPX_BYTES - 1] {
            let mut sig = k.sig.clone();
            sig[off] ^= 0x01;
            let a = cvf(sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            let b = rvf(sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            eqv(&format!("verify corrupt sig@{off}"), a, b);
            eqv(&format!("verify corrupt sig@{off} is -1"), a, -1);
        }
        // (b) fully random signature
        for _ in 0..4 {
            let sig = rng.bytes(SPX_BYTES);
            let a = cvf(sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            let b = rvf(sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
            eqv("verify random sig", a, b);
            eqv("verify random sig is -1", a, -1);
        }
        // (c) wrong message
        let mut m2 = k.m.clone();
        m2[0] ^= 0xff;
        let a = cvf(k.sig.as_ptr(), SPX_BYTES, m2.as_ptr(), m2.len(), k.pk.as_ptr());
        let b = rvf(k.sig.as_ptr(), SPX_BYTES, m2.as_ptr(), m2.len(), k.pk.as_ptr());
        eqv("verify wrong message", a, b);
        eqv("verify wrong message is -1", a, -1);
        // (d) truncated message length
        let a = cvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len() - 1, k.pk.as_ptr());
        let b = rvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len() - 1, k.pk.as_ptr());
        eqv("verify wrong mlen", a, b);
        eqv("verify wrong mlen is -1", a, -1);
        // (e) wrong pk (both halves)
        for off in [0usize, N] {
            let mut pk2 = k.pk.clone();
            pk2[off] ^= 0x80;
            let a = cvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), pk2.as_ptr());
            let b = rvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), pk2.as_ptr());
            eqv(&format!("verify wrong pk@{off}"), a, b);
            eqv(&format!("verify wrong pk@{off} is -1"), a, -1);
        }
    }
}

#[test]
fn e03_verify_ok() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 103);
    unsafe {
        let k = kit(p, &mut rng, 32);
        let cvf = sym!(p.c, b"crypto_sign_verify\0", FVerify);
        let rvf = sym!(p.r, b"crypto_sign_verify\0", FVerify);
        let a = cvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
        let b = rvf(k.sig.as_ptr(), SPX_BYTES, k.m.as_ptr(), k.m.len(), k.pk.as_ptr());
        eqv("verify valid", a, b);
        eqv("verify valid is 0", a, 0);
    }
}

// ============ rows 4-8: crypto_sign_open ============
unsafe fn open_case(p: &Pair, sm: &[u8], smlen: u64, pk: *const u8, label: &str) -> i32 {
    let co = sym!(p.c, b"crypto_sign_open\0", FOpen);
    let ro = sym!(p.r, b"crypto_sign_open\0", FOpen);
    let n = smlen as usize + 64;
    let mut cm = vec![0xA5u8; n];
    let mut rm = vec![0xA5u8; n];
    let mut cml = 0xDEAD_BEEF_u64;
    let mut rml = 0xDEAD_BEEF_u64;
    let a = co(cm.as_mut_ptr(), &mut cml, sm.as_ptr(), smlen, pk);
    let b = ro(rm.as_mut_ptr(), &mut rml, sm.as_ptr(), smlen, pk);
    eqv(&format!("{label} ret"), a, b);
    eqv(&format!("{label} mlen"), cml, rml);
    eqb(&format!("{label} m buffer"), &cm, &rm);
    a
}

#[test]
fn e04_open_smlen_too_small() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 104);
    unsafe {
        let k = kit(p, &mut rng, 32);
        let mut sm = k.sig.clone();
        sm.extend_from_slice(&k.m);
        for smlen in [1u64, 2, 16, (SPX_BYTES / 2) as u64] {
            let r = open_case(p, &sm, smlen, k.pk.as_ptr(), &format!("open smlen={smlen}"));
            eqv("open too small is -1", r, -1);
        }
    }
}

#[test]
fn e05_open_smlen_zero() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 105);
    unsafe {
        let k = kit(p, &mut rng, 8);
        let mut sm = k.sig.clone();
        sm.extend_from_slice(&k.m);
        let r = open_case(p, &sm, 0, k.pk.as_ptr(), "open smlen=0");
        eqv("open smlen=0 is -1", r, -1);
        // also with a NULL public key: the C returns before touching it
        let r = open_case(p, &sm, 0, core::ptr::null(), "open smlen=0 pk=NULL");
        eqv("open smlen=0 pk=NULL is -1", r, -1);
    }
}

#[test]
fn e06_open_smlen_off_by_one() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 106);
    unsafe {
        let k = kit(p, &mut rng, 32);
        let mut sm = k.sig.clone();
        sm.extend_from_slice(&k.m);
        let r = open_case(p, &sm, (SPX_BYTES - 1) as u64, k.pk.as_ptr(), "open smlen=SPX_BYTES-1");
        eqv("open smlen=SPX_BYTES-1 is -1", r, -1);
    }
}

#[test]
fn e07_open_verify_fails() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 107);
    unsafe {
        let k = kit(p, &mut rng, 40);
        let mut sm = k.sig.clone();
        sm.extend_from_slice(&k.m);
        // corrupt the signature part
        for off in [0usize, N, SPX_BYTES - 1] {
            let mut bad = sm.clone();
            bad[off] ^= 0x01;
            let r = open_case(p, &bad, bad.len() as u64, k.pk.as_ptr(), &format!("open bad sig@{off}"));
            eqv("open bad sig is -1", r, -1);
        }
        // corrupt the FIRST message byte: always covered by the digest.
        let mut bad = sm.clone();
        bad[SPX_BYTES] ^= 0x01;
        let r = open_case(p, &bad, bad.len() as u64, k.pk.as_ptr(), "open bad msg[0]");
        eqv("open bad msg[0] is -1", r, -1);
        // corrupt the LAST message byte. NOTE: `hash_blake.c` passes the byte
        // count to `blakeX_update`, which treats it as a BIT count, so only the
        // first `mlen/8` message bytes reach the digest for the BLAKE backend.
        // We therefore only require the two libraries to agree here (they must
        // reproduce that quirk identically), not that the result is -1.
        let mut bad = sm.clone();
        let l = bad.len();
        bad[l - 1] ^= 0x01;
        open_case(p, &bad, l as u64, k.pk.as_ptr(), "open bad msg[last]");
        // wrong pk
        let mut pk2 = k.pk.clone();
        pk2[0] ^= 0x01;
        let r = open_case(p, &sm, sm.len() as u64, pk2.as_ptr(), "open wrong pk");
        eqv("open wrong pk is -1", r, -1);
    }
}

#[test]
fn e08_open_empty_message() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 108);
    unsafe {
        let k = kit(p, &mut rng, 0);
        // sign an empty message via crypto_sign on both sides for the exact sm
        let cs = sym!(p.c, b"crypto_sign\0", FSign);
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
        let mut sm = vec![0u8; SPX_BYTES];
        let mut sl = 0u64;
        let m: Vec<u8> = vec![];
        cs(sm.as_mut_ptr(), &mut sl, m.as_ptr(), 0, k.sk.as_ptr());
        eqv("crypto_sign empty smlen", sl as usize, SPX_BYTES);
        let r = open_case(p, &sm, sl, k.pk.as_ptr(), "open empty");
        eqv("open empty is 0", r, 0);
    }
}

// ============ rows 9-16: seedexpander ============
#[test]
fn e09_seedexpander_init_maxlen() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 109);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        for maxlen in [0x1_0000_0000u64, 0x1_0000_0001, 0xFFFF_FFFF_FFFF_FFFF, 0x2_0000_0000] {
            let mut cctx = vec![0x9Bu8; XOF_BYTES];
            let mut rctx = vec![0x9Bu8; XOF_BYTES];
            let a = ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            let b = ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            eqv(&format!("seedexpander_init maxlen={maxlen:#x}"), a, b);
            eqv("is RNG_BAD_MAXLEN", a, RNG_BAD_MAXLEN);
            eqb("ctx untouched on RNG_BAD_MAXLEN", &cctx, &rctx);
            eqv("ctx really untouched", &cctx, &vec![0x9Bu8; XOF_BYTES]);
        }
    }
}

#[test]
fn e10_seedexpander_init_maxlen_boundary() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 110);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        for maxlen in [0u64, 1, 0xFFFF_FFFE, 0xFFFF_FFFF] {
            let mut cctx = vec![0u8; XOF_BYTES];
            let mut rctx = vec![0u8; XOF_BYTES];
            let a = ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            let b = ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            eqv(&format!("seedexpander_init maxlen={maxlen:#x}"), a, b);
            eqv("is RNG_SUCCESS", a, RNG_SUCCESS);
            eqb("ctx image", &cctx, &rctx);
        }
    }
}

#[test]
fn e11_seedexpander_null_out() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 111);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut cctx = vec![0u8; XOF_BYTES];
        let mut rctx = vec![0u8; XOF_BYTES];
        ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
        ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
        // NULL out with an otherwise-fine length, and with a bad length too:
        // the NULL check comes first, so the result must be RNG_BAD_OUTBUF.
        for xlen in [0u64, 1, 16, 1024, 4096] {
            let before = cctx.clone();
            let a = cse(cctx.as_mut_ptr(), core::ptr::null_mut(), xlen);
            let b = rse(rctx.as_mut_ptr(), core::ptr::null_mut(), xlen);
            eqv(&format!("seedexpander NULL out xlen={xlen}"), a, b);
            eqv("is RNG_BAD_OUTBUF", a, RNG_BAD_OUTBUF);
            eqb("ctx untouched", &cctx, &rctx);
            eqb("ctx really untouched", &cctx, &before);
        }
    }
}

#[test]
fn e12_seedexpander_req_len() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 112);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        for maxlen in [1u64, 16, 100] {
            let mut cctx = vec![0u8; XOF_BYTES];
            let mut rctx = vec![0u8; XOF_BYTES];
            ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            for xlen in [maxlen, maxlen + 1, maxlen * 2, 0xFFFF_FFFF] {
                let before = cctx.clone();
                let mut cb = obuf(0);
                let mut rb = obuf(0);
                let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen);
                let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen);
                eqv(&format!("seedexpander maxlen={maxlen} xlen={xlen}"), a, b);
                eqv("is RNG_BAD_REQ_LEN", a, RNG_BAD_REQ_LEN);
                eqb("no output written", &cb, &rb);
                eqb("ctx untouched", &cctx, &before);
                eqb("ctx C==R", &cctx, &rctx);
            }
        }
    }
}

#[test]
fn e13_seedexpander_req_len_boundary() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 113);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        for maxlen in [2u64, 17, 33, 101] {
            let mut cctx = vec![0u8; XOF_BYTES];
            let mut rctx = vec![0u8; XOF_BYTES];
            ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), maxlen);
            let xlen = maxlen - 1;
            let mut cb = obuf(xlen as usize);
            let mut rb = obuf(xlen as usize);
            let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen);
            let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen);
            eqv(&format!("seedexpander maxlen={maxlen} xlen={xlen}"), a, b);
            eqv("is RNG_SUCCESS", a, RNG_SUCCESS);
            eqb("out", &cb, &rb);
            eqb("ctx", &cctx, &rctx);
        }
    }
}

#[test]
fn e14_seedexpander_zero_len_zero_remaining() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 114);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut cctx = vec![0u8; XOF_BYTES];
        let mut rctx = vec![0u8; XOF_BYTES];
        ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 0);
        ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 0);
        eqb("ctx after init maxlen=0", &cctx, &rctx);
        let mut cb = obuf(0);
        let mut rb = obuf(0);
        let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), 0);
        let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), 0);
        eqv("seedexpander xlen=0 remaining=0", a, b);
        eqv("is RNG_BAD_REQ_LEN", a, RNG_BAD_REQ_LEN);
        eqb("ctx", &cctx, &rctx);
    }
}

#[test]
fn e15_seedexpander_zero_len() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 115);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut cctx = vec![0u8; XOF_BYTES];
        let mut rctx = vec![0u8; XOF_BYTES];
        ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
        ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 1024);
        let before = cctx.clone();
        let mut cb = obuf(0);
        let mut rb = obuf(0);
        let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), 0);
        let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), 0);
        eqv("seedexpander xlen=0", a, b);
        eqv("is RNG_SUCCESS", a, RNG_SUCCESS);
        eqb("no output", &cb, &rb);
        eqb("ctx C==R", &cctx, &rctx);
        eqb("ctx unchanged (length_remaining -= 0)", &cctx, &before);
    }
}

#[test]
fn e16_seedexpander_buffered_path() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 116);
    unsafe {
        let ci = sym!(p.c, b"seedexpander_init\0", FSeInit);
        let ri = sym!(p.r, b"seedexpander_init\0", FSeInit);
        let cse = sym!(p.c, b"seedexpander\0", FSe);
        let rse = sym!(p.r, b"seedexpander\0", FSe);
        let mut seed = rng.bytes(32);
        let mut div = rng.bytes(8);
        let mut cctx = vec![0u8; XOF_BYTES];
        let mut rctx = vec![0u8; XOF_BYTES];
        ci(cctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 100_000);
        ri(rctx.as_mut_ptr(), seed.as_mut_ptr(), div.as_mut_ptr(), 100_000);
        // First call refills the buffer; subsequent 1-byte calls take the
        // early `return RNG_SUCCESS` inside the loop (rng.c:79).
        for xlen in [17u64, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1] {
            let mut cb = obuf(xlen as usize);
            let mut rb = obuf(xlen as usize);
            let a = cse(cctx.as_mut_ptr(), cb.as_mut_ptr(), xlen);
            let b = rse(rctx.as_mut_ptr(), rb.as_mut_ptr(), xlen);
            eqv("seedexpander buffered ret", a, b);
            eqv("is RNG_SUCCESS", a, RNG_SUCCESS);
            eqb("buffered out", &cb, &rb);
            eqb("buffered ctx", &cctx, &rctx);
        }
    }
}

// ============ rows 17-21: DRBG ============
#[test]
fn e17_randombytes_always_success() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 117);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let cr = sym!(p.c, b"randombytes\0", FRb);
        let rr = sym!(p.r, b"randombytes\0", FRb);
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
        for xlen in [0usize, 1, 15, 16, 17, 4096] {
            let mut cb = obuf(xlen);
            let mut rb = obuf(xlen);
            let a = cr(cb.as_mut_ptr(), xlen as u64);
            let b = rr(rb.as_mut_ptr(), xlen as u64);
            eqv("randombytes ret", a, b);
            eqv("randombytes is RNG_SUCCESS", a, RNG_SUCCESS);
            eqb("randombytes out", &cb, &rb);
            eqb("DRBG_ctx", &drbg_image(&p.c), &drbg_image(&p.r));
        }
    }
}

#[test]
fn e18_randombytes_zero_len() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 118);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let cr = sym!(p.c, b"randombytes\0", FRb);
        let rr = sym!(p.r, b"randombytes\0", FRb);
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
        let c_before = drbg_image(&p.c);
        let mut cb = obuf(0);
        let mut rb = obuf(0);
        let a = cr(cb.as_mut_ptr(), 0);
        let b = rr(rb.as_mut_ptr(), 0);
        eqv("randombytes(0) ret", a, b);
        eqv("randombytes(0) is RNG_SUCCESS", a, RNG_SUCCESS);
        eqb("randombytes(0) writes nothing", &cb, &rb);
        let c_after = drbg_image(&p.c);
        let r_after = drbg_image(&p.r);
        eqb("DRBG_ctx after randombytes(0)", &c_after, &r_after);
        assert_ne!(c_before, c_after, "[{}] C DRBG must advance even for xlen=0", cfg_name());
        // reseed_counter is the last 4 bytes
        let cc = i32::from_ne_bytes(c_after[48..52].try_into().unwrap());
        let rc = i32::from_ne_bytes(r_after[48..52].try_into().unwrap());
        eqv("reseed_counter", cc, rc);
        eqv("reseed_counter bumped", cc, 2);
    }
}

#[test]
fn e19_randombytes_init_null_pers() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 119);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        for _ in 0..16 {
            let mut e1 = rng.bytes(48);
            let mut e2 = e1.clone();
            ci(e1.as_mut_ptr(), core::ptr::null_mut());
            ri(e2.as_mut_ptr(), core::ptr::null_mut());
            eqb("DRBG_ctx (pers=NULL)", &drbg_image(&p.c), &drbg_image(&p.r));
            eqb("entropy untouched", &e1, &e2);
        }
    }
}

#[test]
fn e20_randombytes_init_with_pers() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 120);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        for pat in [None, Some(0x00u8), Some(0xffu8)] {
            let mut e1 = rng.bytes(48);
            let mut e2 = e1.clone();
            let mut p1 = match pat {
                None => rng.bytes(48),
                Some(x) => vec![x; 48],
            };
            let mut p2 = p1.clone();
            ci(e1.as_mut_ptr(), p1.as_mut_ptr());
            ri(e2.as_mut_ptr(), p2.as_mut_ptr());
            eqb("DRBG_ctx (pers set)", &drbg_image(&p.c), &drbg_image(&p.r));
            eqb("pers untouched", &p1, &p2);
        }
        // an all-zero personalization string must equal the NULL case
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        let mut z = vec![0u8; 48];
        ci(e1.as_mut_ptr(), z.as_mut_ptr());
        let with_zero = drbg_image(&p.c);
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
        let with_null = drbg_image(&p.r);
        eqb("zero pers == NULL pers", &with_zero, &with_null);
    }
}

#[test]
fn e21_drbg_update_null_provided() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 121);
    unsafe {
        let cf = sym!(p.c, b"AES256_CTR_DRBG_Update\0", FUpd);
        let rf = sym!(p.r, b"AES256_CTR_DRBG_Update\0", FUpd);
        for _ in 0..32 {
            let k = rng.bytes(32);
            let v = rng.bytes(16);
            let mut ck = k.clone();
            let mut rk = k.clone();
            let mut cv = v.clone();
            let mut rv = v.clone();
            cf(core::ptr::null_mut(), ck.as_mut_ptr(), cv.as_mut_ptr());
            rf(core::ptr::null_mut(), rk.as_mut_ptr(), rv.as_mut_ptr());
            eqb("Update(NULL) Key", &ck, &rk);
            eqb("Update(NULL) V", &cv, &rv);

            // and the all-zero provided_data must equal the NULL case
            let mut zk = k.clone();
            let mut zv = v.clone();
            let mut z = vec![0u8; 48];
            cf(z.as_mut_ptr(), zk.as_mut_ptr(), zv.as_mut_ptr());
            eqb("Update(zeros) == Update(NULL) Key", &zk, &ck);
            eqb("Update(zeros) == Update(NULL) V", &zv, &cv);
        }
    }
}

// ============ rows 22-28: out-of-range address field values ============
fn oor_values() -> Vec<u32> {
    vec![7, 8, 0x7f, 0x80, 0xff, 0x100, 0x103, 0x1ff, 0xffff, 0xffff_ff00, 0xffff_ffff]
}

unsafe fn addr_u32_case(p: &Pair, name: &[u8], label: &str) {
    let cf = sym!(p.c, name, FAddrU32);
    let rf = sym!(p.r, name, FAddrU32);
    let mut rng = Rng::new(SEED ^ 200);
    for base in [[0u32; 8], [0xFFFF_FFFFu32; 8], rng.addr()] {
        for v in oor_values() {
            let mut ca = base;
            let mut ra = base;
            cf(ca.as_mut_ptr(), v);
            rf(ra.as_mut_ptr(), v);
            eqb(&format!("{label}({v:#x})"), &addr_to_bytes(&ca), &addr_to_bytes(&ra));
        }
    }
}

#[test]
fn e22_set_type_out_of_range() {
    let p = pair();
    unsafe { addr_u32_case(p, b"SPX_set_type\0", "set_type") }
}

#[test]
fn e23_set_layer_out_of_range() {
    let p = pair();
    unsafe { addr_u32_case(p, b"SPX_set_layer_addr\0", "set_layer_addr") }
}

#[test]
fn e24_set_chain_out_of_range() {
    let p = pair();
    unsafe { addr_u32_case(p, b"SPX_set_chain_addr\0", "set_chain_addr") }
}

#[test]
fn e25_set_hash_out_of_range() {
    let p = pair();
    unsafe { addr_u32_case(p, b"SPX_set_hash_addr\0", "set_hash_addr") }
}

#[test]
fn e26_set_tree_height_out_of_range() {
    let p = pair();
    unsafe { addr_u32_case(p, b"SPX_set_tree_height\0", "set_tree_height") }
}

#[test]
fn e27_u32_fields_full_range() {
    let p = pair();
    unsafe {
        addr_u32_case(p, b"SPX_set_keypair_addr\0", "set_keypair_addr");
        addr_u32_case(p, b"SPX_set_tree_index\0", "set_tree_index");
    }
}

#[test]
fn e28_set_tree_addr_full_range() {
    let p = pair();
    unsafe {
        let cf = sym!(p.c, b"SPX_set_tree_addr\0", FAddrU64);
        let rf = sym!(p.r, b"SPX_set_tree_addr\0", FAddrU64);
        for base in [[0u32; 8], [0xFFFF_FFFFu32; 8]] {
            for v in [0u64, 1, 0xff, 0x100, u64::MAX, 0x0102_0304_0506_0708, 1u64 << 63] {
                let mut ca = base;
                let mut ra = base;
                cf(ca.as_mut_ptr(), v);
                rf(ra.as_mut_ptr(), v);
                eqb(&format!("set_tree_addr({v:#x})"), &addr_to_bytes(&ca), &addr_to_bytes(&ra));
            }
        }
    }
}

// ============ rows 29-32: utils edge cases ============
#[test]
fn e29_ull_to_bytes_zero_len() {
    let p = pair();
    unsafe {
        let cf = sym!(p.c, b"SPX_ull_to_bytes\0", FUll);
        let rf = sym!(p.r, b"SPX_ull_to_bytes\0", FUll);
        for v in [0u64, 1, u64::MAX] {
            let mut cb = vec![0x5Au8; 32];
            let mut rb = vec![0x5Au8; 32];
            cf(cb.as_mut_ptr(), 0, v);
            rf(rb.as_mut_ptr(), 0, v);
            eqb("ull_to_bytes outlen=0", &cb, &rb);
            eqv("nothing written", &cb, &vec![0x5Au8; 32]);
        }
    }
}

#[test]
fn e30_ull_to_bytes_oversized() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 130);
    unsafe {
        let cf = sym!(p.c, b"SPX_ull_to_bytes\0", FUll);
        let rf = sym!(p.r, b"SPX_ull_to_bytes\0", FUll);
        for outlen in [9u32, 10, 12, 16, 24, 32] {
            for _ in 0..16 {
                let v = rng.next_u64();
                let mut cb = vec![0x5Au8; 64];
                let mut rb = vec![0x5Au8; 64];
                cf(cb.as_mut_ptr(), outlen, v);
                rf(rb.as_mut_ptr(), outlen, v);
                eqb(&format!("ull_to_bytes outlen={outlen}"), &cb, &rb);
                // high bytes must be zero
                for i in 0..(outlen as usize - 8) {
                    eqv(&format!("high byte {i} zero"), cb[i], 0u8);
                }
            }
        }
    }
}

#[test]
fn e31_bytes_to_ull_zero_len() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 131);
    unsafe {
        let cf = sym!(p.c, b"SPX_bytes_to_ull\0", FB2U);
        let rf = sym!(p.r, b"SPX_bytes_to_ull\0", FB2U);
        for _ in 0..16 {
            let b = rng.bytes(16);
            let a = cf(b.as_ptr(), 0);
            let c = rf(b.as_ptr(), 0);
            eqv("bytes_to_ull inlen=0", a, c);
            eqv("bytes_to_ull inlen=0 is 0", a, 0u64);
        }
    }
}

#[test]
fn e32_bytes_to_ull_oversized() {
    // `inlen > 8` makes the C shift by >= 64, which is UB in C. We only record
    // what the compiled C .so actually does and require the Rust .so to agree;
    // if the two ever diverge this test documents the exact divergence rather
    // than silently ignoring the input class.
    let p = pair();
    let mut rng = Rng::new(SEED ^ 132);
    unsafe {
        let cf = sym!(p.c, b"SPX_bytes_to_ull\0", FB2U);
        let rf = sym!(p.r, b"SPX_bytes_to_ull\0", FB2U);
        let mut diffs = 0;
        let mut total = 0;
        for inlen in [9u32, 10, 12, 16] {
            for _ in 0..16 {
                let b = rng.bytes(32);
                let a = cf(b.as_ptr(), inlen);
                let c = rf(b.as_ptr(), inlen);
                total += 1;
                if a != c {
                    diffs += 1;
                    println!(
                        "[{}] bytes_to_ull inlen={inlen}: C={a:#x} R={c:#x} (C shift >= 64 is UB)",
                        cfg_name()
                    );
                }
            }
        }
        // The low 8 bytes always dominate; assert the observed behaviour matches
        // and report if the platform's UB resolution differs.
        assert_eq!(
            diffs, 0,
            "[{}] {diffs}/{total} bytes_to_ull(inlen>8) results diverged; the C shift-overflow \
             behaviour on this toolchain differs from Rust's masked shift",
            cfg_name()
        );
    }
}

// ============ rows 33-37: API success paths / no validation ============
#[test]
fn e33_sign_empty_message() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 133);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rkp = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);
        let csg = sym!(p.c, b"crypto_sign_signature\0", FSignature);
        let rsg = sym!(p.r, b"crypto_sign_signature\0", FSignature);
        let seed = rng.bytes(SEED_BYTES);
        let mut cpk = vec![0u8; PK_BYTES];
        let mut rpk = vec![0u8; PK_BYTES];
        let mut csk = vec![0u8; SK_BYTES];
        let mut rsk = vec![0u8; SK_BYTES];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());

        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());

        let m: Vec<u8> = vec![];
        let mut csig = obuf(SPX_BYTES);
        let mut rsig = obuf(SPX_BYTES);
        let mut csl = 0xDEADusize;
        let mut rsl = 0xDEADusize;
        let a = csg(csig.as_mut_ptr(), &mut csl, m.as_ptr(), 0, csk.as_ptr());
        let b = rsg(rsig.as_mut_ptr(), &mut rsl, m.as_ptr(), 0, rsk.as_ptr());
        eqv("signature(mlen=0) ret", a, b);
        eqv("signature(mlen=0) ret is 0", a, 0);
        eqv("siglen", csl, rsl);
        eqv("siglen is SPX_BYTES", csl, SPX_BYTES);
        eqb("signature bytes", &csig, &rsig);
    }
}

#[test]
fn e34_sign_open_empty_roundtrip() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 134);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let ckp = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rkp = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);
        let cs = sym!(p.c, b"crypto_sign\0", FSign);
        let rs = sym!(p.r, b"crypto_sign\0", FSign);
        let co = sym!(p.c, b"crypto_sign_open\0", FOpen);
        let ro = sym!(p.r, b"crypto_sign_open\0", FOpen);

        let seed = rng.bytes(SEED_BYTES);
        let mut cpk = vec![0u8; PK_BYTES];
        let mut rpk = vec![0u8; PK_BYTES];
        let mut csk = vec![0u8; SK_BYTES];
        let mut rsk = vec![0u8; SK_BYTES];
        ckp(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
        rkp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());

        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());

        let m: Vec<u8> = vec![];
        let mut csm = obuf(SPX_BYTES);
        let mut rsm = obuf(SPX_BYTES);
        let mut csl = 0u64;
        let mut rsl = 0u64;
        let a = cs(csm.as_mut_ptr(), &mut csl, m.as_ptr(), 0, csk.as_ptr());
        let b = rs(rsm.as_mut_ptr(), &mut rsl, m.as_ptr(), 0, rsk.as_ptr());
        eqv("crypto_sign(mlen=0) ret", a, b);
        eqv("smlen", csl, rsl);
        eqv("smlen is SPX_BYTES", csl as usize, SPX_BYTES);
        eqb("sm", &csm, &rsm);

        let mut cm = obuf(SPX_BYTES);
        let mut rm = obuf(SPX_BYTES);
        let mut cml = 0xDEADu64;
        let mut rml = 0xDEADu64;
        let a = co(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), csl, cpk.as_ptr());
        let b = ro(rm.as_mut_ptr(), &mut rml, rsm.as_ptr(), rsl, rpk.as_ptr());
        eqv("open ret", a, b);
        eqv("open ret is 0", a, 0);
        eqv("open mlen", cml, rml);
        eqv("open mlen is 0", cml, 0u64);
        eqb("open m", &cm, &rm);
    }
}

#[test]
fn e35_seed_keypair_always_zero() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 135);
    unsafe {
        let cf = sym!(p.c, b"crypto_sign_seed_keypair\0", FSeedKp);
        let rf = sym!(p.r, b"crypto_sign_seed_keypair\0", FSeedKp);
        for seed in [vec![0u8; SEED_BYTES], vec![0xffu8; SEED_BYTES], rng.bytes(SEED_BYTES)] {
            let mut cpk = obuf(PK_BYTES);
            let mut rpk = obuf(PK_BYTES);
            let mut csk = obuf(SK_BYTES);
            let mut rsk = obuf(SK_BYTES);
            let a = cf(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr());
            let b = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr());
            eqv("seed_keypair ret", a, b);
            eqv("seed_keypair ret is 0", a, 0);
            eqb("pk", &cpk, &rpk);
            eqb("sk", &csk, &rsk);
        }
    }
}

#[test]
fn e36_keypair_always_zero() {
    let _g = drbg_lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 136);
    unsafe {
        let ci = sym!(p.c, b"randombytes_init\0", FRbInit);
        let ri = sym!(p.r, b"randombytes_init\0", FRbInit);
        let cf = sym!(p.c, b"crypto_sign_keypair\0", FKp);
        let rf = sym!(p.r, b"crypto_sign_keypair\0", FKp);
        let mut e1 = rng.bytes(48);
        let mut e2 = e1.clone();
        ci(e1.as_mut_ptr(), core::ptr::null_mut());
        ri(e2.as_mut_ptr(), core::ptr::null_mut());
        let mut cpk = obuf(PK_BYTES);
        let mut rpk = obuf(PK_BYTES);
        let mut csk = obuf(SK_BYTES);
        let mut rsk = obuf(SK_BYTES);
        let a = cf(cpk.as_mut_ptr(), csk.as_mut_ptr());
        let b = rf(rpk.as_mut_ptr(), rsk.as_mut_ptr());
        eqv("keypair ret", a, b);
        eqv("keypair ret is 0", a, 0);
        eqb("pk", &cpk, &rpk);
        eqb("sk", &csk, &rsk);
    }
}

#[test]
fn e37_size_getters() {
    let p = pair();
    unsafe {
        for (name, expect) in [
            (&b"crypto_sign_secretkeybytes\0"[..], SK_BYTES),
            (&b"crypto_sign_publickeybytes\0"[..], PK_BYTES),
            (&b"crypto_sign_bytes\0"[..], SPX_BYTES),
            (&b"crypto_sign_seedbytes\0"[..], SEED_BYTES),
        ] {
            let cf = sym!(p.c, name, FSizes);
            let rf = sym!(p.r, name, FSizes);
            eqv(&format!("{}", String::from_utf8_lossy(name)), cf(), rf());
            eqv(&format!("{} value", String::from_utf8_lossy(name)), cf() as usize, expect);
            // repeated calls are stable
            eqv("stable", cf(), cf());
        }
    }
}

// ============ rows 38-39: thash degenerate / branch boundary ============
#[test]
fn e38_thash_zero_inblocks() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 138);
    unsafe {
        let cf = sym!(p.c, b"SPX_thash\0", FThash);
        let rf = sym!(p.r, b"SPX_thash\0", FThash);
        for _ in 0..16 {
            let ps = rng.bytes(N);
            let ss = rng.bytes(N);
            let cc = make_ctx(&p.c, &ps, &ss);
            let rc = make_ctx(&p.r, &ps, &ss);
            let a = rng.addr();
            let mut ca = a;
            let mut ra = a;
            let inp = rng.bytes(16);
            let mut co = obuf(N);
            let mut ro = obuf(N);
            cf(co.as_mut_ptr(), inp.as_ptr(), 0, cc.as_ptr(), ca.as_mut_ptr());
            rf(ro.as_mut_ptr(), inp.as_ptr(), 0, rc.as_ptr(), ra.as_mut_ptr());
            eqb("thash inblocks=0", &co, &ro);
            eqb("thash inblocks=0 addr", &addr_to_bytes(&ca), &addr_to_bytes(&ra));
        }
    }
}

#[test]
fn e39_thash_branch_boundary() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 139);
    unsafe {
        let cf = sym!(p.c, b"SPX_thash\0", FThash);
        let rf = sym!(p.r, b"SPX_thash\0", FThash);
        for ib in [1u32, 2] {
            for _ in 0..24 {
                let ps = rng.bytes(N);
                let ss = rng.bytes(N);
                let cc = make_ctx(&p.c, &ps, &ss);
                let rc = make_ctx(&p.r, &ps, &ss);
                let a = rng.addr();
                let mut ca = a;
                let mut ra = a;
                let inp = rng.bytes(ib as usize * N);
                let mut co = obuf(N);
                let mut ro = obuf(N);
                cf(co.as_mut_ptr(), inp.as_ptr(), ib, cc.as_ptr(), ca.as_mut_ptr());
                rf(ro.as_mut_ptr(), inp.as_ptr(), ib, rc.as_ptr(), ra.as_mut_ptr());
                eqb(&format!("thash inblocks={ib}"), &co, &ro);
                eqb(&format!("thash inblocks={ib} addr"), &addr_to_bytes(&ca), &addr_to_bytes(&ra));
            }
        }
        // the 1 -> 2 transition must actually select a different primitive when
        // SPX_SHA512 / SPX_BLAKE512 is on; a single block repeated twice must
        // not equal the 1-block digest.
        let ps = rng.bytes(N);
        let ss = rng.bytes(N);
        let cc = make_ctx(&p.c, &ps, &ss);
        let a = rng.addr();
        let mut a1 = a;
        let mut a2 = a;
        let inp = rng.bytes(2 * N);
        let mut o1 = obuf(N);
        let mut o2 = obuf(N);
        cf(o1.as_mut_ptr(), inp.as_ptr(), 1, cc.as_ptr(), a1.as_mut_ptr());
        cf(o2.as_mut_ptr(), inp.as_ptr(), 2, cc.as_ptr(), a2.as_mut_ptr());
        assert_ne!(o1, o2, "[{}] thash(1) and thash(2) must differ", cfg_name());
    }
}
