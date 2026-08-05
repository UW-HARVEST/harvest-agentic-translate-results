//! Differential tests for the KEM + IPCRYPT family:
//!   - crypto_kem_mlkem768  (ML-KEM-768)
//!   - crypto_kem_xwing     (X-Wing hybrid)
//!   - crypto_ipcrypt       (ipcrypt: deterministic / nd / ndx / pfx variants)
//!
//! Every call goes through the exported C symbol loaded from BOTH the C `.so`
//! and the Rust `.so`; outputs are compared byte-for-byte. The C library is the
//! ground truth.
//!
//! All comparisons use DETERMINISTIC entry points (seed_keypair /
//! enc_deterministic for the KEMs; ipcrypt is inherently deterministic) so the
//! two libraries must produce bit-identical output. We additionally exercise
//! cross-library round-trips (encaps on C -> decaps on Rust and vice versa).

mod common;
use common::{libs, Rng};

// ---------- ML-KEM-768 constants (from crypto_kem_mlkem768.h) ----------
const MLKEM_PK: usize = 1184;
const MLKEM_SK: usize = 2400;
const MLKEM_CT: usize = 1088;
const MLKEM_SS: usize = 32;
const MLKEM_KP_SEED: usize = 64; // seed_keypair seed
const MLKEM_ENC_SEED: usize = 32; // enc_deterministic seed

// ---------- X-Wing constants (from crypto_kem_xwing.h) ----------
const XWING_PK: usize = 1216;
const XWING_SK: usize = 32;
const XWING_CT: usize = 1120;
const XWING_SS: usize = 32;
const XWING_KP_SEED: usize = 32; // seed_keypair seed
const XWING_ENC_SEED: usize = 64; // enc_deterministic seed

// ---------- ipcrypt constants (from crypto_ipcrypt.h) ----------
const IP_BYTES: usize = 16;
const IP_KEY: usize = 16;
const IP_ND_KEY: usize = 16;
const IP_ND_TWEAK: usize = 8;
const IP_ND_IN: usize = 16;
const IP_ND_OUT: usize = 24;
const IP_NDX_KEY: usize = 32;
const IP_NDX_TWEAK: usize = 16;
const IP_NDX_IN: usize = 16;
const IP_NDX_OUT: usize = 32;
const IP_PFX_KEY: usize = 32;
const IP_PFX_BYTES: usize = 16;

type SeedKeypairFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8) -> i32;
type EncDetFn = unsafe extern "C" fn(*mut u8, *mut u8, *const u8, *const u8) -> i32;
type DecFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8) -> i32;
// ipcrypt one-block: (out, in, k)
type IpFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8);
// ipcrypt tweaked encrypt: (out, in, t, k)
type IpTweakFn = unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8);

const ITERS: usize = 128;

// =====================================================================
// Phase B — ML-KEM-768 valid path (fully deterministic, byte-for-byte)
// =====================================================================
#[test]
fn mlkem768_keypair_encaps_decaps_byte_equal() {
    let l = libs();
    unsafe {
        let (c_kp, r_kp) = sympair!(l, b"crypto_kem_mlkem768_seed_keypair", SeedKeypairFn);
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_mlkem768_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_mlkem768_dec", DecFn);

        let mut rng = Rng::new(0x00A1_1234);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(MLKEM_KP_SEED);
            let enc_seed = rng.vec(MLKEM_ENC_SEED);

            // --- seed_keypair ---
            let (mut cpk, mut rpk) = (vec![0u8; MLKEM_PK], vec![0u8; MLKEM_PK]);
            let (mut csk, mut rsk) = (vec![0u8; MLKEM_SK], vec![0u8; MLKEM_SK]);
            let rc_c = c_kp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kp_seed.as_ptr());
            let rc_r = r_kp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kp_seed.as_ptr());
            assert_eq!(rc_c, rc_r, "mlkem seed_keypair rc");
            assert_eq!(cpk, rpk, "mlkem pk");
            assert_eq!(csk, rsk, "mlkem sk");

            // --- enc_deterministic (use shared pk) ---
            let (mut cct, mut rct) = (vec![0u8; MLKEM_CT], vec![0u8; MLKEM_CT]);
            let (mut css, mut rss) = (vec![0u8; MLKEM_SS], vec![0u8; MLKEM_SS]);
            let ec_c = c_enc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), enc_seed.as_ptr());
            let ec_r = r_enc(rct.as_mut_ptr(), rss.as_mut_ptr(), cpk.as_ptr(), enc_seed.as_ptr());
            assert_eq!(ec_c, ec_r, "mlkem enc rc");
            assert_eq!(ec_c, 0, "mlkem enc should succeed on canonical pk");
            assert_eq!(cct, rct, "mlkem ct");
            assert_eq!(css, rss, "mlkem enc ss");

            // --- dec (both libs on the shared ct/sk) ---
            let (mut css_d, mut rss_d) = (vec![0u8; MLKEM_SS], vec![0u8; MLKEM_SS]);
            let dc_c = c_dec(css_d.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            let dc_r = r_dec(rss_d.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            assert_eq!(dc_c, dc_r, "mlkem dec rc");
            assert_eq!(css_d, rss_d, "mlkem dec ss byte-equal");
            // KEM correctness: decapsulated == encapsulated
            assert_eq!(css_d, css, "mlkem dec == enc ss");
        }
    }
}

// Cross-library: encaps on C, decaps on Rust yields the same shared secret,
// and encaps on Rust, decaps on C also matches.
#[test]
fn mlkem768_cross_library_roundtrip() {
    let l = libs();
    unsafe {
        let (c_kp, r_kp) = sympair!(l, b"crypto_kem_mlkem768_seed_keypair", SeedKeypairFn);
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_mlkem768_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_mlkem768_dec", DecFn);

        let mut rng = Rng::new(0x5151_5151);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(MLKEM_KP_SEED);
            let enc_seed = rng.vec(MLKEM_ENC_SEED);

            // Keypair on C.
            let mut pk = vec![0u8; MLKEM_PK];
            let mut sk = vec![0u8; MLKEM_SK];
            assert_eq!(c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), kp_seed.as_ptr()), 0);
            // (sanity) Rust keypair matches.
            let mut pk2 = vec![0u8; MLKEM_PK];
            let mut sk2 = vec![0u8; MLKEM_SK];
            assert_eq!(r_kp(pk2.as_mut_ptr(), sk2.as_mut_ptr(), kp_seed.as_ptr()), 0);
            assert_eq!(pk, pk2);
            assert_eq!(sk, sk2);

            // Encaps on Rust.
            let mut ct = vec![0u8; MLKEM_CT];
            let mut ss_r = vec![0u8; MLKEM_SS];
            assert_eq!(
                r_enc(ct.as_mut_ptr(), ss_r.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            // Decaps on C.
            let mut ss_c = vec![0u8; MLKEM_SS];
            assert_eq!(c_dec(ss_c.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()), 0);
            assert_eq!(ss_r, ss_c, "Rust-encaps -> C-decaps");

            // Encaps on C, decaps on Rust.
            let mut ct2 = vec![0u8; MLKEM_CT];
            let mut ss_c2 = vec![0u8; MLKEM_SS];
            assert_eq!(
                c_enc(ct2.as_mut_ptr(), ss_c2.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            let mut ss_r2 = vec![0u8; MLKEM_SS];
            assert_eq!(r_dec(ss_r2.as_mut_ptr(), ct2.as_ptr(), sk.as_ptr()), 0);
            assert_eq!(ss_c2, ss_r2, "C-encaps -> Rust-decaps");
        }
    }
}

// =====================================================================
// Phase B — X-Wing valid path (fully deterministic, byte-for-byte)
// =====================================================================
#[test]
fn xwing_keypair_encaps_decaps_byte_equal() {
    let l = libs();
    unsafe {
        let (c_kp, r_kp) = sympair!(l, b"crypto_kem_xwing_seed_keypair", SeedKeypairFn);
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_xwing_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_xwing_dec", DecFn);

        let mut rng = Rng::new(0x7777_0001);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(XWING_KP_SEED);
            let enc_seed = rng.vec(XWING_ENC_SEED);

            let (mut cpk, mut rpk) = (vec![0u8; XWING_PK], vec![0u8; XWING_PK]);
            let (mut csk, mut rsk) = (vec![0u8; XWING_SK], vec![0u8; XWING_SK]);
            let rc_c = c_kp(cpk.as_mut_ptr(), csk.as_mut_ptr(), kp_seed.as_ptr());
            let rc_r = r_kp(rpk.as_mut_ptr(), rsk.as_mut_ptr(), kp_seed.as_ptr());
            assert_eq!(rc_c, rc_r, "xwing seed_keypair rc");
            assert_eq!(cpk, rpk, "xwing pk");
            assert_eq!(csk, rsk, "xwing sk");

            let (mut cct, mut rct) = (vec![0u8; XWING_CT], vec![0u8; XWING_CT]);
            let (mut css, mut rss) = (vec![0u8; XWING_SS], vec![0u8; XWING_SS]);
            let ec_c = c_enc(cct.as_mut_ptr(), css.as_mut_ptr(), cpk.as_ptr(), enc_seed.as_ptr());
            let ec_r = r_enc(rct.as_mut_ptr(), rss.as_mut_ptr(), cpk.as_ptr(), enc_seed.as_ptr());
            assert_eq!(ec_c, ec_r, "xwing enc rc");
            assert_eq!(ec_c, 0, "xwing enc should succeed");
            assert_eq!(cct, rct, "xwing ct");
            assert_eq!(css, rss, "xwing enc ss");

            let (mut css_d, mut rss_d) = (vec![0u8; XWING_SS], vec![0u8; XWING_SS]);
            let dc_c = c_dec(css_d.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            let dc_r = r_dec(rss_d.as_mut_ptr(), cct.as_ptr(), csk.as_ptr());
            assert_eq!(dc_c, dc_r, "xwing dec rc");
            assert_eq!(css_d, rss_d, "xwing dec ss byte-equal");
            assert_eq!(css_d, css, "xwing dec == enc ss");
        }
    }
}

#[test]
fn xwing_cross_library_roundtrip() {
    let l = libs();
    unsafe {
        let (c_kp, _r_kp) = sympair!(l, b"crypto_kem_xwing_seed_keypair", SeedKeypairFn);
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_xwing_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_xwing_dec", DecFn);

        let mut rng = Rng::new(0x9090_ABCD);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(XWING_KP_SEED);
            let enc_seed = rng.vec(XWING_ENC_SEED);

            let mut pk = vec![0u8; XWING_PK];
            let mut sk = vec![0u8; XWING_SK];
            assert_eq!(c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), kp_seed.as_ptr()), 0);

            // Encaps Rust -> decaps C.
            let mut ct = vec![0u8; XWING_CT];
            let mut ss_r = vec![0u8; XWING_SS];
            assert_eq!(
                r_enc(ct.as_mut_ptr(), ss_r.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            let mut ss_c = vec![0u8; XWING_SS];
            assert_eq!(c_dec(ss_c.as_mut_ptr(), ct.as_ptr(), sk.as_ptr()), 0);
            assert_eq!(ss_r, ss_c, "xwing Rust-encaps -> C-decaps");

            // Encaps C -> decaps Rust.
            let mut ct2 = vec![0u8; XWING_CT];
            let mut ss_c2 = vec![0u8; XWING_SS];
            assert_eq!(
                c_enc(ct2.as_mut_ptr(), ss_c2.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            let mut ss_r2 = vec![0u8; XWING_SS];
            assert_eq!(r_dec(ss_r2.as_mut_ptr(), ct2.as_ptr(), sk.as_ptr()), 0);
            assert_eq!(ss_c2, ss_r2, "xwing C-encaps -> Rust-decaps");
        }
    }
}

// =====================================================================
// Phase B — ipcrypt (deterministic block cipher); fixed keys, many inputs
// =====================================================================
#[test]
fn ipcrypt_deterministic_encrypt_decrypt() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_encrypt", IpFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_decrypt", IpFn);

        let mut rng = Rng::new(0x1D57_0001);
        // Two fixed keys.
        for key_seed in [0xA1A1_u64, 0x00FF_u64] {
            let mut kr = Rng::new(key_seed);
            let key = kr.vec(IP_KEY);
            for _ in 0..ITERS {
                let input = rng.vec(IP_BYTES);
                let (mut co, mut ro) = (vec![0u8; IP_BYTES], vec![0u8; IP_BYTES]);
                c_enc(co.as_mut_ptr(), input.as_ptr(), key.as_ptr());
                r_enc(ro.as_mut_ptr(), input.as_ptr(), key.as_ptr());
                assert_eq!(co, ro, "ipcrypt encrypt byte-equal");

                // roundtrip on each lib
                let (mut cd, mut rd) = (vec![0u8; IP_BYTES], vec![0u8; IP_BYTES]);
                c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
                r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
                assert_eq!(cd, rd, "ipcrypt decrypt byte-equal");
                assert_eq!(cd, input, "ipcrypt roundtrip");
            }
        }
    }
}

#[test]
fn ipcrypt_nd_encrypt_decrypt() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_nd_encrypt", IpTweakFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_nd_decrypt", IpFn);

        let mut kr = Rng::new(0xC0FE_EE);
        let key = kr.vec(IP_ND_KEY);
        let mut rng = Rng::new(0x2222_3333);
        for _ in 0..ITERS {
            let input = rng.vec(IP_ND_IN);
            let tweak = rng.vec(IP_ND_TWEAK);
            let (mut co, mut ro) = (vec![0u8; IP_ND_OUT], vec![0u8; IP_ND_OUT]);
            c_enc(co.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            r_enc(ro.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            assert_eq!(co, ro, "ipcrypt nd encrypt byte-equal");
            // tweak is prepended to output
            assert_eq!(&co[..IP_ND_TWEAK], &tweak[..], "nd output carries tweak");

            let (mut cd, mut rd) = (vec![0u8; IP_ND_IN], vec![0u8; IP_ND_IN]);
            c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
            r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
            assert_eq!(cd, rd, "ipcrypt nd decrypt byte-equal");
            assert_eq!(cd, input, "ipcrypt nd roundtrip");
        }
    }
}

#[test]
fn ipcrypt_ndx_encrypt_decrypt() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_ndx_encrypt", IpTweakFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_ndx_decrypt", IpFn);

        let mut kr = Rng::new(0xBEEF_1234);
        let key = kr.vec(IP_NDX_KEY);
        let mut rng = Rng::new(0x4444_5555);
        for _ in 0..ITERS {
            let input = rng.vec(IP_NDX_IN);
            let tweak = rng.vec(IP_NDX_TWEAK);
            let (mut co, mut ro) = (vec![0u8; IP_NDX_OUT], vec![0u8; IP_NDX_OUT]);
            c_enc(co.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            r_enc(ro.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            assert_eq!(co, ro, "ipcrypt ndx encrypt byte-equal");
            assert_eq!(&co[..IP_NDX_TWEAK], &tweak[..], "ndx output carries tweak");

            let (mut cd, mut rd) = (vec![0u8; IP_NDX_IN], vec![0u8; IP_NDX_IN]);
            c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
            r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
            assert_eq!(cd, rd, "ipcrypt ndx decrypt byte-equal");
            assert_eq!(cd, input, "ipcrypt ndx roundtrip");
        }
    }
}

// ndx has a special branch when the two round keys collide at ROUNDS/2.
// Trigger it by using a key whose two halves are identical.
#[test]
fn ipcrypt_ndx_collision_branch() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_ndx_encrypt", IpTweakFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_ndx_decrypt", IpFn);

        // Identical 16-byte halves => tkeys == rkeys => diff == 0 => fallback.
        let mut kr = Rng::new(0xDEAD_BEEF);
        let half = kr.vec(16);
        let mut key = Vec::with_capacity(IP_NDX_KEY);
        key.extend_from_slice(&half);
        key.extend_from_slice(&half);

        let mut rng = Rng::new(0x6666_7777);
        for _ in 0..ITERS {
            let input = rng.vec(IP_NDX_IN);
            let tweak = rng.vec(IP_NDX_TWEAK);
            let (mut co, mut ro) = (vec![0u8; IP_NDX_OUT], vec![0u8; IP_NDX_OUT]);
            c_enc(co.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            r_enc(ro.as_mut_ptr(), input.as_ptr(), tweak.as_ptr(), key.as_ptr());
            assert_eq!(co, ro, "ndx collision-branch encrypt byte-equal");

            let (mut cd, mut rd) = (vec![0u8; IP_NDX_IN], vec![0u8; IP_NDX_IN]);
            c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
            r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
            assert_eq!(cd, rd, "ndx collision-branch decrypt byte-equal");
            assert_eq!(cd, input, "ndx collision-branch roundtrip");
        }
    }
}

#[test]
fn ipcrypt_pfx_encrypt_decrypt() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_pfx_encrypt", IpFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_pfx_decrypt", IpFn);

        let mut kr = Rng::new(0x1357_9BDF);
        let key = kr.vec(IP_PFX_KEY);
        let mut rng = Rng::new(0x8888_9999);
        for i in 0..ITERS {
            // Alternate between a general 16-byte value and an IPv4-mapped
            // address (prefix 0..0,0xff,0xff) to cover both pfx code paths.
            let input: Vec<u8> = if i % 2 == 0 {
                rng.vec(IP_PFX_BYTES)
            } else {
                let mut v = vec![0u8; IP_PFX_BYTES];
                v[10] = 0xff;
                v[11] = 0xff;
                let last4 = rng.vec(4);
                v[12..16].copy_from_slice(&last4);
                v
            };
            let (mut co, mut ro) = (vec![0u8; IP_PFX_BYTES], vec![0u8; IP_PFX_BYTES]);
            c_enc(co.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            r_enc(ro.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            assert_eq!(co, ro, "ipcrypt pfx encrypt byte-equal");

            let (mut cd, mut rd) = (vec![0u8; IP_PFX_BYTES], vec![0u8; IP_PFX_BYTES]);
            c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
            r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
            assert_eq!(cd, rd, "ipcrypt pfx decrypt byte-equal");
            assert_eq!(cd, input, "ipcrypt pfx roundtrip");
        }
    }
}

// pfx has a collision fallback identical to ndx (k1keys[5] == k2keys[5]).
#[test]
fn ipcrypt_pfx_collision_branch() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_ipcrypt_pfx_encrypt", IpFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_ipcrypt_pfx_decrypt", IpFn);

        let mut kr = Rng::new(0x0F0F_0F0F);
        let half = kr.vec(16);
        let mut key = Vec::with_capacity(IP_PFX_KEY);
        key.extend_from_slice(&half);
        key.extend_from_slice(&half);

        let mut rng = Rng::new(0xAAAA_BBBB);
        for _ in 0..ITERS {
            let input = rng.vec(IP_PFX_BYTES);
            let (mut co, mut ro) = (vec![0u8; IP_PFX_BYTES], vec![0u8; IP_PFX_BYTES]);
            c_enc(co.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            r_enc(ro.as_mut_ptr(), input.as_ptr(), key.as_ptr());
            assert_eq!(co, ro, "pfx collision-branch encrypt byte-equal");

            let (mut cd, mut rd) = (vec![0u8; IP_PFX_BYTES], vec![0u8; IP_PFX_BYTES]);
            c_dec(cd.as_mut_ptr(), co.as_ptr(), key.as_ptr());
            r_dec(rd.as_mut_ptr(), ro.as_ptr(), key.as_ptr());
            assert_eq!(cd, rd, "pfx collision-branch decrypt byte-equal");
            assert_eq!(cd, input, "pfx collision-branch roundtrip");
        }
    }
}

// =====================================================================
// Phase C — error paths (return codes / sentinels the C source enforces)
// =====================================================================

// mlkem768_ref_enc_deterministic returns -1 when the public key polyvec is not
// canonical (any coefficient >= q=3329). An all-0xFF pk yields coeffs=0xFFF.
#[test]
fn mlkem768_enc_rejects_noncanonical_pk() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_mlkem768_enc_deterministic", EncDetFn);
        let bad_pk = vec![0xffu8; MLKEM_PK];
        let seed = vec![7u8; MLKEM_ENC_SEED];
        let (mut cct, mut rct) = (vec![0u8; MLKEM_CT], vec![0u8; MLKEM_CT]);
        let (mut css, mut rss) = (vec![0u8; MLKEM_SS], vec![0u8; MLKEM_SS]);
        let rc_c = c_enc(cct.as_mut_ptr(), css.as_mut_ptr(), bad_pk.as_ptr(), seed.as_ptr());
        let rc_r = r_enc(rct.as_mut_ptr(), rss.as_mut_ptr(), bad_pk.as_ptr(), seed.as_ptr());
        assert_eq!(rc_c, -1, "C mlkem enc must reject non-canonical pk");
        assert_eq!(rc_r, -1, "Rust mlkem enc must reject non-canonical pk");
    }
}

// A fully random pk is (with overwhelming probability) non-canonical too;
// verify both libraries reject the SAME set of random keys identically.
#[test]
fn mlkem768_enc_random_pk_return_code_matches() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_mlkem768_enc_deterministic", EncDetFn);
        let mut rng = Rng::new(0xF00D_F00D);
        for _ in 0..ITERS {
            let pk = rng.vec(MLKEM_PK);
            let seed = rng.vec(MLKEM_ENC_SEED);
            let (mut cct, mut rct) = (vec![0u8; MLKEM_CT], vec![0u8; MLKEM_CT]);
            let (mut css, mut rss) = (vec![0u8; MLKEM_SS], vec![0u8; MLKEM_SS]);
            let rc_c = c_enc(cct.as_mut_ptr(), css.as_mut_ptr(), pk.as_ptr(), seed.as_ptr());
            let rc_r = r_enc(rct.as_mut_ptr(), rss.as_mut_ptr(), pk.as_ptr(), seed.as_ptr());
            assert_eq!(rc_c, rc_r, "mlkem enc rc must match for random pk");
            if rc_c == 0 {
                assert_eq!(cct, rct, "ct must match when both accept");
                assert_eq!(css, rss, "ss must match when both accept");
            }
        }
    }
}

// xwing_enc_deterministic returns -1 when the embedded ML-KEM pk (first 1184
// bytes of the 1216-byte pk) is non-canonical.
#[test]
fn xwing_enc_rejects_noncanonical_mlkem_pk() {
    let l = libs();
    unsafe {
        let (c_enc, r_enc) = sympair!(l, b"crypto_kem_xwing_enc_deterministic", EncDetFn);
        // First 1184 bytes 0xFF => ML-KEM part non-canonical; last 32 arbitrary.
        let mut bad_pk = vec![0xffu8; XWING_PK];
        for b in bad_pk[MLKEM_PK..].iter_mut() {
            *b = 0;
        }
        let seed = vec![3u8; XWING_ENC_SEED];
        let (mut cct, mut rct) = (vec![0u8; XWING_CT], vec![0u8; XWING_CT]);
        let (mut css, mut rss) = (vec![0u8; XWING_SS], vec![0u8; XWING_SS]);
        let rc_c = c_enc(cct.as_mut_ptr(), css.as_mut_ptr(), bad_pk.as_ptr(), seed.as_ptr());
        let rc_r = r_enc(rct.as_mut_ptr(), rss.as_mut_ptr(), bad_pk.as_ptr(), seed.as_ptr());
        assert_eq!(rc_c, -1, "C xwing enc must reject non-canonical ML-KEM pk");
        assert_eq!(rc_r, -1, "Rust xwing enc must reject non-canonical ML-KEM pk");
    }
}

// ML-KEM decapsulation uses implicit rejection (FO transform): a tampered
// ciphertext never returns an error; instead it yields a pseudo-random shared
// secret derived from z. Both libs must agree byte-for-byte (and differ from
// the true shared secret).
#[test]
fn mlkem768_dec_implicit_rejection_matches() {
    let l = libs();
    unsafe {
        let (c_kp, _r_kp) = sympair!(l, b"crypto_kem_mlkem768_seed_keypair", SeedKeypairFn);
        let (c_enc, _r_enc) = sympair!(l, b"crypto_kem_mlkem768_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_mlkem768_dec", DecFn);

        let mut rng = Rng::new(0xDEAD_0001);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(MLKEM_KP_SEED);
            let enc_seed = rng.vec(MLKEM_ENC_SEED);
            let mut pk = vec![0u8; MLKEM_PK];
            let mut sk = vec![0u8; MLKEM_SK];
            assert_eq!(c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), kp_seed.as_ptr()), 0);
            let mut ct = vec![0u8; MLKEM_CT];
            let mut ss_true = vec![0u8; MLKEM_SS];
            assert_eq!(
                c_enc(ct.as_mut_ptr(), ss_true.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            // Tamper one byte.
            let idx = rng.range(MLKEM_CT);
            ct[idx] ^= 0x01;

            let (mut css, mut rss) = (vec![0u8; MLKEM_SS], vec![0u8; MLKEM_SS]);
            let rc_c = c_dec(css.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
            let rc_r = r_dec(rss.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
            assert_eq!(rc_c, 0, "mlkem dec returns 0 (implicit rejection)");
            assert_eq!(rc_r, 0, "mlkem dec returns 0 (implicit rejection)");
            assert_eq!(css, rss, "implicit-rejection ss must match byte-for-byte");
            assert_ne!(css, ss_true, "tampered ct must not yield the true ss");
        }
    }
}

// X-Wing decapsulation on a tampered ciphertext: the ML-KEM half uses implicit
// rejection and the combiner always runs, so dec returns 0 with a shared secret
// that differs from the true one. Both libs must agree byte-for-byte.
#[test]
fn xwing_dec_tampered_ct_matches() {
    let l = libs();
    unsafe {
        let (c_kp, _r_kp) = sympair!(l, b"crypto_kem_xwing_seed_keypair", SeedKeypairFn);
        let (c_enc, _r_enc) = sympair!(l, b"crypto_kem_xwing_enc_deterministic", EncDetFn);
        let (c_dec, r_dec) = sympair!(l, b"crypto_kem_xwing_dec", DecFn);

        let mut rng = Rng::new(0xBEEF_0002);
        for _ in 0..ITERS {
            let kp_seed = rng.vec(XWING_KP_SEED);
            let enc_seed = rng.vec(XWING_ENC_SEED);
            let mut pk = vec![0u8; XWING_PK];
            let mut sk = vec![0u8; XWING_SK];
            assert_eq!(c_kp(pk.as_mut_ptr(), sk.as_mut_ptr(), kp_seed.as_ptr()), 0);
            let mut ct = vec![0u8; XWING_CT];
            let mut ss_true = vec![0u8; XWING_SS];
            assert_eq!(
                c_enc(ct.as_mut_ptr(), ss_true.as_mut_ptr(), pk.as_ptr(), enc_seed.as_ptr()),
                0
            );
            // Tamper a byte in the ML-KEM portion of the ct.
            let idx = rng.range(MLKEM_CT);
            ct[idx] ^= 0x01;

            let (mut css, mut rss) = (vec![0u8; XWING_SS], vec![0u8; XWING_SS]);
            let rc_c = c_dec(css.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
            let rc_r = r_dec(rss.as_mut_ptr(), ct.as_ptr(), sk.as_ptr());
            assert_eq!(rc_c, rc_r, "xwing dec rc must match");
            assert_eq!(css, rss, "xwing tampered dec ss must match byte-for-byte");
        }
    }
}
