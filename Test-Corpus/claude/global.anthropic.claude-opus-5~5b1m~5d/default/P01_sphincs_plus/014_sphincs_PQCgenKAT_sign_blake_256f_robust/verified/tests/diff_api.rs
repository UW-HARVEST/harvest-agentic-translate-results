//! Phase B — differential tests for the public API in `app/src/sign.c`
//! (`api.h`).  Because the C library under test is the *deterministic* core
//! (`rng.c`), `crypto_sign_keypair` / `crypto_sign_signature` / `crypto_sign`
//! are reproducible and can be compared byte-for-byte once both DRBGs have been
//! reseeded with the same entropy.

mod common;
use common::*;

type Sizes = unsafe extern "C" fn() -> u64;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);

/// Message lengths used for the (expensive) full-signature comparisons.  The
/// `s` parameter sets have 2^9 leaves per subtree, so a single signature costs
/// ~2M thash calls; a reduced set keeps the suite inside its time budget while
/// the per-message-length boundaries are already covered exhaustively by
/// `diff_core.rs` (gen_message_random / hash_message).
fn api_msg_lens() -> Vec<usize> {
    if TREE_HEIGHT >= 8 {
        vec![0, 1, 64, 137, 1000]
    } else {
        vec![0, 1, 2, 33, 63, 64, 65, 127, 128, 129, 136, 137, 1000]
    }
}

#[test]
fn size_getters_match() {
    let libs = Libs::load();
    for (name, expected) in [
        ("crypto_sign_secretkeybytes", SK_BYTES as u64),
        ("crypto_sign_publickeybytes", PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", SEED_BYTES as u64),
    ] {
        let (c, r) = libs.pair::<Sizes>(name);
        let cv = unsafe { c() };
        let rv = unsafe { r() };
        assert_eq!(cv, rv, "{}", name);
        assert_eq!(cv, expected, "{} disagrees with the header formula", name);
    }
}

#[test]
fn seed_keypair_matches() {
    let libs = Libs::load();
    let (c, r) = libs.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x1000);
    let mut seeds: Vec<Vec<u8>> = vec![vec![0u8; SEED_BYTES], vec![0xffu8; SEED_BYTES]];
    for _ in 0..6 {
        seeds.push(rng.bytes(SEED_BYTES));
    }
    for seed in seeds {
        let mut cpk = vec![0xEEu8; PK_BYTES + 8];
        let mut csk = vec![0xEEu8; SK_BYTES + 8];
        let mut rpk = vec![0xEEu8; PK_BYTES + 8];
        let mut rsk = vec![0xEEu8; SK_BYTES + 8];
        let crc = unsafe { c(cpk.as_mut_ptr(), csk.as_mut_ptr(), seed.as_ptr()) };
        let rrc = unsafe { r(rpk.as_mut_ptr(), rsk.as_mut_ptr(), seed.as_ptr()) };
        assert_eq!(crc, rrc, "crypto_sign_seed_keypair rc");
        assert_eq!(crc, 0);
        assert_bytes_eq("crypto_sign_seed_keypair pk", &cpk, &rpk);
        assert_bytes_eq("crypto_sign_seed_keypair sk", &csk, &rsk);
    }
}

#[test]
fn keypair_matches_after_identical_reseed() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (c, r) = libs.pair::<Keypair>("crypto_sign_keypair");
    let mut rng = Rng::new(0x1001);

    for _ in 0..4 {
        let mut ent = rng.bytes(48);
        let mut cpk = vec![0xEEu8; PK_BYTES + 8];
        let mut csk = vec![0xEEu8; SK_BYTES + 8];
        let mut rpk = vec![0xEEu8; PK_BYTES + 8];
        let mut rsk = vec![0xEEu8; SK_BYTES + 8];
        unsafe {
            ci(ent.as_mut_ptr(), std::ptr::null_mut());
            let crc = c(cpk.as_mut_ptr(), csk.as_mut_ptr());
            ri(ent.as_mut_ptr(), std::ptr::null_mut());
            let rrc = r(rpk.as_mut_ptr(), rsk.as_mut_ptr());
            assert_eq!(crc, rrc, "crypto_sign_keypair rc");
            assert_eq!(crc, 0);
        }
        assert_bytes_eq("crypto_sign_keypair pk", &cpk, &rpk);
        assert_bytes_eq("crypto_sign_keypair sk", &csk, &rsk);
    }
}

#[test]
fn signature_verify_and_cross_verify() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (csig, rsig) = libs.pair::<Signature>("crypto_sign_signature");
    let (cver, rver) = libs.pair::<Verify>("crypto_sign_verify");
    let kp = libs.c::<SeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x1002);

    let seed = rng.bytes(SEED_BYTES);
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    for mlen in api_msg_lens() {
        let m = rng.bytes(mlen);
        let mut ent = rng.bytes(48);

        let mut cs = vec![0xEEu8; SPX_BYTES + 8];
        let mut rs = vec![0xEEu8; SPX_BYTES + 8];
        let mut clen = usize::MAX;
        let mut rlen = usize::MAX;
        unsafe {
            ci(ent.as_mut_ptr(), std::ptr::null_mut());
            let crc = csig(cs.as_mut_ptr(), &mut clen, m.as_ptr(), mlen, sk.as_ptr());
            ri(ent.as_mut_ptr(), std::ptr::null_mut());
            let rrc = rsig(rs.as_mut_ptr(), &mut rlen, m.as_ptr(), mlen, sk.as_ptr());
            assert_eq!(crc, rrc, "crypto_sign_signature rc (mlen={})", mlen);
            assert_eq!(crc, 0);
        }
        assert_eq!(clen, rlen, "siglen (mlen={})", mlen);
        assert_eq!(clen, SPX_BYTES, "siglen must be SPX_BYTES");
        assert_bytes_eq(&format!("crypto_sign_signature (mlen={})", mlen), &cs, &rs);

        // both libraries must accept both signatures
        for (label, s) in [("C sig", &cs), ("Rust sig", &rs)] {
            let cv = unsafe { cver(s.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            let rv = unsafe { rver(s.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            assert_eq!(cv, 0, "C verify rejected {} (mlen={})", label, mlen);
            assert_eq!(rv, 0, "Rust verify rejected {} (mlen={})", label, mlen);
        }
    }
}

#[test]
fn sign_and_open_matches() {
    let _g = drbg_lock();
    let libs = Libs::load();
    let (ci, ri) = libs.pair::<RandombytesInit>("randombytes_init");
    let (cs, rs) = libs.pair::<Sign>("crypto_sign");
    let (co, ro) = libs.pair::<SignOpen>("crypto_sign_open");
    let kp = libs.c::<SeedKeypair>("crypto_sign_seed_keypair");
    let mut rng = Rng::new(0x1003);

    let seed = rng.bytes(SEED_BYTES);
    let mut pk = vec![0u8; PK_BYTES];
    let mut sk = vec![0u8; SK_BYTES];
    unsafe { kp(pk.as_mut_ptr(), sk.as_mut_ptr(), seed.as_ptr()) };

    for mlen in api_msg_lens() {
        let m = rng.bytes(mlen);
        let mut ent = rng.bytes(48);

        let mut csm = vec![0xEEu8; SPX_BYTES + mlen + 8];
        let mut rsm = vec![0xEEu8; SPX_BYTES + mlen + 8];
        let mut csmlen = u64::MAX;
        let mut rsmlen = u64::MAX;
        unsafe {
            ci(ent.as_mut_ptr(), std::ptr::null_mut());
            let crc = cs(csm.as_mut_ptr(), &mut csmlen, m.as_ptr(), mlen as u64, sk.as_ptr());
            ri(ent.as_mut_ptr(), std::ptr::null_mut());
            let rrc = rs(rsm.as_mut_ptr(), &mut rsmlen, m.as_ptr(), mlen as u64, sk.as_ptr());
            assert_eq!(crc, rrc, "crypto_sign rc (mlen={})", mlen);
            assert_eq!(crc, 0);
        }
        assert_eq!(csmlen, rsmlen, "smlen (mlen={})", mlen);
        assert_eq!(csmlen, (SPX_BYTES + mlen) as u64);
        assert_bytes_eq(&format!("crypto_sign sm (mlen={})", mlen), &csm, &rsm);

        // crypto_sign_open on the produced sm
        let smlen = csmlen;
        let mut cm = vec![0xEEu8; SPX_BYTES + mlen + 8];
        let mut rm = vec![0xEEu8; SPX_BYTES + mlen + 8];
        let mut cml = u64::MAX;
        let mut rml = u64::MAX;
        let crc = unsafe { co(cm.as_mut_ptr(), &mut cml, csm.as_ptr(), smlen, pk.as_ptr()) };
        let rrc = unsafe { ro(rm.as_mut_ptr(), &mut rml, csm.as_ptr(), smlen, pk.as_ptr()) };
        assert_eq!(crc, rrc, "crypto_sign_open rc (mlen={})", mlen);
        assert_eq!(crc, 0);
        assert_eq!(cml, rml, "crypto_sign_open mlen (mlen={})", mlen);
        assert_eq!(cml, mlen as u64);
        assert_bytes_eq(&format!("crypto_sign_open m (mlen={})", mlen), &cm, &rm);
        assert_bytes_eq("recovered message", &cm[..mlen], &m);
    }
}
