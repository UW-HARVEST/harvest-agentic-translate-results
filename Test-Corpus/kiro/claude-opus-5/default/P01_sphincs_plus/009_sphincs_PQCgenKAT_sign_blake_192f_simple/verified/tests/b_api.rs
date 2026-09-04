//! Phase B, CONFIGS.md rows 41-47: the public `app/include/api.h` surface.
//!
//! `crypto_sign_keypair` and `crypto_sign_signature` call `randombytes()`.  In
//! the default configuration that is `rng.c`'s deterministic CTR_DRBG, so both
//! sides are re-seeded with the same entropy before every call and their output
//! must agree bit for bit.  Under the `urandom` feature the provider is
//! `/dev/urandom`, so only the deterministic entry points can be compared
//! directly and the randomised ones are covered by the cross-library round trip
//! of row 47.

mod common;

use common::params::*;
use common::*;

type SizeFn = unsafe extern "C" fn() -> u64;
type SeedKeypair = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type Keypair = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
type Signature = unsafe extern "C" fn(*mut u8, *mut usize, *const u8, usize, *const u8) -> i32;
type Verify = unsafe extern "C" fn(*const u8, usize, *const u8, usize, *const u8) -> i32;
type Sign = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type SignOpen = unsafe extern "C" fn(*mut u8, *mut u64, *const u8, u64, *const u8) -> i32;
type RandombytesInit = unsafe extern "C" fn(*mut u8, *mut u8);

const DRBG_DETERMINISTIC: bool = !cfg!(feature = "urandom");

/// Re-seeds both DRBGs with the same entropy so that the `optrand` drawn inside
/// `crypto_sign_signature` matches.
fn sync_drbg(libs: &Libs, entropy: &mut [u8; 48]) {
    let (ic, ir) = libs.pair::<RandombytesInit>("randombytes_init");
    unsafe {
        ic(entropy.as_mut_ptr(), core::ptr::null_mut());
        ir(entropy.as_mut_ptr(), core::ptr::null_mut());
    }
}

/// A key pair that both sides agree on, produced from a seed (no randombytes).
fn keypair(libs: &Libs, rng: &mut Rng) -> (Vec<u8>, Vec<u8>) {
    let (fc, fr) = libs.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    let seed = rng.bytes(CRYPTO_SEEDBYTES);
    let mut pka = vec![0xA5u8; SPX_PK_BYTES + 8];
    let mut ska = vec![0xA5u8; SPX_SK_BYTES + 8];
    let mut pkb = vec![0xA5u8; SPX_PK_BYTES + 8];
    let mut skb = vec![0xA5u8; SPX_SK_BYTES + 8];
    let (ra, rb) = unsafe {
        (
            fc(pka.as_mut_ptr(), ska.as_mut_ptr(), seed.as_ptr()),
            fr(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed.as_ptr()),
        )
    };
    assert_eq!(ra, 0);
    assert_eq!(rb, 0);
    eq("crypto_sign_seed_keypair pk", &pka, &pkb);
    eq("crypto_sign_seed_keypair sk", &ska, &skb);
    (pka[..SPX_PK_BYTES].to_vec(), ska[..SPX_SK_BYTES].to_vec())
}

#[test]
fn row41_size_functions() {
    let libs = load();
    for (name, expect) in [
        ("crypto_sign_secretkeybytes", SPX_SK_BYTES as u64),
        ("crypto_sign_publickeybytes", SPX_PK_BYTES as u64),
        ("crypto_sign_bytes", SPX_BYTES as u64),
        ("crypto_sign_seedbytes", CRYPTO_SEEDBYTES as u64),
    ] {
        let (fc, fr) = libs.pair::<SizeFn>(name);
        unsafe {
            assert_eq!(fc(), fr(), "{name}");
            assert_eq!(fc(), expect, "{name} against params");
        }
    }
    eprintln!(
        "[{}] N={} D={} h={} FORS {}x2^{} WOTS_LEN={} SPX_BYTES={}",
        tag(),
        SPX_N,
        SPX_D,
        SPX_TREE_HEIGHT,
        SPX_FORS_TREES,
        SPX_FORS_HEIGHT,
        SPX_WOTS_LEN,
        SPX_BYTES
    );
}

#[test]
fn row42_seed_keypair() {
    let libs = load();
    let mut rng = Rng::new(42);
    for _ in 0..2 {
        keypair(&libs, &mut rng);
    }
    // extreme seeds
    let (fc, fr) = libs.pair::<SeedKeypair>("crypto_sign_seed_keypair");
    for seed in [
        vec![0u8; CRYPTO_SEEDBYTES],
        vec![0xFFu8; CRYPTO_SEEDBYTES],
    ] {
        let mut pka = vec![0u8; SPX_PK_BYTES];
        let mut ska = vec![0u8; SPX_SK_BYTES];
        let mut pkb = vec![0u8; SPX_PK_BYTES];
        let mut skb = vec![0u8; SPX_SK_BYTES];
        unsafe {
            fc(pka.as_mut_ptr(), ska.as_mut_ptr(), seed.as_ptr());
            fr(pkb.as_mut_ptr(), skb.as_mut_ptr(), seed.as_ptr());
        }
        eq("crypto_sign_seed_keypair pk (extreme)", &pka, &pkb);
        eq("crypto_sign_seed_keypair sk (extreme)", &ska, &skb);
        // sk layout: [SK_SEED || SK_PRF || PUB_SEED || root], pk: [PUB_SEED || root]
        assert_eq!(&ska[2 * SPX_N..3 * SPX_N], &pka[..SPX_N]);
        assert_eq!(&ska[3 * SPX_N..4 * SPX_N], &pka[SPX_N..2 * SPX_N]);
    }
}

#[test]
fn row43_keypair_from_drbg() {
    if !DRBG_DETERMINISTIC {
        eprintln!("[{}] row43 skipped: urandom provider is non-deterministic", tag());
        return;
    }
    let libs = load();
    let (fc, fr) = libs.pair::<Keypair>("crypto_sign_keypair");
    let drbg_c = libs.c::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let drbg_r = libs.r::<*mut Aes256CtrDrbgStruct>("DRBG_ctx");
    let mut rng = Rng::new(43);
    for _ in 0..2 {
        let mut entropy = [0u8; 48];
        rng.fill(&mut entropy);
        sync_drbg(&libs, &mut entropy);
        let mut pka = vec![0xA5u8; SPX_PK_BYTES + 8];
        let mut ska = vec![0xA5u8; SPX_SK_BYTES + 8];
        let mut pkb = vec![0xA5u8; SPX_PK_BYTES + 8];
        let mut skb = vec![0xA5u8; SPX_SK_BYTES + 8];
        unsafe {
            assert_eq!(fc(pka.as_mut_ptr(), ska.as_mut_ptr()), 0);
            assert_eq!(fr(pkb.as_mut_ptr(), skb.as_mut_ptr()), 0);
        }
        eq("crypto_sign_keypair pk", &pka, &pkb);
        eq("crypto_sign_keypair sk", &ska, &skb);
        unsafe {
            eq(
                "DRBG_ctx after keypair",
                (**drbg_c).as_bytes(),
                (**drbg_r).as_bytes(),
            );
        }
    }
}

#[test]
fn row44_45_46_signature_verify_open() {
    let libs = load();
    let (sc, sr) = libs.pair::<Signature>("crypto_sign_signature");
    let (vc, vr) = libs.pair::<Verify>("crypto_sign_verify");
    let (gc, gr) = libs.pair::<Sign>("crypto_sign");
    let (oc, or) = libs.pair::<SignOpen>("crypto_sign_open");
    let mut rng = Rng::new(44);
    let (pk, sk) = keypair(&libs, &mut rng);

    for (mi, &mlen) in MLEN_SWEEP_SMALL.iter().enumerate() {
        let m = rng.bytes(mlen);
        // The combined form is the detached form plus a memmove, so run it for
        // the first and last length only.
        let do_combined = mi == 0 || mi + 1 == MLEN_SWEEP_SMALL.len();

        // ---- row 44: detached signature -------------------------------
        let mut siga = vec![0xA5u8; SPX_BYTES + 8];
        let mut sigb = vec![0xA5u8; SPX_BYTES + 8];
        let mut la = usize::MAX;
        let mut lb = usize::MAX;
        if DRBG_DETERMINISTIC {
            let mut entropy = [0u8; 48];
            rng.fill(&mut entropy);
            sync_drbg(&libs, &mut entropy);
        }
        let (ra, rb) = unsafe {
            (
                sc(siga.as_mut_ptr(), &mut la, m.as_ptr(), mlen, sk.as_ptr()),
                sr(sigb.as_mut_ptr(), &mut lb, m.as_ptr(), mlen, sk.as_ptr()),
            )
        };
        assert_eq!(ra, 0);
        assert_eq!(rb, 0);
        assert_eq!(la, SPX_BYTES, "C siglen");
        assert_eq!(lb, SPX_BYTES, "Rust siglen");
        if DRBG_DETERMINISTIC {
            eq(&format!("crypto_sign_signature(mlen={mlen})"), &siga, &sigb);
        }

        // ---- row 45: verify, each side against both signatures --------
        for (label, sig) in [("C sig", &siga), ("Rust sig", &sigb)] {
            let cv = unsafe { vc(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            let rv = unsafe { vr(sig.as_ptr(), SPX_BYTES, m.as_ptr(), mlen, pk.as_ptr()) };
            assert_eq!(cv, 0, "C verify rejected {label} (mlen={mlen})");
            assert_eq!(rv, 0, "Rust verify rejected {label} (mlen={mlen})");
        }

        // ---- row 46: combined form ------------------------------------
        if !do_combined {
            continue;
        }
        let mut sma = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut smb = vec![0xA5u8; SPX_BYTES + mlen + 8];
        let mut sla = u64::MAX;
        let mut slb = u64::MAX;
        if DRBG_DETERMINISTIC {
            let mut entropy = [0u8; 48];
            rng.fill(&mut entropy);
            sync_drbg(&libs, &mut entropy);
        }
        unsafe {
            assert_eq!(
                gc(sma.as_mut_ptr(), &mut sla, m.as_ptr(), mlen as u64, sk.as_ptr()),
                0
            );
            assert_eq!(
                gr(smb.as_mut_ptr(), &mut slb, m.as_ptr(), mlen as u64, sk.as_ptr()),
                0
            );
        }
        assert_eq!(sla, (SPX_BYTES + mlen) as u64);
        assert_eq!(slb, (SPX_BYTES + mlen) as u64);
        if DRBG_DETERMINISTIC {
            eq(&format!("crypto_sign(mlen={mlen})"), &sma, &smb);
        }

        for (label, sm, smlen) in [("C sm", &sma, sla), ("Rust sm", &smb, slb)] {
            let mut ma = vec![0x5Au8; smlen as usize + 8];
            let mut mb = vec![0x5Au8; smlen as usize + 8];
            let mut mla = u64::MAX;
            let mut mlb = u64::MAX;
            let (ca, cb) = unsafe {
                (
                    oc(ma.as_mut_ptr(), &mut mla, sm.as_ptr(), smlen, pk.as_ptr()),
                    or(mb.as_mut_ptr(), &mut mlb, sm.as_ptr(), smlen, pk.as_ptr()),
                )
            };
            assert_eq!(ca, 0, "C crypto_sign_open rejected {label}");
            assert_eq!(cb, 0, "Rust crypto_sign_open rejected {label}");
            assert_eq!(mla, mlen as u64);
            assert_eq!(mlb, mlen as u64);
            eq(&format!("crypto_sign_open m ({label}, mlen={mlen})"), &ma, &mb);
            assert_eq!(&ma[..mlen], &m[..], "recovered message differs");
        }
    }
}

#[test]
fn row47_cross_library_round_trip() {
    let libs = load();
    let (sc, sr) = libs.pair::<Signature>("crypto_sign_signature");
    let (vc, vr) = libs.pair::<Verify>("crypto_sign_verify");
    let mut rng = Rng::new(47);
    let (pk, sk) = keypair(&libs, &mut rng);
    for &mlen in &[0usize, 137] {
        let m = rng.bytes(mlen);
        let mut sig_c = vec![0u8; SPX_BYTES];
        let mut sig_r = vec![0u8; SPX_BYTES];
        let mut lc = 0usize;
        let mut lr = 0usize;
        unsafe {
            assert_eq!(sc(sig_c.as_mut_ptr(), &mut lc, m.as_ptr(), mlen, sk.as_ptr()), 0);
            assert_eq!(sr(sig_r.as_mut_ptr(), &mut lr, m.as_ptr(), mlen, sk.as_ptr()), 0);
            // C signs -> Rust verifies
            assert_eq!(
                vr(sig_c.as_ptr(), lc, m.as_ptr(), mlen, pk.as_ptr()),
                0,
                "Rust rejected a C signature (mlen={mlen})"
            );
            // Rust signs -> C verifies
            assert_eq!(
                vc(sig_r.as_ptr(), lr, m.as_ptr(), mlen, pk.as_ptr()),
                0,
                "C rejected a Rust signature (mlen={mlen})"
            );
        }
    }
}
